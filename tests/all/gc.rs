use std::cell::Cell;
use std::rc::Rc;
use wasmtime::*;

fn ref_types_module(source: &str) -> anyhow::Result<(Store, Module)> {
    let _ = env_logger::try_init();

    let mut config = Config::new();
    config.wasm_reference_types(true);

    let engine = Engine::new(&config);
    let store = Store::new(&engine);

    let module = Module::new(&engine, source)?;

    Ok((store, module))
}

#[test]
fn smoke_test_gc() -> anyhow::Result<()> {
    let (store, module) = ref_types_module(
        r#"
            (module
                (import "" "" (func $do_gc))
                (func $recursive (export "func") (param i32 externref) (result externref)
                    local.get 0
                    i32.eqz
                    if (result externref)
                        call $do_gc
                        local.get 1
                    else
                        local.get 0
                        i32.const 1
                        i32.sub
                        local.get 1
                        call $recursive
                    end
                )
            )
        "#,
    )?;

    let do_gc = Func::wrap(&store, {
        let store = store.clone();
        move || {
            // Do a GC with `externref`s on the stack in Wasm frames.
            store.gc();
        }
    });
    let instance = Instance::new(&store, &module, &[do_gc.into()])?;
    let func = instance.get_func("func").unwrap();

    let inner_dropped = Rc::new(Cell::new(false));
    let r = ExternRef::new(&store, SetFlagOnDrop(inner_dropped.clone()));
    {
        let args = [Val::I32(5), Val::ExternRef(Some(r.clone()))];
        func.call(&args)?;
    }

    // Still held alive by the `VMExternRefActivationsTable` (potentially in
    // multiple slots within the table) and by this `r` local.
    assert!(r.strong_count() >= 2);

    // Doing a GC should see that there aren't any `externref`s on the stack in
    // Wasm frames anymore.
    store.gc();
    assert_eq!(r.strong_count(), 1);

    // Dropping `r` should drop the inner `SetFlagOnDrop` value.
    drop(r);
    assert!(inner_dropped.get());

    return Ok(());

    struct SetFlagOnDrop(Rc<Cell<bool>>);

    impl Drop for SetFlagOnDrop {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }
}

#[test]
fn wasm_dropping_refs() -> anyhow::Result<()> {
    let (store, module) = ref_types_module(
        r#"
            (module
                (func (export "drop_ref") (param externref)
                    nop
                )
            )
        "#,
    )?;

    let instance = Instance::new(&store, &module, &[])?;
    let drop_ref = instance.get_func("drop_ref").unwrap();

    let num_refs_dropped = Rc::new(Cell::new(0));

    // NB: 4096 is greater than the initial `VMExternRefActivationsTable`
    // capacity, so this will trigger at least one GC.
    for _ in 0..4096 {
        let r = ExternRef::new(&store, CountDrops(num_refs_dropped.clone()));
        let args = [Val::ExternRef(Some(r))];
        drop_ref.call(&args)?;
    }

    assert!(num_refs_dropped.get() > 0);

    // And after doing a final GC, all the refs should have been dropped.
    store.gc();
    assert_eq!(num_refs_dropped.get(), 4096);

    return Ok(());

    struct CountDrops(Rc<Cell<usize>>);

    impl Drop for CountDrops {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
}

#[test]
fn many_live_refs() -> anyhow::Result<()> {
    let mut wat = r#"
        (module
            ;; Make new `externref`s.
            (import "" "make_ref" (func $make_ref (result externref)))

            ;; Observe an `externref` so it is kept live.
            (import "" "observe_ref" (func $observe_ref (param externref)))

            (func (export "many_live_refs")
    "#
    .to_string();

    // This is more than the initial `VMExternRefActivationsTable` capacity, so
    // it will need to allocate additional bump chunks.
    const NUM_LIVE_REFS: usize = 1024;

    // Push `externref`s onto the stack.
    for _ in 0..NUM_LIVE_REFS {
        wat.push_str("(call $make_ref)\n");
    }

    // Pop `externref`s from the stack. Because we pass each of them to a
    // function call here, they are all live references for the duration of
    // their lifetimes.
    for _ in 0..NUM_LIVE_REFS {
        wat.push_str("(call $observe_ref)\n");
    }

    wat.push_str(
        "
            ) ;; func
        ) ;; module
        ",
    );

    let (store, module) = ref_types_module(&wat)?;

    let live_refs = Rc::new(Cell::new(0));

    let make_ref = Func::new(
        &store,
        FuncType::new(
            vec![].into_boxed_slice(),
            vec![ValType::ExternRef].into_boxed_slice(),
        ),
        {
            let store = store.clone();
            let live_refs = live_refs.clone();
            move |_caller, _params, results| {
                results[0] = Val::ExternRef(Some(ExternRef::new(
                    &store,
                    CountLiveRefs::new(live_refs.clone()),
                )));
                Ok(())
            }
        },
    );

    let observe_ref = Func::new(
        &store,
        FuncType::new(
            vec![ValType::ExternRef].into_boxed_slice(),
            vec![].into_boxed_slice(),
        ),
        |_caller, params, _results| {
            let r = params[0].externref().unwrap().unwrap();
            let r = r.data().downcast_ref::<CountLiveRefs>().unwrap();
            assert!(r.live_refs.get() > 0);
            Ok(())
        },
    );

    let instance = Instance::new(&store, &module, &[make_ref.into(), observe_ref.into()])?;
    let many_live_refs = instance.get_func("many_live_refs").unwrap();

    many_live_refs.call(&[])?;

    store.gc();
    assert_eq!(live_refs.get(), 0);

    return Ok(());

    struct CountLiveRefs {
        live_refs: Rc<Cell<usize>>,
    }

    impl CountLiveRefs {
        fn new(live_refs: Rc<Cell<usize>>) -> Self {
            let live = live_refs.get();
            live_refs.set(live + 1);
            Self { live_refs }
        }
    }

    impl Drop for CountLiveRefs {
        fn drop(&mut self) {
            let live = self.live_refs.get();
            self.live_refs.set(live - 1);
        }
    }
}

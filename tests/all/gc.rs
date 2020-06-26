use super::ref_types_module;
use std::cell::Cell;
use std::rc::Rc;
use wasmtime::*;

struct SetFlagOnDrop(Rc<Cell<bool>>);

impl Drop for SetFlagOnDrop {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

struct GcOnDrop {
    store: Store,
    gc_count: Rc<Cell<usize>>,
}

impl Drop for GcOnDrop {
    fn drop(&mut self) {
        self.store.gc();
        self.gc_count.set(self.gc_count.get() + 1);
    }
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

    let do_gc = Func::wrap(&store, |caller: Caller| {
        // Do a GC with `externref`s on the stack in Wasm frames.
        caller.store().gc();
    });
    let instance = Instance::new(&store, &module, &[do_gc.into()])?;
    let func = instance.get_func("func").unwrap();

    let inner_dropped = Rc::new(Cell::new(false));
    let r = ExternRef::new(SetFlagOnDrop(inner_dropped.clone()));
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

    Ok(())
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
        let r = ExternRef::new(CountDrops(num_refs_dropped.clone()));
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
            let live_refs = live_refs.clone();
            move |_caller, _params, results| {
                results[0] =
                    Val::ExternRef(Some(ExternRef::new(CountLiveRefs::new(live_refs.clone()))));
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

#[test]
fn drop_externref_via_table_set() -> anyhow::Result<()> {
    let (store, module) = ref_types_module(
        r#"
            (module
                (table $t 1 externref)

                (func (export "table-set") (param externref)
                  (table.set $t (i32.const 0) (local.get 0))
                )
            )
        "#,
    )?;

    let instance = Instance::new(&store, &module, &[])?;
    let table_set = instance.get_func("table-set").unwrap();

    let foo_is_dropped = Rc::new(Cell::new(false));
    let bar_is_dropped = Rc::new(Cell::new(false));

    let foo = ExternRef::new(SetFlagOnDrop(foo_is_dropped.clone()));
    let bar = ExternRef::new(SetFlagOnDrop(bar_is_dropped.clone()));

    {
        let args = vec![Val::ExternRef(Some(foo))];
        table_set.call(&args)?;
    }
    store.gc();
    assert!(!foo_is_dropped.get());
    assert!(!bar_is_dropped.get());

    {
        let args = vec![Val::ExternRef(Some(bar))];
        table_set.call(&args)?;
    }
    store.gc();
    assert!(foo_is_dropped.get());
    assert!(!bar_is_dropped.get());

    table_set.call(&[Val::ExternRef(None)])?;
    assert!(foo_is_dropped.get());
    assert!(bar_is_dropped.get());

    Ok(())
}

#[test]
fn gc_in_externref_dtor() -> anyhow::Result<()> {
    let (store, module) = ref_types_module(
        r#"
            (module
                (table $t 1 externref)

                (func (export "table-set") (param externref)
                  (table.set $t (i32.const 0) (local.get 0))
                )
            )
        "#,
    )?;

    let instance = Instance::new(&store, &module, &[])?;
    let table_set = instance.get_func("table-set").unwrap();

    let gc_count = Rc::new(Cell::new(0));

    // Put a `GcOnDrop` into the table.
    {
        let args = vec![Val::ExternRef(Some(ExternRef::new(GcOnDrop {
            store: store.clone(),
            gc_count: gc_count.clone(),
        })))];
        table_set.call(&args)?;
    }

    // Remove the `GcOnDrop` from the `VMExternRefActivationsTable`.
    store.gc();

    // Overwrite the `GcOnDrop` table element, causing it to be dropped, and
    // triggering a GC.
    table_set.call(&[Val::ExternRef(None)])?;
    assert_eq!(gc_count.get(), 1);

    Ok(())
}

#[test]
fn touch_own_table_element_in_externref_dtor() -> anyhow::Result<()> {
    let (store, module) = ref_types_module(
        r#"
            (module
                (table $t (export "table") 1 externref)

                (func (export "table-set") (param externref)
                  (table.set $t (i32.const 0) (local.get 0))
                )
            )
        "#,
    )?;

    let instance = Instance::new(&store, &module, &[])?;
    let table = instance.get_table("table").unwrap();
    let table_set = instance.get_func("table-set").unwrap();

    let touched = Rc::new(Cell::new(false));

    {
        let args = vec![Val::ExternRef(Some(ExternRef::new(TouchTableOnDrop {
            table,
            touched: touched.clone(),
        })))];
        table_set.call(&args)?;
    }

    // Remove the `TouchTableOnDrop` from the `VMExternRefActivationsTable`.
    store.gc();

    table_set.call(&[Val::ExternRef(Some(ExternRef::new("hello".to_string())))])?;
    assert!(touched.get());

    return Ok(());

    struct TouchTableOnDrop {
        table: Table,
        touched: Rc<Cell<bool>>,
    }

    impl Drop for TouchTableOnDrop {
        fn drop(&mut self) {
            // From the `Drop` implementation, we see the new table element, not
            // `self`.
            let elem = self.table.get(0).unwrap().unwrap_externref().unwrap();
            assert!(elem.data().is::<String>());
            assert_eq!(elem.data().downcast_ref::<String>().unwrap(), "hello");
            self.touched.set(true);
        }
    }
}

#[test]
fn gc_during_gc_when_passing_refs_into_wasm() -> anyhow::Result<()> {
    let (store, module) = ref_types_module(
        r#"
            (module
                (table $t 1 externref)
                (func (export "f") (param externref)
                  (table.set $t (i32.const 0) (local.get 0))
                )
            )
        "#,
    )?;

    let instance = Instance::new(&store, &module, &[])?;
    let f = instance.get_func("f").unwrap();

    let gc_count = Rc::new(Cell::new(0));

    for _ in 0..1024 {
        let args = vec![Val::ExternRef(Some(ExternRef::new(GcOnDrop {
            store: store.clone(),
            gc_count: gc_count.clone(),
        })))];
        f.call(&args)?;
    }

    f.call(&[Val::ExternRef(None)])?;
    store.gc();
    assert_eq!(gc_count.get(), 1024);

    Ok(())
}

#[test]
fn gc_during_gc_from_many_table_gets() -> anyhow::Result<()> {
    let (store, module) = ref_types_module(
        r#"
            (module
                (import "" "" (func $observe_ref (param externref)))
                (table $t 1 externref)
                (func (export "init") (param externref)
                    (table.set $t (i32.const 0) (local.get 0))
                )
                (func (export "run") (param i32)
                    (loop $continue
                        (if (i32.eqz (local.get 0)) (return))
                        (call $observe_ref (table.get $t (i32.const 0)))
                        (local.set 0 (i32.sub (local.get 0) (i32.const 1)))
                        (br $continue)
                    )
                )
            )
        "#,
    )?;

    let observe_ref = Func::new(
        &store,
        FuncType::new(
            vec![ValType::ExternRef].into_boxed_slice(),
            vec![].into_boxed_slice(),
        ),
        |_caller, _params, _results| Ok(()),
    );

    let instance = Instance::new(&store, &module, &[observe_ref.into()])?;
    let init = instance.get_func("init").unwrap();
    let run = instance.get_func("run").unwrap();

    let gc_count = Rc::new(Cell::new(0));

    // Initialize the table element with a `GcOnDrop`. This also puts it in the
    // `VMExternRefActivationsTable`.
    {
        let args = vec![Val::ExternRef(Some(ExternRef::new(GcOnDrop {
            store: store.clone(),
            gc_count: gc_count.clone(),
        })))];
        init.call(&args)?;
    }

    // Overwrite the `GcOnDrop` with another reference. The `GcOnDrop` is still
    // in the `VMExternRefActivationsTable`.
    {
        let args = vec![Val::ExternRef(Some(ExternRef::new(String::from("hello"))))];
        init.call(&args)?;
    }

    // Now call `run`, which does a bunch of `table.get`s, filling up the
    // `VMExternRefActivationsTable`'s bump region, and eventually triggering a
    // GC that will deallocate our `GcOnDrop` which will also trigger a nested
    // GC.
    run.call(&[Val::I32(1024)])?;

    // We should have done our nested GC.
    assert_eq!(gc_count.get(), 1);

    Ok(())
}

#[test]
fn pass_externref_into_wasm_during_destructor_in_gc() -> anyhow::Result<()> {
    let (store, module) = ref_types_module(
        r#"
            (module
                (table $t 1 externref)

                (func (export "f") (param externref)
                  nop
                )
            )
        "#,
    )?;

    let instance = Instance::new(&store, &module, &[])?;
    let f = instance.get_func("f").unwrap();
    let r = ExternRef::new("hello");
    let did_call = Rc::new(Cell::new(false));

    // Put a `CallOnDrop` into the `VMExternRefActivationsTable`.
    {
        let args = vec![Val::ExternRef(Some(ExternRef::new(CallOnDrop(
            f.clone(),
            r.clone(),
            did_call.clone(),
        ))))];
        f.call(&args)?;
    }

    // One ref count for `r`, one for the `CallOnDrop`.
    assert_eq!(r.strong_count(), 2);

    // Do a GC, which will see that the only reference holding the `CallOnDrop`
    // is the `VMExternRefActivationsTable`, and will drop it. Dropping it will
    // cause it to call into `f` again.
    store.gc();
    assert!(did_call.get());

    // The `CallOnDrop` is no longer holding onto `r`, but the
    // `VMExternRefActivationsTable` is.
    assert_eq!(r.strong_count(), 2);

    // GC again to empty the `VMExternRefActivationsTable`. Now `r` is the only
    // thing holding its `externref` alive.
    store.gc();
    assert_eq!(r.strong_count(), 1);

    return Ok(());

    struct CallOnDrop(Func, ExternRef, Rc<Cell<bool>>);

    impl Drop for CallOnDrop {
        fn drop(&mut self) {
            self.0
                .call(&[Val::ExternRef(Some(self.1.clone()))])
                .unwrap();
            self.2.set(true);
        }
    }
}

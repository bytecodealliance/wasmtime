use super::ref_types_module;
use super::skip_pooling_allocator_tests;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;
use wasmtime::*;

struct SetFlagOnDrop(Arc<AtomicBool>);

impl Drop for SetFlagOnDrop {
    fn drop(&mut self) {
        self.0.store(true, SeqCst);
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn smoke_test_gc_no_epochs() -> Result<()> {
    smoke_test_gc_impl(false)
}

#[test]
#[cfg_attr(miri, ignore)]
fn smoke_test_gc_yes_epochs() -> Result<()> {
    smoke_test_gc_impl(true)
}

fn smoke_test_gc_impl(use_epochs: bool) -> Result<()> {
    let (mut store, module) = ref_types_module(
        use_epochs,
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

    let do_gc = Func::wrap(&mut store, |mut caller: Caller<'_, _>| {
        // Do a GC with `externref`s on the stack in Wasm frames.
        caller.gc();
    });
    let instance = Instance::new(&mut store, &module, &[do_gc.into()])?;
    let func = instance.get_func(&mut store, "func").unwrap();

    let inner_dropped = Arc::new(AtomicBool::new(false));

    {
        let mut scope = RootScope::new(&mut store);

        let r = ExternRef::new(&mut scope, SetFlagOnDrop(inner_dropped.clone()))?;
        {
            let args = [Val::I32(5), Val::ExternRef(Some(r))];
            func.call(&mut scope, &args, &mut [Val::I32(0)])?;
        }

        // Doing a GC should see that there aren't any `externref`s on the stack in
        // Wasm frames anymore.
        scope.as_context_mut().gc();

        // But the scope should still be rooting `r`.
        assert!(!inner_dropped.load(SeqCst));
    }

    // Exiting the scope and unrooting `r` should have dropped the inner
    // `SetFlagOnDrop` value.
    assert!(inner_dropped.load(SeqCst));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn wasm_dropping_refs() -> Result<()> {
    let (mut store, module) = ref_types_module(
        false,
        r#"
            (module
                (func (export "drop_ref") (param externref)
                    nop
                )
            )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let drop_ref = instance.get_func(&mut store, "drop_ref").unwrap();

    let num_refs_dropped = Arc::new(AtomicUsize::new(0));

    // NB: 4096 is greater than the initial `VMExternRefActivationsTable`
    // capacity, so this will trigger at least one GC.
    for _ in 0..4096 {
        let mut scope = RootScope::new(&mut store);
        let r = ExternRef::new(&mut scope, CountDrops(num_refs_dropped.clone()))?;
        let args = [Val::ExternRef(Some(r))];
        drop_ref.call(&mut scope, &args, &mut [])?;
    }

    assert!(num_refs_dropped.load(SeqCst) > 0);

    // And after doing a final GC, all the refs should have been dropped.
    store.gc();
    assert_eq!(num_refs_dropped.load(SeqCst), 4096);

    return Ok(());

    struct CountDrops(Arc<AtomicUsize>);

    impl Drop for CountDrops {
        fn drop(&mut self) {
            self.0.fetch_add(1, SeqCst);
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn many_live_refs() -> Result<()> {
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

    let (mut store, module) = ref_types_module(false, &wat)?;

    let live_refs = Arc::new(AtomicUsize::new(0));

    let make_ref = Func::wrap(&mut store, {
        let live_refs = live_refs.clone();
        move |mut caller: Caller<'_, _>| {
            Ok(Some(ExternRef::new(
                &mut caller,
                CountLiveRefs::new(live_refs.clone()),
            )?))
        }
    });

    let observe_ref = Func::wrap(
        &mut store,
        |caller: Caller<'_, _>, r: Option<Rooted<ExternRef>>| {
            let r = r
                .unwrap()
                .data(&caller)
                .unwrap()
                .unwrap()
                .downcast_ref::<CountLiveRefs>()
                .unwrap();
            assert!(r.live_refs.load(SeqCst) > 0);
        },
    );

    let instance = Instance::new(&mut store, &module, &[make_ref.into(), observe_ref.into()])?;
    let many_live_refs = instance.get_func(&mut store, "many_live_refs").unwrap();

    many_live_refs.call(&mut store, &[], &mut [])?;

    store.as_context_mut().gc();
    assert_eq!(live_refs.load(SeqCst), 0);

    return Ok(());

    struct CountLiveRefs {
        live_refs: Arc<AtomicUsize>,
    }

    impl CountLiveRefs {
        fn new(live_refs: Arc<AtomicUsize>) -> Self {
            live_refs.fetch_add(1, SeqCst);
            Self { live_refs }
        }
    }

    impl Drop for CountLiveRefs {
        fn drop(&mut self) {
            self.live_refs.fetch_sub(1, SeqCst);
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn drop_externref_via_table_set() -> Result<()> {
    let (mut store, module) = ref_types_module(
        false,
        r#"
            (module
                (table $t 1 externref)

                (func (export "table-set") (param externref)
                  (table.set $t (i32.const 0) (local.get 0))
                )
            )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let table_set = instance.get_func(&mut store, "table-set").unwrap();

    let foo_is_dropped = Arc::new(AtomicBool::new(false));
    let bar_is_dropped = Arc::new(AtomicBool::new(false));

    {
        let mut scope = RootScope::new(&mut store);

        let foo = ExternRef::new(&mut scope, SetFlagOnDrop(foo_is_dropped.clone()))?;
        let bar = ExternRef::new(&mut scope, SetFlagOnDrop(bar_is_dropped.clone()))?;

        {
            let args = vec![Val::ExternRef(Some(foo))];
            table_set.call(&mut scope, &args, &mut [])?;
        }

        scope.as_context_mut().gc();
        assert!(!foo_is_dropped.load(SeqCst));
        assert!(!bar_is_dropped.load(SeqCst));

        {
            let args = vec![Val::ExternRef(Some(bar))];
            table_set.call(&mut scope, &args, &mut [])?;
        }
    }

    store.gc();
    assert!(foo_is_dropped.load(SeqCst));
    assert!(!bar_is_dropped.load(SeqCst));

    table_set.call(&mut store, &[Val::ExternRef(None)], &mut [])?;
    assert!(foo_is_dropped.load(SeqCst));
    assert!(bar_is_dropped.load(SeqCst));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn global_drops_externref() -> Result<()> {
    let _ = env_logger::try_init();
    test_engine(&Engine::default())?;

    if !skip_pooling_allocator_tests() {
        test_engine(&Engine::new(
            Config::new().allocation_strategy(InstanceAllocationStrategy::pooling()),
        )?)?;
    }

    return Ok(());

    fn test_engine(engine: &Engine) -> Result<()> {
        let mut store = Store::new(&engine, ());
        let flag = Arc::new(AtomicBool::new(false));
        let externref = ExternRef::new(&mut store, SetFlagOnDrop(flag.clone()))?;
        Global::new(
            &mut store,
            GlobalType::new(ValType::EXTERNREF, Mutability::Const),
            externref.into(),
        )?;
        drop(store);
        assert!(flag.load(SeqCst));

        let mut store = Store::new(&engine, ());
        let module = Module::new(
            &engine,
            r#"
                (module
                    (global (mut externref) (ref.null extern))

                    (func (export "run") (param externref)
                        local.get 0
                        global.set 0
                    )
                )
            "#,
        )?;
        let instance = Instance::new(&mut store, &module, &[])?;
        let run = instance.get_typed_func::<Option<Rooted<ExternRef>>, ()>(&mut store, "run")?;
        let flag = Arc::new(AtomicBool::new(false));
        let externref = ExternRef::new(&mut store, SetFlagOnDrop(flag.clone()))?;
        run.call(&mut store, Some(externref))?;
        drop(store);
        assert!(flag.load(SeqCst));
        Ok(())
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn table_drops_externref() -> Result<()> {
    let _ = env_logger::try_init();
    test_engine(&Engine::default())?;

    if !skip_pooling_allocator_tests() {
        test_engine(&Engine::new(
            Config::new().allocation_strategy(InstanceAllocationStrategy::pooling()),
        )?)?;
    }

    return Ok(());

    fn test_engine(engine: &Engine) -> Result<()> {
        let mut store = Store::new(&engine, ());
        let flag = Arc::new(AtomicBool::new(false));
        let externref = ExternRef::new(&mut store, SetFlagOnDrop(flag.clone()))?;
        Table::new(
            &mut store,
            TableType::new(RefType::EXTERNREF, 1, None),
            externref.into(),
        )?;
        drop(store);
        assert!(flag.load(SeqCst));

        let mut store = Store::new(&engine, ());
        let module = Module::new(
            &engine,
            r#"
            (module
                (table 1 externref)

                (func (export "run") (param externref)
                    i32.const 0
                    local.get 0
                    table.set 0
                )
            )
        "#,
        )?;
        let instance = Instance::new(&mut store, &module, &[])?;
        let run = instance.get_typed_func::<Option<Rooted<ExternRef>>, ()>(&mut store, "run")?;
        let flag = Arc::new(AtomicBool::new(false));
        let externref = ExternRef::new(&mut store, SetFlagOnDrop(flag.clone()))?;
        run.call(&mut store, Some(externref))?;
        drop(store);
        assert!(flag.load(SeqCst));
        Ok(())
    }
}

#[test]
fn global_init_no_leak() -> Result<()> {
    let (mut store, module) = ref_types_module(
        false,
        r#"
            (module
                (import "" "" (global externref))
                (global externref (global.get 0))
            )
        "#,
    )?;

    let flag = Arc::new(AtomicBool::new(false));
    let externref = ExternRef::new(&mut store, SetFlagOnDrop(flag.clone()))?;
    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::EXTERNREF, Mutability::Const),
        externref.into(),
    )?;
    Instance::new(&mut store, &module, &[global.into()])?;
    drop(store);
    assert!(flag.load(SeqCst));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn no_gc_middle_of_args() -> Result<()> {
    let (mut store, module) = ref_types_module(
        false,
        r#"
            (module
                (import "" "return_some" (func $return (result externref externref externref)))
                (import "" "take_some" (func $take (param externref externref externref)))
                (func (export "run")
                    (local i32)
                    i32.const 1000
                    local.set 0
                    loop
                        call $return
                        call $take
                        local.get 0
                        i32.const -1
                        i32.add
                        local.tee 0
                        br_if 0
                    end
                )
            )
        "#,
    )?;

    let mut linker = Linker::new(store.engine());
    linker.func_wrap("", "return_some", |mut caller: Caller<'_, _>| {
        let a = Some(ExternRef::new(&mut caller, String::from("a"))?);
        let b = Some(ExternRef::new(&mut caller, String::from("b"))?);
        let c = Some(ExternRef::new(&mut caller, String::from("c"))?);
        Ok((a, b, c))
    })?;
    linker.func_wrap(
        "",
        "take_some",
        |caller: Caller<'_, _>,
         a: Option<Rooted<ExternRef>>,
         b: Option<Rooted<ExternRef>>,
         c: Option<Rooted<ExternRef>>| {
            let a = a.unwrap();
            let b = b.unwrap();
            let c = c.unwrap();
            assert_eq!(
                a.data(&caller)
                    .expect("rooted")
                    .expect("host data")
                    .downcast_ref::<String>()
                    .expect("is string"),
                "a"
            );
            assert_eq!(
                b.data(&caller)
                    .expect("rooted")
                    .expect("host data")
                    .downcast_ref::<String>()
                    .expect("is string"),
                "b"
            );
            assert_eq!(
                c.data(&caller)
                    .expect("rooted")
                    .expect("host data")
                    .downcast_ref::<String>()
                    .expect("is string"),
                "c"
            );
        },
    )?;

    let instance = linker.instantiate(&mut store, &module)?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    func.call(&mut store, ())?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn gc_and_tail_calls_and_stack_arguments() -> Result<()> {
    // Test that GC refs in tail-calls' stack arguments get properly accounted
    // for in stack maps.
    //
    // What we do _not_ want to happen is for tail callers to be responsible for
    // including stack arguments in their stack maps (and therefore whether or
    // not they get marked at runtime). If that was the case, then we could have
    // the following scenario:
    //
    // * `f` calls `g` without any stack arguments,
    // * `g` tail calls `h` with GC ref stack arguments,
    // * and then `h` triggers a GC.
    //
    // Because `g`, who is responsible for including the GC refs in its stack
    // map in this hypothetical scenario, is no longer on the stack, we never
    // see its stack map, and therefore never mark the GC refs, and then we
    // collect them too early, and then we can get user-after-free bugs. Not
    // good! Note also that `f`, which is the frame that `h` will return to,
    // _cannot_ be responsible for including these stack arguments in its stack
    // map, because it has no idea what frame will be returning to it, and it
    // could be any number of different functions using that frame for long (and
    // indirect!) tail-call chains.
    //
    // In Cranelift we avoid this scenario because stack arguments are eagerly
    // loaded into virtual registers, and then when we insert a GC safe point,
    // we spill these virtual registers to the callee stack frame, and the stack
    // map includes entries for these stack slots.
    //
    // Nonetheless, this test exercises the above scenario just in case we do
    // something in the future like lazily load stack arguments into virtual
    // registers, to make sure that everything shows up in stack maps like they
    // are supposed to.

    let (mut store, module) = ref_types_module(
        false,
        r#"
            (module
                (import "" "make_some" (func $make (result externref externref externref)))
                (import "" "take_some" (func $take (param externref externref externref)))
                (import "" "gc" (func $gc))

                (func $stack_args (param externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref externref)
                  call $gc
                  ;; Make sure all these GC refs are live, so that they need to
                  ;; be put into the stack map.
                  local.get 0
                  local.get 1
                  local.get 2
                  call $take
                  local.get 3
                  local.get 4
                  local.get 5
                  call $take
                  local.get 6
                  local.get 7
                  local.get 8
                  call $take
                  local.get 9
                  local.get 10
                  local.get 11
                  call $take
                  local.get 12
                  local.get 13
                  local.get 14
                  call $take
                  local.get 15
                  local.get 16
                  local.get 17
                  call $take
                  local.get 18
                  local.get 19
                  local.get 20
                  call $take
                  local.get 21
                  local.get 22
                  local.get 23
                  call $take
                  local.get 24
                  local.get 25
                  local.get 26
                  call $take
                  local.get 27
                  local.get 28
                  local.get 29
                  call $take
                )

                (func $no_stack_args
                  call $make
                  call $make
                  call $make
                  call $make
                  call $make
                  call $make
                  call $make
                  call $make
                  call $make
                  call $make
                  return_call $stack_args
                )

                (func (export "run")
                    (local i32)
                    i32.const 1000
                    local.set 0
                    loop
                        call $no_stack_args
                        local.get 0
                        i32.const -1
                        i32.add
                        local.tee 0
                        br_if 0
                    end
                )
            )
        "#,
    )?;

    let mut linker = Linker::new(store.engine());
    linker.func_wrap("", "make_some", |mut caller: Caller<'_, _>| {
        Ok((
            Some(ExternRef::new(&mut caller, "a".to_string())?),
            Some(ExternRef::new(&mut caller, "b".to_string())?),
            Some(ExternRef::new(&mut caller, "c".to_string())?),
        ))
    })?;
    linker.func_wrap(
        "",
        "take_some",
        |caller: Caller<'_, _>,
         a: Option<Rooted<ExternRef>>,
         b: Option<Rooted<ExternRef>>,
         c: Option<Rooted<ExternRef>>| {
            let a = a.unwrap();
            let b = b.unwrap();
            let c = c.unwrap();
            assert_eq!(
                a.data(&caller)
                    .unwrap()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap(),
                "a"
            );
            assert_eq!(
                b.data(&caller)
                    .unwrap()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap(),
                "b"
            );
            assert_eq!(
                c.data(&caller)
                    .unwrap()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap(),
                "c"
            );
        },
    )?;
    linker.func_wrap("", "gc", |mut caller: Caller<()>| {
        caller.gc();
    })?;

    let instance = linker.instantiate(&mut store, &module)?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    func.call(&mut store, ())?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn no_leak_with_global_get_elem_segment() -> anyhow::Result<()> {
    let dropped = Arc::new(AtomicBool::new(false));

    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "" (global $init externref))
                (start $f)
                (table $t 1 externref)
                (elem $e externref (global.get $init))

                (func $f
                    i32.const 0
                    i32.const 0
                    i32.const 1
                    table.init $t $e

                    i32.const 0
                    i32.const 0
                    i32.const 1
                    table.init $t $e
                )
            )
        "#,
    )?;

    let externref = ExternRef::new(&mut store, SetFlagOnDrop(dropped.clone()))?;
    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::EXTERNREF, Mutability::Const),
        externref.into(),
    )?;

    Instance::new(&mut store, &module, &[global.into()])?;

    drop(store);

    assert!(dropped.load(SeqCst));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn table_init_with_externref_global_get() -> anyhow::Result<()> {
    let dropped = Arc::new(AtomicBool::new(false));

    let mut config = Config::new();
    config.wasm_function_references(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "" (global $init externref))
                (table $t 1 externref (global.get $init))
            )
        "#,
    )?;

    let externref = ExternRef::new(&mut store, SetFlagOnDrop(dropped.clone()))?;
    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::EXTERNREF, Mutability::Const),
        externref.into(),
    )?;

    Instance::new(&mut store, &module, &[global.into()])?;

    drop(store);

    assert!(dropped.load(SeqCst));
    Ok(())
}

#[test]
fn rooted_gets_collected_after_scope_exit() -> Result<()> {
    let mut store = Store::<()>::default();
    let flag = Arc::new(AtomicBool::new(false));

    {
        let mut scope = RootScope::new(&mut store);
        let _externref = ExternRef::new(&mut scope, SetFlagOnDrop(flag.clone()))?;

        scope.as_context_mut().gc();
        assert!(!flag.load(SeqCst), "not dropped when still rooted");
    }

    store.as_context_mut().gc();
    assert!(flag.load(SeqCst), "dropped after being unrooted");

    Ok(())
}

#[test]
fn manually_rooted_gets_collected_after_unrooting() -> Result<()> {
    let mut store = Store::<()>::default();
    let flag = Arc::new(AtomicBool::new(false));

    let externref = ExternRef::new_manually_rooted(&mut store, SetFlagOnDrop(flag.clone()))?;

    store.gc();
    assert!(!flag.load(SeqCst), "not dropped when still rooted");

    externref.unroot(&mut store);
    store.gc();
    assert!(flag.load(SeqCst), "dropped after being unrooted");

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn round_trip_gc_ref_through_typed_wasm_func() -> Result<()> {
    let mut store = Store::<()>::default();
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (import "" "" (func $gc))
                (func (export "f") (param externref) (result externref)
                    call $gc
                    local.get 0
                )
            )
        "#,
    )?;
    let gc = Func::wrap(&mut store, |mut caller: Caller<'_, _>| caller.gc());
    let instance = Instance::new(&mut store, &module, &[gc.into()])?;
    let f = instance
        .get_typed_func::<Option<Rooted<ExternRef>>, Option<Rooted<ExternRef>>>(&mut store, "f")?;
    let x1 = ExternRef::new(&mut store, 1234)?;
    let x2 = f.call(&mut store, Some(x1))?.unwrap();
    assert!(Rooted::ref_eq(&store, &x1, &x2)?);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn round_trip_gc_ref_through_func_wrap() -> Result<()> {
    let mut store = Store::<()>::default();
    let f = Func::wrap(
        &mut store,
        |mut caller: Caller<'_, _>, x: Rooted<ExternRef>| {
            caller.gc();
            x
        },
    );
    let f = f.typed::<Rooted<ExternRef>, Rooted<ExternRef>>(&store)?;
    let x1 = ExternRef::new(&mut store, 1234)?;
    let x2 = f.call(&mut store, x1)?;
    assert!(Rooted::ref_eq(&store, &x1, &x2)?);
    Ok(())
}

#[test]
fn to_raw_from_raw_doesnt_leak() -> Result<()> {
    let mut store = Store::<()>::default();
    let flag = Arc::new(AtomicBool::new(false));

    {
        let mut scope = RootScope::new(&mut store);
        let x = ExternRef::new(&mut scope, SetFlagOnDrop(flag.clone()))?;
        let raw = unsafe { x.to_raw(&mut scope)? };
        let _x = unsafe { ExternRef::from_raw(&mut scope, raw) };
    }

    store.gc();
    assert!(flag.load(SeqCst));
    Ok(())
}

#[test]
fn table_fill_doesnt_leak() -> Result<()> {
    let _ = env_logger::try_init();

    let mut store = Store::<()>::default();
    let flag = Arc::new(AtomicBool::new(false));

    {
        let mut scope = RootScope::new(&mut store);
        let x = ExternRef::new(&mut scope, SetFlagOnDrop(flag.clone()))?;
        let table = Table::new(
            &mut scope,
            TableType::new(RefType::EXTERNREF, 10, Some(10)),
            x.into(),
        )?;
        table.fill(&mut scope, 0, Ref::Extern(None), 10)?;
    }

    store.gc();
    assert!(flag.load(SeqCst));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn table_copy_doesnt_leak() -> Result<()> {
    let _ = env_logger::try_init();

    let mut store = Store::<()>::default();
    let flag = Arc::new(AtomicBool::new(false));

    {
        let mut scope = RootScope::new(&mut store);
        let table = Table::new(
            &mut scope,
            TableType::new(RefType::EXTERNREF, 10, Some(10)),
            Ref::Extern(None),
        )?;

        let x = ExternRef::new(&mut scope, SetFlagOnDrop(flag.clone()))?;
        table.fill(&mut scope, 2, x.into(), 3)?;

        Table::copy(&mut scope, &table, 0, &table, 5, 5)?;
    }

    store.gc();
    assert!(flag.load(SeqCst));
    Ok(())
}

#[test]
fn ref_matches() -> Result<()> {
    let mut store = Store::<()>::default();
    let engine = store.engine().clone();

    let func_ty = FuncType::new(&engine, None, None);
    let func_ref_ty = RefType::new(true, HeapType::ConcreteFunc(func_ty.clone()));
    let f = Func::new(&mut store, func_ty, |_, _, _| Ok(()));

    let pre = StructRefPre::new(&mut store, StructType::new(&engine, [])?);
    let s = StructRef::new(&mut store, &pre, &[])?.to_anyref();

    let pre = ArrayRefPre::new(
        &mut store,
        ArrayType::new(&engine, FieldType::new(Mutability::Const, StorageType::I8)),
    );
    let a = ArrayRef::new(&mut store, &pre, &Val::I32(0), 0)?.to_anyref();

    let i31 = AnyRef::from_i31(&mut store, I31::wrapping_i32(1234));

    let e = ExternRef::new(&mut store, "hello")?;

    for (val, ty, expected) in [
        // nulls to nullexternref
        (Ref::Extern(None), RefType::NULLEXTERNREF, true),
        (Ref::Any(None), RefType::NULLEXTERNREF, false),
        (Ref::Func(None), RefType::NULLEXTERNREF, false),
        // nulls to externref
        (Ref::Extern(None), RefType::EXTERNREF, true),
        (Ref::Any(None), RefType::EXTERNREF, false),
        (Ref::Func(None), RefType::EXTERNREF, false),
        // nulls to nullref
        (Ref::Extern(None), RefType::NULLREF, false),
        (Ref::Any(None), RefType::NULLREF, true),
        (Ref::Func(None), RefType::NULLREF, false),
        // nulls to structref
        (Ref::Extern(None), RefType::STRUCTREF, false),
        (Ref::Any(None), RefType::STRUCTREF, true),
        (Ref::Func(None), RefType::STRUCTREF, false),
        // nulls to arrayref
        (Ref::Extern(None), RefType::ARRAYREF, false),
        (Ref::Any(None), RefType::ARRAYREF, true),
        (Ref::Func(None), RefType::ARRAYREF, false),
        // nulls to i31ref
        (Ref::Extern(None), RefType::I31REF, false),
        (Ref::Any(None), RefType::I31REF, true),
        (Ref::Func(None), RefType::I31REF, false),
        // nulls to eqref
        (Ref::Extern(None), RefType::EQREF, false),
        (Ref::Any(None), RefType::EQREF, true),
        (Ref::Func(None), RefType::EQREF, false),
        // nulls to anyref
        (Ref::Extern(None), RefType::ANYREF, false),
        (Ref::Any(None), RefType::ANYREF, true),
        (Ref::Func(None), RefType::ANYREF, false),
        // non-null structref
        (Ref::Any(Some(s)), RefType::NULLFUNCREF, false),
        (Ref::Any(Some(s)), func_ref_ty.clone(), false),
        (Ref::Any(Some(s)), RefType::FUNCREF, false),
        (Ref::Any(Some(s)), RefType::NULLEXTERNREF, false),
        (Ref::Any(Some(s)), RefType::EXTERNREF, false),
        (Ref::Any(Some(s)), RefType::NULLREF, false),
        (Ref::Any(Some(s)), RefType::STRUCTREF, true),
        (Ref::Any(Some(s)), RefType::ARRAYREF, false),
        (Ref::Any(Some(s)), RefType::I31REF, false),
        (Ref::Any(Some(s)), RefType::EQREF, true),
        (Ref::Any(Some(s)), RefType::ANYREF, true),
        // non-null arrayref
        (Ref::Any(Some(a)), RefType::NULLFUNCREF, false),
        (Ref::Any(Some(a)), func_ref_ty.clone(), false),
        (Ref::Any(Some(a)), RefType::FUNCREF, false),
        (Ref::Any(Some(a)), RefType::NULLEXTERNREF, false),
        (Ref::Any(Some(a)), RefType::EXTERNREF, false),
        (Ref::Any(Some(a)), RefType::NULLREF, false),
        (Ref::Any(Some(a)), RefType::STRUCTREF, false),
        (Ref::Any(Some(a)), RefType::ARRAYREF, true),
        (Ref::Any(Some(a)), RefType::I31REF, false),
        (Ref::Any(Some(a)), RefType::EQREF, true),
        (Ref::Any(Some(a)), RefType::ANYREF, true),
        // non-null i31ref
        (Ref::Any(Some(i31)), RefType::NULLFUNCREF, false),
        (Ref::Any(Some(i31)), func_ref_ty.clone(), false),
        (Ref::Any(Some(i31)), RefType::FUNCREF, false),
        (Ref::Any(Some(i31)), RefType::NULLEXTERNREF, false),
        (Ref::Any(Some(i31)), RefType::EXTERNREF, false),
        (Ref::Any(Some(i31)), RefType::NULLREF, false),
        (Ref::Any(Some(i31)), RefType::STRUCTREF, false),
        (Ref::Any(Some(i31)), RefType::ARRAYREF, false),
        (Ref::Any(Some(i31)), RefType::I31REF, true),
        (Ref::Any(Some(i31)), RefType::EQREF, true),
        (Ref::Any(Some(i31)), RefType::ANYREF, true),
        // non-null funcref
        (Ref::Func(Some(f.clone())), RefType::NULLFUNCREF, false),
        (Ref::Func(Some(f.clone())), func_ref_ty.clone(), true),
        (Ref::Func(Some(f.clone())), RefType::FUNCREF, true),
        (Ref::Func(Some(f.clone())), RefType::NULLEXTERNREF, false),
        (Ref::Func(Some(f.clone())), RefType::EXTERNREF, false),
        (Ref::Func(Some(f.clone())), RefType::NULLREF, false),
        (Ref::Func(Some(f.clone())), RefType::STRUCTREF, false),
        (Ref::Func(Some(f.clone())), RefType::ARRAYREF, false),
        (Ref::Func(Some(f.clone())), RefType::I31REF, false),
        (Ref::Func(Some(f.clone())), RefType::EQREF, false),
        (Ref::Func(Some(f.clone())), RefType::ANYREF, false),
        // non-null externref
        (Ref::Extern(Some(e)), RefType::NULLFUNCREF, false),
        (Ref::Extern(Some(e)), func_ref_ty.clone(), false),
        (Ref::Extern(Some(e)), RefType::FUNCREF, false),
        (Ref::Extern(Some(e)), RefType::NULLEXTERNREF, false),
        (Ref::Extern(Some(e)), RefType::EXTERNREF, true),
        (Ref::Extern(Some(e)), RefType::NULLREF, false),
        (Ref::Extern(Some(e)), RefType::STRUCTREF, false),
        (Ref::Extern(Some(e)), RefType::ARRAYREF, false),
        (Ref::Extern(Some(e)), RefType::I31REF, false),
        (Ref::Extern(Some(e)), RefType::EQREF, false),
        (Ref::Extern(Some(e)), RefType::ANYREF, false),
    ] {
        let actual = val.matches_ty(&mut store, &ty)?;
        assert_eq!(
            actual, expected,
            "{val:?} matches {ty:?}? expected {expected}, got {actual}"
        );
    }

    Ok(())
}

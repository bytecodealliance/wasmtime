//! Tests for instrumentation-based debugging.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wasmtime::{
    AsContextMut, Caller, Config, DebugEvent, DebugHandler, Engine, Extern, FrameParentResult,
    Func, Global, GlobalType, Instance, Module, Mutability, Store, StoreContextMut, Val, ValType,
};

#[test]
fn debugging_does_not_work_with_signal_based_traps() {
    let mut config = Config::default();
    config.guest_debug(true).signals_based_traps(true);
    let err = Engine::new(&config).expect_err("invalid config should produce an error");
    assert!(format!("{err:?}").contains("cannot use signals-based traps"));
}

#[test]
fn debugging_apis_are_denied_without_debugging() -> wasmtime::Result<()> {
    let mut config = Config::default();
    config.guest_debug(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, "(module (global $g (mut i32) (i32.const 0)))")?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    assert!(store.debug_frames().is_none());
    assert!(instance.debug_global(&mut store, 0).is_none());

    Ok(())
}

fn get_module_and_store<C: Fn(&mut Config)>(
    c: C,
    wat: &str,
) -> wasmtime::Result<(Module, Store<()>)> {
    let mut config = Config::default();
    config.guest_debug(true);
    config.wasm_exceptions(true);
    c(&mut config);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wat)?;
    Ok((module, Store::new(&engine, ())))
}

fn test_stack_values<C: Fn(&mut Config), F: Fn(Caller<'_, ()>) + Send + Sync + 'static>(
    wat: &str,
    c: C,
    f: F,
) -> wasmtime::Result<()> {
    let (module, mut store) = get_module_and_store(c, wat)?;
    let func = Func::wrap(&mut store, move |caller: Caller<'_, ()>| {
        f(caller);
    });
    let instance = Instance::new(&mut store, &module, &[Extern::Func(func)])?;
    let mut results = [];
    instance
        .get_func(&mut store, "main")
        .unwrap()
        .call(&mut store, &[], &mut results)?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn stack_values_two_frames() -> wasmtime::Result<()> {
    let _ = env_logger::try_init();

    for inlining in [false, true] {
        test_stack_values(
            r#"
    (module
      (import "" "host" (func))
      (func (export "main")
        i32.const 1
        i32.const 2
        call 2
        drop)
      (func (param i32 i32) (result i32)
        local.get 0
        local.get 1
        call 0
        i32.add))
    "#,
            |config| {
                config.compiler_inlining(inlining);
                if inlining {
                    unsafe {
                        config.cranelift_flag_set("wasmtime_inlining_intra_module", "true");
                    }
                }
            },
            |mut caller: Caller<'_, ()>| {
                let mut stack = caller.debug_frames().unwrap();
                assert!(!stack.done());
                assert_eq!(stack.wasm_function_index_and_pc().unwrap().0.as_u32(), 1);
                assert_eq!(stack.wasm_function_index_and_pc().unwrap().1, 65);

                assert_eq!(stack.num_locals(), 2);
                assert_eq!(stack.num_stacks(), 2);
                assert_eq!(stack.local(0).unwrap_i32(), 1);
                assert_eq!(stack.local(1).unwrap_i32(), 2);
                assert_eq!(stack.stack(0).unwrap_i32(), 1);
                assert_eq!(stack.stack(1).unwrap_i32(), 2);

                assert_eq!(stack.move_to_parent(), FrameParentResult::SameActivation);
                assert!(!stack.done());
                assert_eq!(stack.wasm_function_index_and_pc().unwrap().0.as_u32(), 0);
                assert_eq!(stack.wasm_function_index_and_pc().unwrap().1, 55);

                assert_eq!(stack.move_to_parent(), FrameParentResult::SameActivation);
                assert!(stack.done());
            },
        )?;
    }
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn stack_values_exceptions() -> wasmtime::Result<()> {
    test_stack_values(
        r#"
    (module
      (tag $t (param i32))
      (import "" "host" (func))
      (func (export "main")
        (block $b (result i32)
          (try_table (catch $t $b)
            (throw $t (i32.const 42)))
          i32.const 0)
        (call 0)
        (drop)))
    "#,
        |_config| {},
        |mut caller: Caller<'_, ()>| {
            let mut stack = caller.debug_frames().unwrap();
            assert!(!stack.done());
            assert_eq!(stack.num_stacks(), 1);
            assert_eq!(stack.stack(0).unwrap_i32(), 42);
            assert_eq!(stack.move_to_parent(), FrameParentResult::SameActivation);
            assert!(stack.done());
        },
    )
}

#[test]
#[cfg_attr(miri, ignore)]
fn stack_values_dead_gc_ref() -> wasmtime::Result<()> {
    test_stack_values(
        r#"
    (module
      (type $s (struct))
      (import "" "host" (func))
      (func (export "main")
        (struct.new $s)
        (call 0)
        (drop)))
    "#,
        |config| {
            config.wasm_gc(true);
        },
        |mut caller: Caller<'_, ()>| {
            let mut stack = caller.debug_frames().unwrap();
            assert!(!stack.done());
            assert_eq!(stack.num_stacks(), 1);
            assert!(stack.stack(0).unwrap_anyref().is_some());
            assert_eq!(stack.move_to_parent(), FrameParentResult::SameActivation);
            assert!(stack.done());
        },
    )
}

#[test]
#[cfg_attr(miri, ignore)]
fn gc_access_during_call() -> wasmtime::Result<()> {
    test_stack_values(
        r#"
    (module
      (type $s (struct (field i32)))
      (import "" "host" (func))
      (func (export "main")
        (local $l (ref null $s))
        (local.set $l (struct.new $s (i32.const 42)))
        (call 0)))
    "#,
        |config| {
            config.wasm_gc(true);
        },
        |mut caller: Caller<'_, ()>| {
            let mut stack = caller.debug_frames().unwrap();

            // Do a GC while we hold the stack cursor.
            stack.as_context_mut().gc(None);

            assert!(!stack.done());
            assert_eq!(stack.num_stacks(), 0);
            assert_eq!(stack.num_locals(), 1);
            // Note that this struct is dead during the call, and the
            // ref could otherwise be optimized away (no longer in the
            // stackmap at this point); but we verify it is still
            // alive here because it is rooted in the
            // debug-instrumentation slot.
            let s = stack
                .local(0)
                .unwrap_any_ref()
                .unwrap()
                .unwrap_struct(&stack)
                .unwrap();
            assert_eq!(s.field(&mut stack, 0).unwrap().unwrap_i32(), 42);
            assert_eq!(stack.move_to_parent(), FrameParentResult::SameActivation);
            assert!(stack.done());
        },
    )
}

#[test]
#[cfg_attr(miri, ignore)]
fn stack_values_two_activations() -> wasmtime::Result<()> {
    let _ = env_logger::try_init();

    let mut config = Config::default();
    config.guest_debug(true);
    config.wasm_exceptions(true);
    let engine = Engine::new(&config)?;
    let module1 = Module::new(
        &engine,
        r#"
    (module
      (import "" "host1" (func (param i32 i32) (result i32)))
      (func (export "main") (result i32)
        i32.const 1
        i32.const 2
        call 0))
    "#,
    )?;
    let module2 = Module::new(
        &engine,
        r#"
    (module
      (import "" "host2" (func))
      (func (export "inner") (param i32 i32) (result i32)
        local.get 0
        local.get 1
        call 0
        i32.add))
    "#,
    )?;
    let mut store = Store::new(&engine, ());

    let module1_clone = module1.clone();
    let module2_clone = module2.clone();
    let host2 = Func::wrap(&mut store, move |mut caller: Caller<'_, ()>| {
        let mut stack = caller.debug_frames().unwrap();
        assert!(!stack.done());
        assert_eq!(stack.wasm_function_index_and_pc().unwrap().0.as_u32(), 0);
        assert_eq!(stack.wasm_function_index_and_pc().unwrap().1, 56);
        assert!(Module::same(stack.module().unwrap(), &module2_clone));
        assert_eq!(stack.num_locals(), 2);
        assert_eq!(stack.num_stacks(), 2);
        assert_eq!(stack.local(0).unwrap_i32(), 1);
        assert_eq!(stack.local(1).unwrap_i32(), 2);
        assert_eq!(stack.stack(0).unwrap_i32(), 1);
        assert_eq!(stack.stack(1).unwrap_i32(), 2);
        let inner_instance = stack.instance();

        assert_eq!(stack.move_to_parent(), FrameParentResult::NewActivation);
        assert!(!stack.done());

        assert_eq!(stack.wasm_function_index_and_pc().unwrap().0.as_u32(), 0);
        assert_eq!(stack.wasm_function_index_and_pc().unwrap().1, 56);
        assert!(Module::same(stack.module().unwrap(), &module1_clone));
        assert_eq!(stack.num_locals(), 0);
        assert_eq!(stack.num_stacks(), 2);
        assert_eq!(stack.stack(0).unwrap_i32(), 1);
        assert_eq!(stack.stack(1).unwrap_i32(), 2);
        let outer_instance = stack.instance();

        assert_ne!(inner_instance, outer_instance);

        assert_eq!(stack.move_to_parent(), FrameParentResult::SameActivation);
        assert!(stack.done());
    });

    let instance2 = Instance::new(&mut store, &module2, &[Extern::Func(host2)])?;
    let inner = instance2.get_func(&mut store, "inner").unwrap();

    let host1 = Func::wrap(
        &mut store,
        move |mut caller: Caller<'_, ()>, a: i32, b: i32| -> i32 {
            let mut results = [Val::I32(0)];
            inner
                .call(&mut caller, &[Val::I32(a), Val::I32(b)], &mut results[..])
                .unwrap();
            results[0].unwrap_i32()
        },
    );

    let instance1 = Instance::new(&mut store, &module1, &[Extern::Func(host1)])?;
    let main = instance1.get_func(&mut store, "main").unwrap();

    let mut results = [Val::I32(0)];
    main.call(&mut store, &[], &mut results)?;
    assert_eq!(results[0].unwrap_i32(), 3);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn debug_frames_on_store_with_no_wasm_activation() -> wasmtime::Result<()> {
    let mut config = Config::default();
    config.guest_debug(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let frames = store
        .debug_frames()
        .expect("Debug frames should be available");
    assert!(frames.done());
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn private_entity_access() -> wasmtime::Result<()> {
    let mut config = Config::default();
    config.guest_debug(true);
    config.wasm_gc(true);
    config.gc_support(true);
    config.wasm_exceptions(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
        (module
          (import "" "i" (global (mut i32)))
          (import "" "f" (func (result i32)))
          (global $g (mut i32) (i32.const 0))
          (memory $m 1 1)
          (table $t 10 10 i31ref)
          (tag $tag (param f64))
          (func (export "main")
            ;; $g := 42
            i32.const 42
            global.set $g
            ;; $m[1024] := 1
            i32.const 1024
            i32.const 1
            i32.store8 $m
            ;; $t[1] := (ref.i31 (i32.const 100))
            i32.const 1
            i32.const 100
            ref.i31
            table.set $t)

          (func (param i32)
            local.get 0
            global.set $g))
        "#,
    )?;

    let host_global = Global::new(
        &mut store,
        GlobalType::new(ValType::I32, Mutability::Var),
        Val::I32(1000),
    )?;
    let host_func = Func::wrap(&mut store, |_caller: Caller<'_, ()>| -> i32 { 7 });

    let instance = Instance::new(
        &mut store,
        &module,
        &[Extern::Global(host_global), Extern::Func(host_func)],
    )?;
    let func = instance.get_func(&mut store, "main").unwrap();
    func.call(&mut store, &[], &mut [])?;

    // Nothing is exported except for `main`, yet we can still access
    // (below).
    let exports = instance.exports(&mut store).collect::<Vec<_>>();
    assert_eq!(exports.len(), 1);
    assert!(exports.into_iter().next().unwrap().into_func().is_some());

    // We can call a non-exported function.
    let f = instance.debug_function(&mut store, 2).unwrap();
    f.call(&mut store, &[Val::I32(1234)], &mut [])?;

    let g = instance.debug_global(&mut store, 1).unwrap();
    assert_eq!(g.get(&mut store).unwrap_i32(), 1234);

    let m = instance.debug_memory(&mut store, 0).unwrap();
    assert_eq!(m.data(&mut store)[1024], 1);

    let t = instance.debug_table(&mut store, 0).unwrap();
    let t_val = t.get(&mut store, 1).unwrap();
    let t_val = t_val.as_any().unwrap().unwrap().unwrap_i31(&store).unwrap();
    assert_eq!(t_val.get_u32(), 100);

    let tag = instance.debug_tag(&mut store, 0).unwrap();
    assert!(matches!(
        tag.ty(&store).ty().param(0).unwrap(),
        ValType::F64
    ));

    // Check that we can access an imported global in the instance's
    // index space.
    let host_global_import = instance.debug_global(&mut store, 0).unwrap();
    assert_eq!(host_global_import.get(&mut store).unwrap_i32(), 1000);

    // Check that we can call an imported function in the instance's
    // index space.
    let host_func_import = instance.debug_function(&mut store, 0).unwrap();
    let mut results = [Val::I32(0)];
    host_func_import.call(&mut store, &[], &mut results[..])?;
    assert_eq!(results[0].unwrap_i32(), 7);

    // Check that out-of-bounds returns `None` rather than panic'ing.
    assert!(instance.debug_global(&mut store, 2).is_none());

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
#[cfg(target_pointer_width = "64")] // Threads not supported on 32-bit systems.
fn private_entity_access_shared_memory() -> wasmtime::Result<()> {
    let mut config = Config::default();
    config.guest_debug(true);
    config.shared_memory(true);
    config.wasm_threads(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
        (module
          (memory 1 1 shared))
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;

    let m = instance.debug_shared_memory(&mut store, 0).unwrap();
    let unsafe_cell = &m.data()[1024];
    assert_eq!(unsafe { *unsafe_cell.get() }, 0);

    Ok(())
}

macro_rules! debug_event_checker {
    ($ty:tt,
     $store:tt,
     $(
         { $i:expr ; $pat:pat => $body:tt }
     ),*)
    =>
    {
        #[derive(Clone)]
        struct $ty(Arc<AtomicUsize>);
        impl $ty {
            fn new_and_counter() -> (Self, Arc<AtomicUsize>) {
                let counter = Arc::new(AtomicUsize::new(0));
                let counter_clone = counter.clone();
                ($ty(counter), counter_clone)
            }
        }
        impl DebugHandler for $ty {
            type Data = ();
            fn handle(
                &self,
                #[allow(unused_variables, reason = "macro rules")]
                #[allow(unused_mut, reason = "macro rules")]
                mut $store: StoreContextMut<'_, ()>,
                event: DebugEvent<'_>,
            ) -> impl Future<Output = ()> + Send {
                let step = self.0.fetch_add(1, Ordering::Relaxed);
                async move {
                    if false {}
                    $(
                        else if step == $i {
                            match event {
                                $pat => {
                                    $body;
                                }
                                _ => panic!("Incorrect event"),
                            }
                        }
                    )*
                    else {
                        panic!("Too many steps");
                    }
                }
            }
        }
    }
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn uncaught_exception_events() -> wasmtime::Result<()> {
    let _ = env_logger::try_init();

    let (module, mut store) = get_module_and_store(
        |config| {
            config.async_support(true);
            config.wasm_exceptions(true);
        },
        r#"
    (module
      (tag $t (param i32))
      (func (export "main")
        call 1)
      (func
        (local $i i32)
        (local.set $i (i32.const 100))
        (throw $t (i32.const 42))))
    "#,
    )?;

    debug_event_checker!(
        D, store,
        { 0 ;
          wasmtime::DebugEvent::UncaughtExceptionThrown(e) => {
              assert_eq!(e.field(&mut store, 0).unwrap().unwrap_i32(), 42);
              let mut stack = store.debug_frames().expect("frame cursor must be available");
              assert!(!stack.done());
              assert_eq!(stack.num_locals(), 1);
              assert_eq!(stack.local(0).unwrap_i32(), 100);
              assert_eq!(stack.move_to_parent(), FrameParentResult::SameActivation);
              assert!(!stack.done());
              assert_eq!(stack.move_to_parent(), FrameParentResult::SameActivation);
              assert!(stack.done());
          }
        }
    );

    let (handler, counter) = D::new_and_counter();
    store.set_debug_handler(handler);

    let instance = Instance::new_async(&mut store, &module, &[]).await?;
    let func = instance.get_func(&mut store, "main").unwrap();
    let mut results = [];
    let result = func.call_async(&mut store, &[], &mut results).await;
    assert!(result.is_err()); // Uncaught exception.
    assert_eq!(counter.load(Ordering::Relaxed), 1);

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn caught_exception_events() -> wasmtime::Result<()> {
    let _ = env_logger::try_init();

    let (module, mut store) = get_module_and_store(
        |config| {
            config.async_support(true);
            config.wasm_exceptions(true);
        },
        r#"
    (module
      (tag $t (param i32))
      (func (export "main")
        (block $b (result i32)
          (try_table (catch $t $b)
            call 1)
          i32.const 0)
        drop)
      (func
        (local $i i32)
        (local.set $i (i32.const 100))
        (throw $t (i32.const 42))))
    "#,
    )?;

    debug_event_checker!(
        D, store,
        { 0 ;
          wasmtime::DebugEvent::CaughtExceptionThrown(e) => {
              assert_eq!(e.field(&mut store, 0).unwrap().unwrap_i32(), 42);
              let mut stack = store.debug_frames().expect("frame cursor must be available");
              assert!(!stack.done());
              assert_eq!(stack.num_locals(), 1);
              assert_eq!(stack.local(0).unwrap_i32(), 100);
              assert_eq!(stack.move_to_parent(), FrameParentResult::SameActivation);
              assert!(!stack.done());
              assert_eq!(stack.move_to_parent(), FrameParentResult::SameActivation);
              assert!(stack.done());
          }
        }
    );

    let (handler, counter) = D::new_and_counter();
    store.set_debug_handler(handler);

    let instance = Instance::new_async(&mut store, &module, &[]).await?;
    let func = instance.get_func(&mut store, "main").unwrap();
    let mut results = [];
    func.call_async(&mut store, &[], &mut results).await?;
    assert_eq!(counter.load(Ordering::Relaxed), 1);

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn hostcall_trap_events() -> wasmtime::Result<()> {
    let _ = env_logger::try_init();

    let (module, mut store) = get_module_and_store(
        |config| {
            config.async_support(true);
            config.wasm_exceptions(true);
        },
        r#"
    (module
      (func (export "main")
        i32.const 0
        i32.const 0
        i32.div_u
        drop))
    "#,
    )?;

    debug_event_checker!(
        D, store,
        { 0 ;
          wasmtime::DebugEvent::Trap(wasmtime_environ::Trap::IntegerDivisionByZero) => {}
        }
    );

    let (handler, counter) = D::new_and_counter();
    store.set_debug_handler(handler);

    let instance = Instance::new_async(&mut store, &module, &[]).await?;
    let func = instance.get_func(&mut store, "main").unwrap();
    let mut results = [];
    let result = func.call_async(&mut store, &[], &mut results).await;
    assert!(result.is_err()); // Uncaught trap.
    assert_eq!(counter.load(Ordering::Relaxed), 1);

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn hostcall_error_events() -> wasmtime::Result<()> {
    let _ = env_logger::try_init();

    let (module, mut store) = get_module_and_store(
        |config| {
            config.async_support(true);
            config.wasm_exceptions(true);
        },
        r#"
    (module
      (import "" "do_a_trap" (func))
      (func (export "main")
        call 0))
    "#,
    )?;

    debug_event_checker!(
        D, store,
        { 0 ;
          wasmtime::DebugEvent::HostcallError(e) => {
              assert!(format!("{e:?}").contains("secret error message"));
          }
        }
    );

    let (handler, counter) = D::new_and_counter();
    store.set_debug_handler(handler);

    let do_a_trap = Func::wrap(
        &mut store,
        |_caller: Caller<'_, ()>| -> wasmtime::Result<()> {
            Err(wasmtime::format_err!("secret error message"))
        },
    );
    let instance = Instance::new_async(&mut store, &module, &[Extern::Func(do_a_trap)]).await?;
    let func = instance.get_func(&mut store, "main").unwrap();
    let mut results = [];
    let result = func.call_async(&mut store, &[], &mut results).await;
    assert!(result.is_err()); // Uncaught trap.
    assert_eq!(counter.load(Ordering::Relaxed), 1);
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn breakpoint_events() -> wasmtime::Result<()> {
    let _ = env_logger::try_init();

    let (module, mut store) = get_module_and_store(
        |config| {
            config.async_support(true);
            config.wasm_exceptions(true);
        },
        r#"
    (module
      (func (export "main") (param i32 i32) (result i32)
        local.get 0
        local.get 1
        i32.add))
    "#,
    )?;

    debug_event_checker!(
        D, store,
        { 0 ;
          wasmtime::DebugEvent::Breakpoint => {
              let mut stack = store.debug_frames().expect("frame cursor must be available");
              assert!(!stack.done());
              assert_eq!(stack.num_locals(), 2);
              assert_eq!(stack.local(0).unwrap_i32(), 1);
              assert_eq!(stack.local(1).unwrap_i32(), 2);
              let (func, pc) = stack.wasm_function_index_and_pc().unwrap();
              assert_eq!(func.as_u32(), 0);
              assert_eq!(pc, 0x28);
              assert_eq!(stack.move_to_parent(), FrameParentResult::SameActivation);
              assert!(stack.done());
          }
        }
    );

    let (handler, counter) = D::new_and_counter();
    store.set_debug_handler(handler);
    store
        .edit_breakpoints()
        .unwrap()
        .add_breakpoint(&module, 0x28)?;

    let instance = Instance::new_async(&mut store, &module, &[]).await?;
    let func = instance.get_func(&mut store, "main").unwrap();
    let mut results = [Val::I32(0)];
    func.call_async(&mut store, &[Val::I32(1), Val::I32(2)], &mut results)
        .await?;
    assert_eq!(counter.load(Ordering::Relaxed), 1);
    assert_eq!(results[0].unwrap_i32(), 3);

    let breakpoints = store.breakpoints().unwrap().collect::<Vec<_>>();
    assert_eq!(breakpoints.len(), 1);
    assert!(Module::same(&breakpoints[0].module, &module));
    assert_eq!(breakpoints[0].pc, 0x28);

    store
        .edit_breakpoints()
        .unwrap()
        .remove_breakpoint(&module, 0x28)?;
    func.call_async(&mut store, &[Val::I32(1), Val::I32(2)], &mut results)
        .await?;
    assert_eq!(counter.load(Ordering::Relaxed), 1); // Should not have incremented from above.
    assert_eq!(results[0].unwrap_i32(), 3);

    // Enable single-step mode (on top of the breakpoint already enabled).
    assert!(!store.is_single_step());
    store.edit_breakpoints().unwrap().single_step(true).unwrap();
    assert!(store.is_single_step());

    debug_event_checker!(
        D2, store,
        { 0 ;
          wasmtime::DebugEvent::Breakpoint => {
              let stack = store.debug_frames().unwrap();
              assert!(!stack.done());
              let (_, pc) = stack.wasm_function_index_and_pc().unwrap();
              assert_eq!(pc, 0x24);
          }
        },
        {
          1 ;
          wasmtime::DebugEvent::Breakpoint => {
              let stack = store.debug_frames().unwrap();
              assert!(!stack.done());
              let (_, pc) = stack.wasm_function_index_and_pc().unwrap();
              assert_eq!(pc, 0x26);
          }
        },
        {
          2 ;
          wasmtime::DebugEvent::Breakpoint => {
              let stack = store.debug_frames().unwrap();
              assert!(!stack.done());
              let (_, pc) = stack.wasm_function_index_and_pc().unwrap();
              assert_eq!(pc, 0x28);
          }
        },
        {
          3 ;
          wasmtime::DebugEvent::Breakpoint => {
              let stack = store.debug_frames().unwrap();
              assert!(!stack.done());
              let (_, pc) = stack.wasm_function_index_and_pc().unwrap();
              assert_eq!(pc, 0x29);
          }
        }
    );

    let (handler, counter) = D2::new_and_counter();
    store.set_debug_handler(handler);

    func.call_async(&mut store, &[Val::I32(1), Val::I32(2)], &mut results)
        .await?;
    assert_eq!(counter.load(Ordering::Relaxed), 4);

    // Re-enable individual breakpoint.
    store
        .edit_breakpoints()
        .unwrap()
        .add_breakpoint(&module, 0x28)
        .unwrap();

    // Now disable single-stepping. The single breakpoint set above
    // should still remain.
    store
        .edit_breakpoints()
        .unwrap()
        .single_step(false)
        .unwrap();

    let (handler, counter) = D::new_and_counter();
    store.set_debug_handler(handler);

    func.call_async(&mut store, &[Val::I32(1), Val::I32(2)], &mut results)
        .await?;
    assert_eq!(counter.load(Ordering::Relaxed), 1);

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn breakpoints_in_inlined_code() -> wasmtime::Result<()> {
    let _ = env_logger::try_init();

    let (module, mut store) = get_module_and_store(
        |config| {
            config.async_support(true);
            config.wasm_exceptions(true);
            config.compiler_inlining(true);
            unsafe {
                config.cranelift_flag_set("wasmtime_inlining_intra_module", "true");
            }
        },
        r#"
    (module
      (func $f (export "f") (param i32 i32) (result i32)
        local.get 0
        local.get 1
        i32.add)
      
      (func (export "main") (param i32 i32) (result i32)
        local.get 0
        local.get 1
        call $f))
    "#,
    )?;

    debug_event_checker!(
        D, store,
        { 0 ;
          wasmtime::DebugEvent::Breakpoint => {}
        },
        { 1 ;
          wasmtime::DebugEvent::Breakpoint => {}
        }
    );

    let (handler, counter) = D::new_and_counter();
    store.set_debug_handler(handler);
    store
        .edit_breakpoints()
        .unwrap()
        .add_breakpoint(&module, 0x2d)?; // `i32.add` in `$f`.

    let instance = Instance::new_async(&mut store, &module, &[]).await?;
    let func_main = instance.get_func(&mut store, "main").unwrap();
    let func_f = instance.get_func(&mut store, "f").unwrap();
    let mut results = [Val::I32(0)];
    // Breakpoint in `$f` should have been hit in `main` even if it
    // was inlined.
    func_main
        .call_async(&mut store, &[Val::I32(1), Val::I32(2)], &mut results)
        .await?;
    assert_eq!(counter.load(Ordering::Relaxed), 1);
    assert_eq!(results[0].unwrap_i32(), 3);

    // Breakpoint in `$f` should be hit when called directly, too.
    func_f
        .call_async(&mut store, &[Val::I32(1), Val::I32(2)], &mut results)
        .await?;
    assert_eq!(counter.load(Ordering::Relaxed), 2);
    assert_eq!(results[0].unwrap_i32(), 3);

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn epoch_events() -> wasmtime::Result<()> {
    let _ = env_logger::try_init();

    let (module, mut store) = get_module_and_store(
        |config| {
            config.async_support(true);
            config.epoch_interruption(true);
        },
        r#"
    (module
      (func $f (export "f") (param i32 i32) (result i32)
        local.get 0
        local.get 1
        i32.add))
    "#,
    )?;

    debug_event_checker!(
        D, store,
        { 0 ;
          wasmtime::DebugEvent::EpochYield => {}
        }
    );

    let (handler, counter) = D::new_and_counter();
    store.set_debug_handler(handler);

    store.set_epoch_deadline(1);
    store.epoch_deadline_async_yield_and_update(1);
    store.engine().increment_epoch();

    let instance = Instance::new_async(&mut store, &module, &[]).await?;
    let func_f = instance.get_func(&mut store, "f").unwrap();
    let mut results = [Val::I32(0)];
    func_f
        .call_async(&mut store, &[Val::I32(1), Val::I32(2)], &mut results)
        .await?;
    assert_eq!(counter.load(Ordering::Relaxed), 1);
    assert_eq!(results[0].unwrap_i32(), 3);

    Ok(())
}

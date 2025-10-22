//! Tests for instrumentation-based debugging.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wasmtime::{
    AsContextMut, Caller, Config, DebugEvent, DebugHandler, Engine, Extern, Func, Instance, Module,
    Store, StoreContextMut,
};

fn get_module_and_store<C: Fn(&mut Config)>(
    c: C,
    wat: &str,
) -> anyhow::Result<(Module, Store<()>)> {
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
) -> anyhow::Result<()> {
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
fn stack_values_two_frames() -> anyhow::Result<()> {
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

                stack.move_to_parent();
                assert!(!stack.done());
                assert_eq!(stack.wasm_function_index_and_pc().unwrap().0.as_u32(), 0);
                assert_eq!(stack.wasm_function_index_and_pc().unwrap().1, 55);

                stack.move_to_parent();
                assert!(stack.done());
            },
        )?;
    }
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn stack_values_exceptions() -> anyhow::Result<()> {
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
            stack.move_to_parent();
            assert!(stack.done());
        },
    )
}

#[test]
#[cfg_attr(miri, ignore)]
fn stack_values_dead_gc_ref() -> anyhow::Result<()> {
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
            stack.move_to_parent();
            assert!(stack.done());
        },
    )
}

#[test]
#[cfg_attr(miri, ignore)]
fn gc_access_during_call() -> anyhow::Result<()> {
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
            stack.move_to_parent();
            assert!(stack.done());
        },
    )
}

#[test]
#[cfg_attr(miri, ignore)]
fn debug_frames_on_store_with_no_wasm_activation() -> anyhow::Result<()> {
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

macro_rules! debug_event_checker {
    ($ty:tt,
     $store:tt,
     $(
         { $i:expr ; $pat:pat => $body:tt }
     ),*)
    =>
    {
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
            fn handle<'a>(
                self: Arc<Self>,
                #[allow(unused_variables, reason = "macro rules")]
                #[allow(unused_mut, reason = "macro rules")]
                mut $store: StoreContextMut<'a, ()>,
                event: DebugEvent,
            ) -> Box<dyn Future<Output = ()> + Send + 'a> {
                let step = self.0.fetch_add(1, Ordering::Relaxed);
                Box::new(async move {
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
                })
            }
        }
    }
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn uncaught_exception_events() -> anyhow::Result<()> {
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
              stack.move_to_parent();
              assert!(!stack.done());
              stack.move_to_parent();
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
async fn caught_exception_events() -> anyhow::Result<()> {
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
              stack.move_to_parent();
              assert!(!stack.done());
              stack.move_to_parent();
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
async fn hostcall_trap_events() -> anyhow::Result<()> {
    let _ = env_logger::try_init();

    let (module, mut store) = get_module_and_store(
        |config| {
            config.async_support(true);
            config.wasm_exceptions(true);
            // Force hostcall-based traps.
            config.signals_based_traps(false);
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
async fn hostcall_error_events() -> anyhow::Result<()> {
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
          wasmtime::DebugEvent::HostcallError => {}
        }
    );

    let (handler, counter) = D::new_and_counter();
    store.set_debug_handler(handler);

    let do_a_trap = Func::wrap(
        &mut store,
        |_caller: Caller<'_, ()>| -> anyhow::Result<()> { Err(anyhow::anyhow!("error")) },
    );
    let instance = Instance::new_async(&mut store, &module, &[Extern::Func(do_a_trap)]).await?;
    let func = instance.get_func(&mut store, "main").unwrap();
    let mut results = [];
    let result = func.call_async(&mut store, &[], &mut results).await;
    assert!(result.is_err()); // Uncaught trap.
    assert_eq!(counter.load(Ordering::Relaxed), 1);
    Ok(())
}

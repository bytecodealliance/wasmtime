use crate::async_functions::{PollOnce, execute_across_threads};
use futures::stream::{FuturesUnordered, TryStreamExt as _};
use std::collections::HashMap;
use std::future::{self, Future};
use std::iter;
use std::mem;
use std::ops::DerefMut;
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use wasmtime::Result;
use wasmtime::component::{TaskGroupHook, TaskGroupId};
use wasmtime::{Config, Engine, Store, StoreContextMut, Trap, component::*};
use wasmtime_component_util::REALLOC_AND_FREE;

/// This is super::func::thunks, except with an async store.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn smoke() -> Result<()> {
    let component = r#"
        (component
            (core module $m
                (func (export "thunk"))
                (func (export "thunk-trap") unreachable)
            )
            (core instance $i (instantiate $m))
            (func (export "thunk")
                (canon lift (core func $i "thunk"))
            )
            (func (export "thunk-trap")
                (canon lift (core func $i "thunk-trap"))
            )
        )
    "#;

    let engine = super::async_engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;

    let thunk = instance.get_typed_func::<(), ()>(&mut store, "thunk")?;

    thunk.call_async(&mut store, ()).await?;

    let err = instance
        .get_typed_func::<(), ()>(&mut store, "thunk-trap")?
        .call_async(&mut store, ())
        .await
        .unwrap_err();
    assert_eq!(err.downcast::<Trap>()?, Trap::UnreachableCodeReached);

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn run_concurrent_yields_during_high_priority_work() -> Result<()> {
    const TASKS: usize = 256;

    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let completed = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();

    // Spawning store tasks queues high-priority work. Each task is immediately
    // ready, so the event loop can drain them without returning to Tokio.
    for _ in 0..TASKS {
        let completed = completed.clone();
        handles.push(store.spawn(async move |_| {
            completed.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })?);
    }

    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        _ = tx.send(completed.load(Ordering::Relaxed));
    });

    // On a current-thread runtime, the spawned Tokio task can only run once
    // `run_concurrent` returns Pending. It should get a turn before all store
    // tasks have completed.
    let completed_when_tokio_ran = store.run_concurrent(async |_| rx.await).await??;
    store
        .run_concurrent(async move |_| {
            for handle in handles {
                handle.await;
            }
        })
        .await?;
    assert!(
        completed_when_tokio_ran < TASKS,
        "Tokio did not run until all {TASKS} store tasks completed"
    );
    store.assert_concurrent_state_empty();
    Ok(())
}

// A callback-driven guest can continuously cancel host subtasks. If Tokio's
// cooperative budget is exhausted, even a ready timer cannot complete until
// Wasmtime returns Pending to Tokio.
#[tokio::test(flavor = "current_thread")]
#[cfg_attr(miri, ignore)]
async fn guest_cancellation_loop_does_not_starve_tokio() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    config.wasm_component_model_more_async_builtins(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(
        &engine,
        r#"
        (component
          (import "burn" (func $burn))
          (import "probe" (func $probe async))
          (import "pending" (func $pending async))

          (core module $m
            (import "" "burn" (func $burn))
            (import "" "probe" (func $probe (result i32)))
            (import "" "pending" (func $pending (result i32)))
            (import "" "cancel" (func $cancel (param i32) (result i32)))
            (import "" "drop" (func $drop (param i32)))
            (import "" "new" (func $new (result i32)))
            (import "" "join" (func $join (param i32 i32)))
            (import "" "task.return" (func $task.return))

            (global $set (mut i32) (i32.const 0))
            (global $probe-handle (mut i32) (i32.const 0))
            (global $pending-handle (mut i32) (i32.const 0))
            (global $cancellations (mut i32) (i32.const 0))

            (func $start (param $status i32) (result i32)
              (if (i32.ne (i32.and (local.get $status) (i32.const 15)) (i32.const 1))
                (then unreachable))
              (i32.shr_u (local.get $status) (i32.const 4)))

            (func $start-pending
              (global.set $pending-handle (call $start (call $pending)))
              (if (i32.ne (call $cancel (global.get $pending-handle)) (i32.const -1))
                (then unreachable))
              (call $join (global.get $pending-handle) (global.get $set)))

            (func (export "run") (result i32)
              (call $burn)
              (global.set $set (call $new))
              (global.set $probe-handle (call $start (call $probe)))
              (call $join (global.get $probe-handle) (global.get $set))
              (call $start-pending)
              (i32.or (i32.const 2) (i32.shl (global.get $set) (i32.const 4))))

            (func (export "callback") (param $event i32) (param $waitable i32)
              (param $code i32) (result i32)
              (if (i32.ne (local.get $event) (i32.const 1)) (then unreachable))
              (if (i32.eq (local.get $waitable) (global.get $probe-handle))
                (then
                  (if (i32.ne (local.get $code) (i32.const 2)) (then unreachable))
                  (call $drop (global.get $probe-handle))
                  (call $task.return)
                  (return (i32.const 0))))
              (if (i32.ne (local.get $waitable) (global.get $pending-handle))
                (then unreachable))
              (if (i32.ne (local.get $code) (i32.const 4)) (then unreachable))
              (call $drop (global.get $pending-handle))
              (global.set $cancellations (i32.add (global.get $cancellations) (i32.const 1)))
              (if (i32.gt_u (global.get $cancellations) (i32.const 500))
                (then unreachable))
              (call $start-pending)
              (i32.or (i32.const 2) (i32.shl (global.get $set) (i32.const 4))))
          )

          (core func $burn (canon lower (func $burn)))
          (core func $probe (canon lower (func $probe) async))
          (core func $pending (canon lower (func $pending) async))
          (core func $cancel (canon subtask.cancel async))
          (core func $drop (canon subtask.drop))
          (core func $new (canon waitable-set.new))
          (core func $join (canon waitable.join))
          (core func $task.return (canon task.return))
          (core instance $i (instantiate $m (with "" (instance
            (export "burn" (func $burn))
            (export "probe" (func $probe))
            (export "pending" (func $pending))
            (export "cancel" (func $cancel))
            (export "drop" (func $drop))
            (export "new" (func $new))
            (export "join" (func $join))
            (export "task.return" (func $task.return))
          ))))
          (func (export "run") async
            (canon lift (core func $i "run") async
              (callback (core func $i "callback"))))
        )
        "#,
    )?;

    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("burn", |_: StoreContextMut<()>, ()| {
            let mut context = Context::from_waker(std::task::Waker::noop());
            let mut exhausted = false;
            for _ in 0..256 {
                let (sender, mut receiver) = tokio::sync::oneshot::channel();
                sender.send(()).unwrap();
                if Pin::new(&mut receiver).poll(&mut context).is_pending() {
                    exhausted = true;
                    break;
                }
            }
            assert!(exhausted, "failed to exhaust Tokio's cooperative budget");
            Ok(())
        })?;
    linker.root().func_wrap_concurrent("probe", |_, ()| {
        Box::pin(async {
            tokio::time::sleep(std::time::Duration::ZERO).await;
            Ok(())
        })
    })?;
    linker.root().func_wrap_concurrent("pending", |_, ()| {
        Box::pin(async {
            std::future::pending::<()>().await;
            Ok(())
        })
    })?;
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate_async(&mut store, &component).await?;
    instance
        .get_typed_func::<(), ()>(&mut store, "run")?
        .call_async(&mut store, ())
        .await?;
    Ok(())
}

/// Handle an import function, created using component::Linker::func_wrap_async.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn smoke_func_wrap() -> Result<()> {
    let component = r#"
        (component
            (type $f (func))
            (import "i" (func $f))

            (core module $m
                (import "imports" "i" (func $i))
                (func (export "thunk") call $i)
            )

            (core func $f (canon lower (func $f)))
            (core instance $i (instantiate $m
                (with "imports" (instance
                    (export "i" (func $f))
                ))
             ))
            (func (export "thunk")
                (canon lift (core func $i "thunk"))
            )
        )
    "#;

    let engine = super::async_engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    let mut root = linker.root();
    root.func_wrap_async("i", |_: StoreContextMut<()>, _: ()| {
        Box::new(async { Ok(()) })
    })?;

    let instance = linker.instantiate_async(&mut store, &component).await?;

    let thunk = instance.get_typed_func::<(), ()>(&mut store, "thunk")?;

    thunk.call_async(&mut store, ()).await?;

    Ok(())
}

// This test stresses TLS management in combination with the `realloc` option
// for imported functions. This will create an async computation which invokes a
// component that invokes an imported function. The imported function returns a
// list which will require invoking malloc.
//
// As an added stressor all polls are sprinkled across threads through
// `execute_across_threads`. Yields are injected liberally by configuring 1
// fuel consumption to trigger a yield.
//
// Overall a yield should happen during malloc which should be an "interesting
// situation" with respect to the runtime.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn resume_separate_thread() -> Result<()> {
    let mut config = wasmtime_test_util::component::config();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let component = format!(
        r#"
            (component
                (import "yield" (func $yield (result (list u8))))
                (core module $libc
                    (memory (export "memory") 1)
                    {REALLOC_AND_FREE}
                )
                (core instance $libc (instantiate $libc))

                (core func $yield
                    (canon lower
                        (func $yield)
                        (memory (core memory $libc "memory"))
                        (realloc (core func $libc "realloc"))
                    )
                )

                (core module $m
                    (import "" "yield" (func $yield (param i32)))
                    (import "libc" "memory" (memory 0))
                    (func $start
                        i32.const 8
                        call $yield
                    )
                    (start $start)
                )
                (core instance (instantiate $m
                    (with "" (instance (export "yield" (func $yield))))
                    (with "libc" (instance $libc))
                ))
            )
        "#
    );
    let component = Component::new(&engine, component)?;
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap_async("yield", |_: StoreContextMut<()>, _: ()| {
            Box::new(async {
                tokio::task::yield_now().await;
                Ok((vec![1u8, 2u8],))
            })
        })?;

    execute_across_threads(async move {
        let mut store = Store::new(&engine, ());
        store.set_fuel(u64::MAX).unwrap();
        store.fuel_async_yield_interval(Some(1)).unwrap();
        linker.instantiate_async(&mut store, &component).await?;
        Ok::<_, wasmtime::Error>(())
    })
    .await?;
    Ok(())
}

// This test is intended to stress TLS management in the component model around
// the management of the `realloc` function. This creates an async computation
// representing the execution of a component model function where entry into the
// component uses `realloc` and then the component runs. This async computation
// is then polled iteratively with another "wasm activation" (in this case a
// core wasm function) on the stack. The poll-per-call should work and nothing
// should in theory have problems here.
//
// As an added stressor all polls are sprinkled across threads through
// `execute_across_threads`. Yields are injected liberally by configuring 1
// fuel consumption to trigger a yield.
//
// Overall a yield should happen during malloc which should be an "interesting
// situation" with respect to the runtime.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn poll_through_wasm_activation() -> Result<()> {
    let mut config = wasmtime_test_util::component::config();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let component = format!(
        r#"
            (component
                (core module $m
                    {REALLOC_AND_FREE}
                    (memory (export "memory") 1)
                    (func (export "run") (param i32 i32)
                    )
                )
                (core instance $i (instantiate $m))
                (func (export "run") (param "x" (list u8))
                    (canon lift (core func $i "run")
                                (memory (core memory $i "memory"))
                                (realloc (core func $i "realloc"))))
            )
        "#
    );
    let component = Component::new(&engine, component)?;
    let linker = Linker::new(&engine);

    let invoke_component = {
        let engine = engine.clone();
        async move {
            let mut store = Store::new(&engine, ());
            store.set_fuel(u64::MAX).unwrap();
            store.fuel_async_yield_interval(Some(1)).unwrap();
            let instance = linker.instantiate_async(&mut store, &component).await?;
            let func = instance.get_typed_func::<(Vec<u8>,), ()>(&mut store, "run")?;
            func.call_async(&mut store, (vec![1, 2, 3],)).await?;
            Ok::<_, wasmtime::Error>(())
        }
    };

    execute_across_threads(async move {
        let mut store = Store::new(&engine, Some(Box::pin(invoke_component)));
        let poll_once = wasmtime::Func::wrap_async(&mut store, |mut cx, _: ()| {
            let invoke_component = cx.data_mut().take().unwrap();
            Box::new(async move {
                match PollOnce::new(invoke_component).await {
                    Ok(result) => {
                        result?;
                        Ok(1)
                    }
                    Err(future) => {
                        *cx.data_mut() = Some(future);
                        Ok(0)
                    }
                }
            })
        });
        let poll_once = poll_once.typed::<(), i32>(&mut store)?;
        while poll_once.call_async(&mut store, ()).await? != 1 {
            // loop around to call again
        }
        Ok::<_, wasmtime::Error>(())
    })
    .await?;
    Ok(())
}

/// Test async drop method for host resources.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn drop_resource_async() -> Result<()> {
    use std::sync::Arc;
    use std::sync::Mutex;

    let engine = super::async_engine();
    let c = Component::new(
        &engine,
        r#"
            (component
                (import "t" (type $t (sub resource)))

                (core func $drop (canon resource.drop $t))

                (core module $m
                    (import "" "drop" (func $drop (param i32)))
                    (func (export "f") (param i32)
                        (call $drop (local.get 0))
                    )
                )
                (core instance $i (instantiate $m
                    (with "" (instance
                        (export "drop" (func $drop))
                    ))
                ))

                (func (export "f") (param "x" (own $t))
                    (canon lift (core func $i "f")))
            )
        "#,
    )?;

    struct MyType;

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    let drop_status = Arc::new(Mutex::new("not dropped"));
    let ds = drop_status.clone();

    linker
        .root()
        .resource_async("t", ResourceType::host::<MyType>(), move |_, _| {
            let ds = ds.clone();
            Box::new(async move {
                *ds.lock().unwrap() = "before yield";
                tokio::task::yield_now().await;
                *ds.lock().unwrap() = "after yield";
                Ok(())
            })
        })?;
    let i = linker.instantiate_async(&mut store, &c).await?;
    let f = i.get_typed_func::<(Resource<MyType>,), ()>(&mut store, "f")?;

    execute_across_threads(async move {
        let resource = Resource::new_own(100);
        f.call_async(&mut store, (resource,)).await?;
        Ok::<_, wasmtime::Error>(())
    })
    .await?;

    assert_eq!("after yield", *drop_status.lock().unwrap());

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn cancel_host_future() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;

    let component = Component::new(
        &engine,
        r#"
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.cancel-read" (func $future.cancel-read (param i32) (result i32)))
    (memory (export "memory") 1)

    (func (export "run") (param i32)
      ;; read/cancel attempt 1
      (call $future.read (local.get 0) (i32.const 100))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      (call $future.cancel-read (local.get 0))
      i32.const 2 ;; CANCELLED
      i32.ne
      if unreachable end

      ;; read/cancel attempt 2
      (call $future.read (local.get 0) (i32.const 100))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      (call $future.cancel-read (local.get 0))
      i32.const 2 ;; CANCELLED
      i32.ne
      if unreachable end
    )
  )

  (type $f (future u32))
  (core func $future.read (canon future.read $f async (memory (core memory $libc "memory"))))
  (core func $future.cancel-read (canon future.cancel-read $f))

  (core instance $i (instantiate $m
    (with "" (instance
      (export "future.read" (func $future.read))
      (export "future.cancel-read" (func $future.cancel-read))
    ))
  ))

  (func (export "run") async (param "f" $f)
    (canon lift
      (core func $i "run")
      (memory (core memory $libc "memory"))
    )
  )
)
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let func = instance.get_typed_func::<(FutureReader<u32>,), ()>(&mut store, "run")?;
    let reader = FutureReader::new(&mut store, MyFutureReader)?;
    func.call_async(&mut store, (reader,)).await?;

    return Ok(());

    struct MyFutureReader;

    impl FutureProducer<()> for MyFutureReader {
        type Item = u32;

        fn poll_produce(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _store: StoreContextMut<()>,
            finish: bool,
        ) -> Poll<Result<Option<Self::Item>>> {
            if finish {
                Poll::Ready(Ok(None))
            } else {
                Poll::Pending
            }
        }
    }
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn run_wasm_in_call_async() -> Result<()> {
    _ = env_logger::try_init();

    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;

    let a = Component::new(
        &engine,
        r#"
(component
  (type $t (func async))
  (import "a" (func $f (type $t)))
  (core func $f (canon lower (func $f)))
  (core module $a
    (import "" "f" (func $f))
    (func (export "run") call $f)
  )
  (core instance $a (instantiate $a
    (with "" (instance (export "f" (func $f))))
  ))
  (func (export "run") (type $t)
    (canon lift (core func $a "run")))
)
        "#,
    )?;
    let b = Component::new(
        &engine,
        r#"
(component
  (type $t (func async))
  (core module $a
    (func (export "run"))
  )
  (core instance $a (instantiate $a))
  (func (export "run") (type $t)
    (canon lift (core func $a "run")))
)
        "#,
    )?;

    type State = Option<Instance>;

    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap_concurrent("a", |accessor: &Accessor<State>, (): ()| {
            Box::pin(async move {
                let func = accessor.with(|mut access| {
                    access
                        .get()
                        .unwrap()
                        .get_typed_func::<(), ()>(&mut access, "run")
                })?;
                func.call_concurrent(accessor, ()).await?;
                Ok(())
            })
        })?;
    let mut store = Store::new(&engine, None);
    let instance_a = linker.instantiate_async(&mut store, &a).await?;
    let instance_b = linker.instantiate_async(&mut store, &b).await?;
    *store.data_mut() = Some(instance_b);
    let run = instance_a.get_typed_func::<(), ()>(&mut store, "run")?;
    run.call_async(&mut store, ()).await?;
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn require_concurrency_support() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    let mut store = Store::new(&engine, ());

    assert!(
        store
            .run_concurrent(async |_| wasmtime::error::Ok(()))
            .await
            .is_err()
    );

    assert!(StreamReader::<u32>::new(&mut store, Vec::new()).is_err());
    assert!(FutureReader::new(&mut store, async { wasmtime::error::Ok(0) }).is_err());

    let mut linker = Linker::<()>::new(&engine);
    let mut root = linker.root();

    assert!(
        root.func_wrap_concurrent::<(), (), _>("f1", |_, _| { todo!() })
            .is_err()
    );
    assert!(
        root.func_new_concurrent("f2", |_, _, _, _| { todo!() })
            .is_err()
    );
    assert!(
        root.resource_concurrent("f3", ResourceType::host::<u32>(), |_, _| { todo!() })
            .is_err()
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn cancel_host_task_does_not_leak() -> Result<()> {
    test_cancel_host_task(HostTaskCancelMode::Sync).await
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn async_cancel_host_task_does_not_leak() -> Result<()> {
    test_cancel_host_task(HostTaskCancelMode::Async).await
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn async_cancel_host_task_drop_before_completion_traps() -> Result<()> {
    test_cancel_host_task(HostTaskCancelMode::DropDuringAsyncCancel).await
}

enum HostTaskCancelMode {
    Sync,
    Async,
    DropDuringAsyncCancel,
}

async fn test_cancel_host_task(mode: HostTaskCancelMode) -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    config.wasm_component_model_more_async_builtins(true);
    config.wasm_component_model_async_stackful(true);
    let engine = Engine::new(&config)?;

    let mut store = Store::new(&engine, ());
    let async_cancel = !matches!(mode, HostTaskCancelMode::Sync);
    let drop_early = matches!(mode, HostTaskCancelMode::DropDuringAsyncCancel);
    let cancel_option = if async_cancel { "async" } else { "" };
    let expected_cancel_status = if async_cancel { -1 } else { 4 };
    let early_drop = if drop_early {
        r#"
                    ;; Cancellation returned BLOCKED, so dropping must trap.
                    (call $drop (local.get $task))
                    ;; Stop here if the drop was incorrectly accepted.
                    unreachable
        "#
    } else {
        ""
    };
    let component = Component::new(
        &engine,
        format!(
            r#"(component
            (import "f" (func $f async))
            (core module $memory (memory (export "memory") 1))
            (core instance $memory (instantiate $memory))

            (core module $m
                (import "" "memory" (memory 1))
                (import "" "f" (func $f (result i32)))
                (import "" "cancel" (func $cancel (param i32) (result i32)))
                (import "" "drop" (func $drop (param i32)))
                (import "" "new-set" (func $new-set (result i32)))
                (import "" "join" (func $join (param i32 i32)))
                (import "" "wait" (func $wait (param i32 i32) (result i32)))
                (import "" "drop-set" (func $drop-set (param i32)))
                (func (export "run")
                    (local $task i32) (local $status i32) (local $set i32)

                    ;; start the subtask, asserting it's `STARTED`
                    call $f
                    local.tee 0
                    i32.const 0xf
                    i32.and
                    i32.const 1 ;; STARTED
                    i32.ne
                    if unreachable end

                    ;; extract the task id
                    local.get 0
                    i32.const 4
                    i32.shr_u
                    local.set 0

                    ;; Request cancellation and check its status.
                    (local.set $status (call $cancel (local.get $task)))
                    (if (i32.ne (local.get $status) (i32.const {expected_cancel_status}))
                        (then unreachable))
                    {early_drop}
                    (if (i32.eq (local.get $status) (i32.const -1))
                        (then
                            (local.set $set (call $new-set))
                            (call $join (local.get $task) (local.get $set))
                            (if (i32.ne (call $wait (local.get $set) (i32.const 0)) (i32.const 1))
                                (then unreachable))
                            (if (i32.ne (i32.load (i32.const 0)) (local.get $task))
                                (then unreachable))
                            (local.set $status (i32.load (i32.const 4)))))
                    (if (i32.ne (local.get $status) (i32.const 4)) ;; RETURN_CANCELLED
                        (then unreachable))

                    ;; drop the subtask
                    local.get 0
                    call $drop
                    (if (local.get $set) (then (call $drop-set (local.get $set))))
                )
            )
            (core func $f (canon lower (func $f) async))
            (core func $cancel (canon subtask.cancel {cancel_option}))
            (core func $drop (canon subtask.drop))
            (core func $new-set (canon waitable-set.new))
            (core func $join (canon waitable.join))
            (core func $wait (canon waitable-set.wait (memory (core memory $memory "memory"))))
            (core func $drop-set (canon waitable-set.drop))
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "memory" (memory $memory "memory"))
                    (export "f" (func $f))
                    (export "cancel" (func $cancel))
                    (export "drop" (func $drop))
                    (export "new-set" (func $new-set))
                    (export "join" (func $join))
                    (export "wait" (func $wait))
                    (export "drop-set" (func $drop-set))
                ))
            ))

            (func (export "f") async
                (canon lift (core func $i "run")))


        )"#
        ),
    )?;

    let mut linker = Linker::new(&engine);
    linker.root().func_wrap_concurrent("f", |_, ()| {
        Box::pin(async move {
            std::future::pending::<()>().await;
            Ok(())
        })
    })?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "f")?;
    let result = store
        .run_concurrent(async |store| -> wasmtime::Result<()> {
            func.call_concurrent(store, ()).await?;

            for _ in 0..5 {
                tokio::task::yield_now().await;
            }
            Ok(())
        })
        .await
        .and_then(std::convert::identity);

    if drop_early {
        let err = result.unwrap_err();
        assert_eq!(err.downcast::<Trap>()?, Trap::SubtaskDropNotResolved);
    } else {
        result?;
        // The host task was cancelled, nothing should remain.
        store.assert_concurrent_state_empty();
    }

    Ok(())
}

/// Instantiates the component `wat`, with any extra host functions defined by
/// `define_host`, and calls its `run` export once.
///
/// The component may import a `record` function which it should call as it
/// runs to keep track of the maximum size of the table in concurrent_state.
///
/// Asserts that the concurrent state is cleaned up both at the end and
/// incrementally along the way, i.e. that the table never reaches
/// `max_table_size` entries.
async fn assert_concurrent_state_does_not_leak(
    wat: &str,
    max_table_size: usize,
    define_host: impl FnOnce(&mut Linker<usize>) -> Result<()>,
) -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;

    let mut store = Store::new(&engine, 0);
    let component = Component::new(&engine, wat)?;

    let mut linker = Linker::<usize>::new(&engine);
    linker
        .root()
        .func_wrap("record", |mut s: StoreContextMut<'_, usize>, (): ()| {
            // Keep track of the maximum size of the table in
            // concurrent_state.
            let cur = s.concurrent_state_table_size();
            let max = s.data_mut();
            *max = (*max).max(cur);
            Ok(())
        })?;
    define_host(&mut linker)?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    func.call_async(&mut store, ()).await?;

    // First-level assertion: nothing should remain after the guest has exited.
    store.assert_concurrent_state_empty();

    // Second-level assertion: state should be incrementally cleaned up along
    // the way as the guest runs. Things shouldn't leak until the guest exits
    // at the end, for example.
    assert!(
        *store.data() < max_table_size,
        "the store peaked at over {max_table_size} items in the concurrent \
         table which indicates that something isn't getting cleaned up \
         between calls"
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn sync_lower_async_host_does_not_leak() -> Result<()> {
    assert_concurrent_state_does_not_leak(
        r#"(component
            (import "f" (func $f async))
            (import "record" (func $record))

            (core module $m
                (import "" "f" (func $f))
                (import "" "record" (func $record))
                (func (export "run")
                    (local $c i32)

                    ;; call the host 100 times
                    loop
                      call $f

                      ;; track the max table size
                      call $record

                      (local.tee $c (i32.add (local.get $c) (i32.const 1)))
                      i32.const 100
                      i32.ne
                      if br 1 end
                    end
                )
            )
            (core func $f (canon lower (func $f) ))
            (core func $record (canon lower (func $record)))
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "f" (func $f))
                    (export "record" (func $record))
                ))
            ))

            (func (export "run") async
                (canon lift (core func $i "run")))


        )"#,
        100,
        |linker| {
            linker.root().func_wrap_concurrent("f", |_, (): ()| {
                Box::pin(async move {
                    // Ensure that this doesn't hit the fast path of "ready
                    // on first poll"
                    for _ in 0..5 {
                        tokio::task::yield_now().await;
                    }
                    Ok(())
                })
            })?;
            Ok(())
        },
    )
    .await
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn async_lower_guest_immediate_return_does_not_leak() -> Result<()> {
    assert_concurrent_state_does_not_leak(
        r#"(component
            (import "record" (func $record))

            (component $callee
                (core module $m
                    (func (export "f"))
                )
                (core instance $i (instantiate $m))

                (func (export "f") async
                    (canon lift (core func $i "f")))
            )
            (instance $callee (instantiate $callee))

            (component $caller
                (import "f" (func $f async))
                (import "record" (func $record))

                (core module $m
                    (import "" "f" (func $f (result i32)))
                    (import "" "record" (func $record))
                    (func (export "run")
                        (local $c i32)

                        ;; call the callee 100 times
                        loop
                          ;; the callee returns immediately, so the result
                          ;; is RETURNED with no subtask handle
                          call $f
                          i32.const 2 ;; RETURNED
                          i32.ne
                          if unreachable end

                          ;; track the max table size
                          call $record

                          (local.tee $c (i32.add (local.get $c) (i32.const 1)))
                          i32.const 100
                          i32.ne
                          if br 1 end
                        end
                    )
                )
                (core func $f (canon lower (func $f) async))
                (core func $record (canon lower (func $record)))
                (core instance $i (instantiate $m
                    (with "" (instance
                        (export "f" (func $f))
                        (export "record" (func $record))
                    ))
                ))

                (func (export "run") async
                    (canon lift (core func $i "run")))
            )
            (instance $caller (instantiate $caller
                (with "f" (func $callee "f"))
                (with "record" (func $record))
            ))

            (export "run" (func $caller "run"))
        )"#,
        100,
        |_| Ok(()),
    )
    .await
}

/// Regression test: `stream.cancel-read` with `async` option must not corrupt
/// the read state when it returns BLOCKED.
///
/// Bug: cancel_read/cancel_write unconditionally transitioned the read/write
/// state from GuestReady to Open after the cancel, even when the cancel
/// returned BLOCKED. This destroyed the buffer address/count info, causing
/// an error when the host later tried to access the stream state.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn stream_cancel_read_async_does_not_corrupt_state() -> Result<()> {
    _ = env_logger::try_init();

    let mut config = Config::new();
    config.wasm_component_model_async(true);
    config.wasm_component_model_more_async_builtins(true);
    config.wasm_component_model_async_stackful(true);
    let engine = Engine::new(&config)?;

    let component = Component::new(
        &engine,
        r#"
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.cancel-read" (func $stream.cancel-read (param i32) (result i32)))
    (import "" "stream.drop-readable" (func $stream.drop-readable (param i32)))
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
    (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
    (memory (export "memory") 1)

    (func (export "run") (param $sr i32)
      (local $cancel_result i32)
      (local $ws i32)

      ;; Async read into buffer at 0x100, length 4.
      ;; Should return BLOCKED (-1) since the host producer never writes.
      (call $stream.read (local.get $sr) (i32.const 0x100) (i32.const 4))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      ;; Async cancel-read. The host write end is HostReady, so this returns
      ;; BLOCKED. Bug: the cancel unconditionally transitions GuestReady -> Open,
      ;; destroying the buffer info.
      (local.set $cancel_result (call $stream.cancel-read (local.get $sr)))

      ;; If cancel returned BLOCKED (-1), wait for the cancel to complete.
      ;; This is where the bug manifests: when the host processes the cancel,
      ;; it accesses the read state which was corrupted from GuestReady to Open.
      (if (i32.eq (local.get $cancel_result) (i32.const -1))
        (then
          (local.set $ws (call $waitable-set.new))
          (call $waitable.join (local.get $sr) (local.get $ws))
          ;; Wait for the stream event (cancel completion). Event buffer at 0x200.
          (drop (call $waitable-set.wait (local.get $ws) (i32.const 0x200)))
          ;; Unjoin stream from waitable-set (join to 0 = unjoin)
          (call $waitable.join (local.get $sr) (i32.const 0))
          (call $waitable-set.drop (local.get $ws))
        )
      )

      ;; Drop the stream
      (call $stream.drop-readable (local.get $sr))
    )
  )

  (type $s (stream u8))
  (core func $stream.read (canon stream.read $s async (memory (core memory $libc "memory"))))
  (core func $stream.cancel-read (canon stream.cancel-read $s async))
  (core func $stream.drop-readable (canon stream.drop-readable $s))
  (canon waitable.join (core func $waitable.join))
  (canon waitable-set.new (core func $waitable-set.new))
  (canon waitable-set.wait (memory (core memory $libc "memory")) (core func $waitable-set.wait))
  (canon waitable-set.drop (core func $waitable-set.drop))

  (core instance $i (instantiate $m
    (with "" (instance
      (export "stream.read" (func $stream.read))
      (export "stream.cancel-read" (func $stream.cancel-read))
      (export "stream.drop-readable" (func $stream.drop-readable))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "waitable-set.drop" (func $waitable-set.drop))
    ))
  ))

  (func (export "run") async (param "s" (stream u8))
    (canon lift
      (core func $i "run")
      (memory (core memory $libc "memory"))
    )
  )
)
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let func = instance.get_typed_func::<(StreamReader<u8>,), ()>(&mut store, "run")?;

    // Create a host-side stream that never produces data (always Pending).
    // When cancel is requested (finish=true), it acknowledges the cancellation.
    let reader = StreamReader::new(&mut store, NeverWriteStreamProducer)?;
    func.call_async(&mut store, (reader,)).await?;

    return Ok(());

    struct NeverWriteStreamProducer;

    impl StreamProducer<()> for NeverWriteStreamProducer {
        type Item = u8;
        type Buffer = Option<u8>;

        fn poll_produce<'a>(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _store: StoreContextMut<'a, ()>,
            _destination: Destination<'a, Self::Item, Self::Buffer>,
            finish: bool,
        ) -> Poll<Result<StreamResult>> {
            if finish {
                // Cancel requested — acknowledge it.
                Poll::Ready(Ok(StreamResult::Cancelled))
            } else {
                // Never produce data.
                Poll::Pending
            }
        }
    }
}

/// Regression test: multiple threads may concurrently make a synchronous
/// call into the same async host function without corrupting state.
///
/// Bug: waitable sets for host calls used to be shared across all threads, so if two threads
/// called a sync-lowered async host function concurrently, the waitable set state got overwritten.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn concurrent_sync_calls_to_async_host() -> Result<()> {
    _ = env_logger::try_init();

    let mut config = Config::new();
    config.wasm_component_model_async(true);
    config.wasm_component_model_more_async_builtins(true);
    config.wasm_component_model_async_stackful(true);
    config.wasm_component_model_threading(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, 0);

    let component = Component::new(
        &engine,
        r#"(component
            (import "await-three-calls" (func $await-three-calls async))

            (core module $libc
                (table (export "__indirect_function_table") 1 funcref))

            (core module $m
                (import "" "await-three-calls" (func $await-three-calls))
                (import "" "thread.new-indirect" (func $thread-new-indirect (param i32 i32) (result i32)))
                (import "" "thread.resume-later" (func $thread-resume-later (param i32)))
                (import "libc" "__indirect_function_table" (table $indirect-function-table 1 funcref))

                (func (export "run")
                    (call $thread-new-indirect (i32.const 0) (i32.const 0))
                    (call $thread-resume-later)
                    (call $thread-new-indirect (i32.const 0) (i32.const 0))
                    (call $thread-resume-later)
                    (call $await-three-calls)
                )
                (func $thread-entry (param i32)
                    (call $await-three-calls)
                )
                (elem (table $indirect-function-table) (i32.const 0) func $thread-entry)
            )
            ;; Instantiate the libc module to get the table
            (core instance $libc (instantiate $libc))
            ;; Get access to `thread.new-indirect` that uses the table from libc
            (core type $start-func-ty (func (param i32)))
            (alias core export $libc "__indirect_function_table" (core table $indirect-function-table))
            (core func $thread-new-indirect
                (canon thread.new-indirect $start-func-ty (core table $indirect-function-table)))
            (core func $thread-resume-later (canon thread.resume-later))

            (core func $await-three-calls (canon lower (func $await-three-calls) ))
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "await-three-calls" (func $await-three-calls))
                    (export "thread.new-indirect" (func $thread-new-indirect))
                    (export "thread.resume-later" (func $thread-resume-later))
                ))
                (with "libc" (instance $libc))
            ))
            (func (export "run") async
                (canon lift (core func $i "run")))
        )"#,
    )?;

    let mut linker = Linker::<i32>::new(&engine);
    linker
        .root()
        .func_wrap_concurrent("await-three-calls", |accessor, (): ()| {
            Box::pin(async move {
                accessor.with(|mut s| {
                    *s.data_mut() += 1;
                });
                while accessor.with(|mut s| *s.data_mut()) < 3 {
                    tokio::task::yield_now().await;
                }
                Ok(())
            })
        })?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    func.call_async(&mut store, ()).await?;

    store.assert_concurrent_state_empty();

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn bytes_stream_producer() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;

    let component = Component::new(
        &engine,
        r#"
        (component
            (core module $libc (memory (export "mem") 1))
            (core instance $libc (instantiate $libc))
            (core module $m
                (import "" "mem" (memory 1))
                (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))

                (func (export "read") (param i32 i32) (result i32)
                    (call $stream.read (local.get 0) (i32.const 0) (local.get 1))
                )
            )
            (type $s (stream u8))
            (core func $stream.read (canon stream.read $s async (memory (core memory $libc "mem"))))
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "mem" (memory $libc "mem"))
                    (export "stream.read" (func $stream.read))
                ))
            ))
            (func (export "read") (param "s" (stream u8)) (param "l" u32) (result u32)
                (canon lift (core func $i "read")))
        )
    "#,
    )?;

    let linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let func = instance.get_typed_func::<(StreamReader<u8>, u32), (u32,)>(&mut store, "read")?;

    // read less than the capacity
    let reader = StreamReader::new(&mut store, bytes::Bytes::from_static(b"hello"))?;
    assert_eq!(
        func.call_async(&mut store, (reader, 1)).await?,
        ((1 << 4) | 0,),
    );

    // read more than the capacity
    let reader = StreamReader::new(&mut store, bytes::Bytes::from_static(b"hello"))?;
    assert_eq!(
        func.call_async(&mut store, (reader, 100)).await?,
        ((5 << 4) | 1,),
    );

    Ok(())
}

#[derive(Copy, Clone)]
enum TaskGroupHookTestVariant {
    WithDrop,
    WithoutDrop,
    Trap,
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn task_group_hook_with_drop() -> Result<()> {
    test_task_group_hook(TaskGroupHookTestVariant::WithDrop).await
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn task_group_hook_without_drop() -> Result<()> {
    test_task_group_hook(TaskGroupHookTestVariant::WithoutDrop).await
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn task_group_hook_trap() -> Result<()> {
    test_task_group_hook(TaskGroupHookTestVariant::Trap).await
}

/// Verify that all the expected `TaskGroupHook` events are delivered,
/// regardless of whether the guest tasks exit normally, leak subtasks, or trap.
async fn test_task_group_hook(variant: TaskGroupHookTestVariant) -> Result<()> {
    _ = env_logger::try_init();
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;

    let call_drop = if let TaskGroupHookTestVariant::WithDrop = variant {
        "(call $subtask.drop (local.get $waitable))"
    } else {
        ""
    };

    let component = Component::new(
        &engine,
        format!(
            r#"
        (component
            (import "a" (func $a async))
            (core func $a (canon lower (func $a) async))

            (core module $a
                (import "" "a" (func $a (result i32)))
                (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
                (import "" "waitable.join" (func $waitable.join (param i32 i32)))
                (import "" "subtask.drop" (func $subtask.drop (param i32)))
                (import "" "task.return" (func $task.return))
                (func (export "a") (result i32)
                    (local $ret i32)
                    (local $subtask i32)
                    (local $set i32)

                    (local.set $ret (call $a))
                    (if (i32.ne (i32.const 1 (; STARTED ;)) (i32.and (local.get $ret) (i32.const 0xf)))
                        (then unreachable))
                    (local.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))
                    (local.set $set (call $waitable-set.new))
                    (call $waitable.join (local.get $subtask) (local.get $set))
                    (i32.or (i32.const 2 (; WAIT ;)) (i32.shl (local.get $set) (i32.const 4)))
                )

                (func (export "a-callback")
                      (param $event_code i32)
                      (param $waitable i32)
                      (param $payload i32)
                      (result i32)
                    (if (i32.ne (local.get $event_code) (i32.const 1 (; SUBTASK ;)))
                        (then unreachable))
                    (if (i32.ne (local.get $payload) (i32.const 2 (; RETURNED ;)))
                        (then unreachable))
                    (call $waitable.join (local.get $waitable) (i32.const 0))
                    {call_drop}
                    (call $task.return)
                    (i32.const 0 (; EXIT ;))
                )
            )
            (canon waitable-set.new (core func $waitable-set.new))
            (canon waitable.join (core func $waitable.join))
            (canon subtask.drop (core func $subtask.drop))
            (canon task.return (core func $task.return))
            (core instance $a (instantiate $a
                (with "" (instance
                    (export "a" (func $a))
                    (export "waitable-set.new" (func $waitable-set.new))
                    (export "waitable.join" (func $waitable.join))
                    (export "subtask.drop" (func $subtask.drop))
                    (export "task.return" (func $task.return))
                ))
            ))
            (func (export "a") async (canon lift (core func $a "a") async (callback (core func $a "a-callback"))))
        )
    "#
        ),
    )?;

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    enum Event {
        Start,
        PollHostCall,
        Finish,
    }

    #[derive(Default)]
    struct HookInner {
        events: Vec<(Event, usize)>,
        next_id: usize,
        ids: HashMap<TaskGroupId, usize>,
        entered: Option<usize>,
        call_count: usize,
    }

    #[derive(Clone)]
    struct Hook(Arc<Mutex<HookInner>>);

    impl TaskGroupHook for Hook {
        fn handle_start(&mut self, group: TaskGroupId) -> Result<()> {
            let mut inner = self.0.try_lock().unwrap();
            let id = inner.next_id;
            inner.next_id += 1;
            // This `group` should not be equal to any other
            // started-and-not-yet-finished group:
            assert!(inner.ids.insert(group, id).is_none());
            inner.events.push((Event::Start, id));
            Ok(())
        }

        fn handle_enter(&mut self, group: TaskGroupId) -> Result<()> {
            let mut inner = self.0.try_lock().unwrap();
            let id = *inner.ids.get(&group).unwrap();
            // We should not have entered-but-not-yet-exited this or any other
            // group:
            assert!(inner.entered.replace(id).is_none());
            Ok(())
        }

        fn handle_exit(&mut self, group: TaskGroupId) -> Result<()> {
            let mut inner = self.0.try_lock().unwrap();
            let id = *inner.ids.get(&group).unwrap();
            // We should have entered-but-not-yet-exited this group:
            assert_eq!(Some(id), inner.entered);
            inner.entered = None;
            Ok(())
        }

        fn handle_finish(&mut self, group: TaskGroupId) -> Result<()> {
            let mut inner = self.0.try_lock().unwrap();
            let id = inner.ids.remove(&group).unwrap();
            inner.events.push((Event::Finish, id));
            Ok(())
        }
    }

    let yield_count = 5;
    let call_count = 3;

    let hook = Hook(Arc::new(Mutex::new(HookInner::default())));

    let mut linker = Linker::new(&engine);
    linker.root().func_wrap_concurrent("a", {
        let hook = hook.clone();
        move |_, ()| {
            let hook = hook.clone();
            Box::pin(async move {
                let mut previous_id = None;
                let mut on_poll = || {
                    let mut inner = hook.0.try_lock().unwrap();
                    // We should only ever be polled after a `handle_enter` and
                    // before a `handle_exit` event:
                    let id = inner.entered.unwrap();
                    inner.events.push((Event::PollHostCall, id));

                    // Should see the same task group on every poll:
                    if let Some(previous_id) = previous_id {
                        assert_eq!(previous_id, id);
                    } else {
                        previous_id = Some(id);
                    }

                    id
                };

                on_poll();
                for _ in 0..yield_count {
                    tokio::task::yield_now().await;
                    on_poll();
                }

                let mut inner = hook.0.try_lock().unwrap();
                inner.call_count += 1;
                if let TaskGroupHookTestVariant::Trap = variant
                    && inner.call_count == call_count
                {
                    Err(wasmtime::format_err!("yikes"))
                } else {
                    Ok(())
                }
            })
        }
    })?;

    {
        let mut store = Store::new(&engine, ());
        let instance = linker.instantiate_async(&mut store, &component).await?;
        let func = instance.get_typed_func::<(), ()>(&mut store, "a")?;

        store.task_group_hook(hook.clone());

        // Call the guest export `call_count` times concurrently, recording the
        // events in `hook`:
        let result = store
            .run_concurrent(async |store| {
                iter::repeat_with(|| func.call_concurrent(store, ()))
                    .take(call_count)
                    .collect::<FuturesUnordered<_>>()
                    .try_collect::<()>()
                    .await
            })
            .await;

        match (result, variant) {
            (
                Ok(result),
                TaskGroupHookTestVariant::WithDrop | TaskGroupHookTestVariant::WithoutDrop,
            ) => result?,
            (Err(error), TaskGroupHookTestVariant::Trap) => {
                assert!(error.to_string().contains("yikes"), "{error}");
            }
            _ => unreachable!(),
        }
    }

    // Group the events by task group, preserving order:
    let mut map = HashMap::<_, Vec<_>>::new();
    for (event, id) in mem::take(&mut hook.0.try_lock().unwrap().events) {
        map.entry(id).or_default().push(event);
    }

    // Assert that the events start with a `Start`, followed by the expected
    // number of `PollHostCall`s, followed by a `Finish`:
    let expected = iter::once(Event::Start)
        .chain(iter::repeat(Event::PollHostCall).take(yield_count + 1))
        .chain(Some(Event::Finish))
        .collect::<Vec<_>>();

    for id in 0..call_count {
        assert_eq!(&map.get(&id).unwrap()[..], &expected[..])
    }

    assert_eq!(None, hook.0.try_lock().unwrap().entered);
    assert!(hook.0.try_lock().unwrap().ids.is_empty());

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn inter_component_stream_is_not_intra_component() -> Result<()> {
    let engine = Engine::default();

    let writer = Component::new(
        &engine,
        r#"
(component
  (core module $libc
    (memory (export "m") 1)
    (data (i32.const 0x100) "hello")
  )
  (core instance $libc (instantiate $libc))

  (type $s (stream string))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.write (canon stream.write $s async (memory (core memory $libc "m"))))

  (core module $m
    (import "" "m" (memory 1))
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))

    (global $w (mut i32) (i32.const 0))

    (func (export "mk") (result i32)
      (local $tmp i64)
      (local.set $tmp (call $stream.new))
      (global.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
      (i32.wrap_i64 (local.get $tmp))
    )

    (func (export "write")
      ;; store ptr/len at 0x10
      (i32.store (i32.const 0x10) (i32.const 0x100))
      (i32.store (i32.const 0x14) (i32.const 5))

      (call $stream.write (global.get $w) (i32.const 0x10) (i32.const 1))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "m" (memory $libc "m"))
      (export "stream.new" (func $stream.new))
      (export "stream.write" (func $stream.write))
    ))
  ))

  (func (export "mk") (result $s) (canon lift (core func $i "mk")))
  (func (export "write") (canon lift (core func $i "write")))
)
        "#,
    )?;

    let reader = Component::new(
        &engine,
        r#"
(component
  (core module $libc
    (memory (export "m") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (i32.const 0x200)
    )
  )
  (core instance $libc (instantiate $libc))

  (type $s (stream string))
  (core func $stream.read
    (canon stream.read $s async
      (memory (core memory $libc "m"))
      (realloc (core func $libc "realloc"))))

  (core module $m
    (import "" "m" (memory 1))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))

    (func (export "read") (param $r i32) (result i32)
      (call $stream.read (local.get $r) (i32.const 0x10) (i32.const 1))
      i32.const 0x10 ;; COMPLETED | (1 << 4)
      i32.ne
      if unreachable end

      i32.const 0x10
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "m" (memory $libc "m"))
      (export "stream.read" (func $stream.read))
    ))
  ))

  (func (export "read") (param "s" $s) (result string)
    (canon lift (core func $i "read") (memory (core memory $libc "m"))))
)
        "#,
    )?;

    let linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ());
    let writer = linker.instantiate(&mut store, &writer)?;
    let reader = linker.instantiate(&mut store, &reader)?;

    let mk = writer.get_typed_func::<(), (StreamReader<String>,)>(&mut store, "mk")?;
    let write = writer.get_typed_func::<(), ()>(&mut store, "write")?;
    let read = reader.get_typed_func::<(StreamReader<String>,), (String,)>(&mut store, "read")?;

    let (stream,) = mk.call(&mut store, ())?;
    write.call(&mut store, ())?;
    assert_eq!(read.call(&mut store, (stream,))?, ("hello".to_string(),));

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn join_handle_used_from_other_threads() -> Result<()> {
    use std::task::Waker;
    use std::time::Duration;

    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    // Spawn some tasks and hand each `JoinHandle` to its own thread, which
    // polls, aborts, and drops it while the event loop (below) is driving and
    // dropping the same tasks. The two race, so this doesn't reproduce the
    // underlying lock-contention bug on every run.
    let remaining = Arc::new(AtomicUsize::new(4));
    let mut threads = Vec::new();
    for _ in 0..4 {
        let mut handle = store.spawn(async |_| std::future::pending::<Result<()>>().await)?;
        let remaining = remaining.clone();
        threads.push(std::thread::spawn(move || {
            let mut cx = Context::from_waker(Waker::noop());
            assert!(Pin::new(&mut handle).poll(&mut cx).is_pending());
            handle.abort();
            // Wait for the event loop to drop the aborted task, racing it.
            while Pin::new(&mut handle).poll(&mut cx).is_pending() {
                std::hint::spin_loop();
            }
            remaining.fetch_sub(1, Ordering::Relaxed);
        }));
    }

    // Drive the event loop until every thread is done, bounded by a timeout so
    // a hang fails the test rather than blocking forever.
    tokio::time::timeout(
        Duration::from_secs(60),
        store.run_concurrent(async |_| {
            while remaining.load(Ordering::Relaxed) != 0 {
                tokio::task::yield_now().await;
            }
        }),
    )
    .await
    .expect("timed out driving the event loop")?;

    for thread in threads {
        thread.join().unwrap();
    }
    Ok(())
}

/// Yields to the executor `n` times.
async fn yield_times(n: usize) {
    for _ in 0..n {
        let mut yielded = false;
        std::future::poll_fn(|cx| {
            if yielded {
                Poll::Ready(())
            } else {
                yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await
    }
}

/// Starts a guest task which yields forever (calling the host's `tick` each
/// time it runs) concurrently with one which fails, then checks that once the
/// store is poisoned the event loop refuses to run any further guest code.
async fn no_guest_code_after_trap(boom_calls_failing_host: bool) -> Result<()> {
    let boom = if boom_calls_failing_host {
        "(call $fail) unreachable"
    } else {
        "unreachable"
    };
    let wat = format!(
        r#"
(component
  (import "tick" (func $tick))
  (import "fail" (func $fail))
  (component $Spin
    (import "tick" (func $tick))
    (core func $tick (canon lower (func $tick)))
    (core module $m
      (import "" "tick" (func $tick))
      (func (export "run") (result i32) (call $tick) (i32.const 1)) ;; YIELD
      (func (export "cb") (param i32 i32 i32) (result i32)
        (call $tick) (i32.const 1))) ;; YIELD
    (core instance $i (instantiate $m (with "" (instance (export "tick" (func $tick))))))
    (func (export "run") async
      (canon lift (core func $i "run") async (callback (core func $i "cb")))))
  (component $Boom
    (import "fail" (func $fail))
    (core func $fail (canon lower (func $fail)))
    (core module $m
      (import "" "fail" (func $fail))
      (func (export "run") (result i32) {boom})
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance (export "fail" (func $fail))))))
    (func (export "run") async
      (canon lift (core func $i "run") async (callback (core func $i "cb")))))
  (instance $s (instantiate $Spin (with "tick" (func $tick))))
  (instance $b (instantiate $Boom (with "fail" (func $fail))))
  (export "spin" (func $s "run"))
  (export "boom" (func $b "run")))
"#
    );

    let engine = Engine::default();
    let component = Component::new(&engine, wat)?;
    let mut linker = Linker::<u32>::new(&engine);
    linker
        .root()
        .func_wrap("tick", |mut store: StoreContextMut<'_, u32>, (): ()| {
            *store.data_mut() += 1;
            Ok(())
        })?;
    linker.root().func_wrap(
        "fail",
        |_: StoreContextMut<'_, u32>, (): ()| -> Result<()> { wasmtime::bail!("host error") },
    )?;
    let mut store = Store::new(&engine, 0);
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let spin = instance.get_typed_func::<(), ()>(&mut store, "spin")?;
    let boom = instance.get_typed_func::<(), ()>(&mut store, "boom")?;

    let result = store
        .run_concurrent(async |a| {
            let s = spin.call_concurrent(a, ());
            let b = boom.call_concurrent(a, ());
            tokio::join!(s, b)
        })
        .await;
    assert!(result.is_err());
    let ticks = *store.data();

    // Neither running the event loop for a host-only future nor entering the
    // guest may run any more guest code.
    let result = store.run_concurrent(async |_| yield_times(10).await).await;
    assert_eq!(*store.data(), ticks);
    assert_eq!(
        result.unwrap_err().downcast::<Trap>()?,
        Trap::CannotEnterComponent,
    );
    let result = store
        .run_concurrent(async |a| spin.call_concurrent(a, ()).await)
        .await
        .and_then(|r| r);
    assert_eq!(*store.data(), ticks);
    assert_eq!(
        result.unwrap_err().downcast::<Trap>()?,
        Trap::CannotEnterComponent,
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn no_guest_code_after_trap_in_run_concurrent() -> Result<()> {
    no_guest_code_after_trap(false).await
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn no_guest_code_after_host_error_in_run_concurrent() -> Result<()> {
    no_guest_code_after_trap(true).await
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn event_loop_after_realloc_trap_during_host_stream_write() -> Result<()> {
    let engine = Engine::default();
    let component = Component::new(
        &engine,
        r#"
(component
  (type $st (stream string))
  (core module $libc
    (memory (export "mem") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) unreachable))
  (core instance $libc (instantiate $libc))
  (core func $stream.read (canon stream.read $st async (memory (core memory $libc "mem"))
    (realloc (core func $libc "realloc"))))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $task.return (canon task.return (result u32)))
  (core module $m
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))
    (import "" "task.return" (func $task.return (param i32)))
    (func (export "consume") (param $s i32) (result i32)
      (local $set i32)
      (drop (call $stream.read (local.get $s) (i32.const 0) (i32.const 4)))
      (local.set $set (call $waitable-set.new))
      (call $waitable.join (local.get $s) (local.get $set))
      ;; WAIT on $set
      (i32.or (i32.const 2) (i32.shl (local.get $set) (i32.const 4))))
    (func (export "cb") (param i32 i32 i32) (result i32)
      (call $task.return (i32.const 0))
      (i32.const 0))) ;; EXIT
  (core instance $i (instantiate $m (with "" (instance
    (export "stream.read" (func $stream.read))
    (export "waitable-set.new" (func $waitable-set.new))
    (export "waitable.join" (func $waitable.join))
    (export "task.return" (func $task.return))))))
  (func (export "consume") async (param "s" $st) (result u32)
    (canon lift (core func $i "consume") async (callback (core func $i "cb"))))
)
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let consume =
        instance.get_typed_func::<(StreamReader<String>,), (u32,)>(&mut store, "consume")?;

    let stream = StreamReader::new(&mut store, vec!["hello".to_string()])?;
    let err = consume.call_async(&mut store, (stream,)).await.unwrap_err();
    assert_eq!(
        err.downcast_ref::<Trap>(),
        Some(&Trap::UnreachableCodeReached)
    );

    let result = store.run_concurrent(async |_| yield_times(5).await).await;
    assert_eq!(
        result.unwrap_err().downcast::<Trap>()?,
        Trap::CannotEnterComponent,
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn start_function_trap_reported_faithfully() -> Result<()> {
    let engine = Engine::default();
    let component = Component::new(
        &engine,
        r#"
(component
  (core module $m (func $s unreachable) (start $s))
  (core instance (instantiate $m)))
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    let err = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await
        .unwrap_err();
    assert_eq!(err.downcast::<Trap>()?, Trap::UnreachableCodeReached);
    Ok(())
}

const DROP_MID_FLIGHT_WAT: &str = r#"
(component
  (core func $task.return (canon task.return))
  (core module $m
    (import "" "task.return" (func $tr))
    (memory 1)
    (func (export "run") (result i32) (call $tr) (i32.const 0 (; EXIT ;)))
    (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
  (core instance $i (instantiate $m (with "" (instance (export "task.return" (func $task.return))))))
  (func (export "run") async (canon lift (core func $i "run") async (callback (core func $i "cb"))))
)
"#;

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn drop_instantiate_async_blocked_in_limiter() -> Result<()> {
    struct Limiter {
        block: bool,
    }

    #[async_trait::async_trait]
    impl wasmtime::ResourceLimiterAsync for Limiter {
        async fn memory_growing(&mut self, _: usize, _: usize, _: Option<usize>) -> Result<bool> {
            if self.block {
                std::future::pending::<()>().await;
            }
            Ok(true)
        }
        async fn table_growing(&mut self, _: usize, _: usize, _: Option<usize>) -> Result<bool> {
            Ok(true)
        }
    }

    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, DROP_MID_FLIGHT_WAT)?;
    let linker = Linker::new(&engine);
    let mut store = Store::new(&engine, Limiter { block: true });
    store.limiter_async(|l| l);

    let future = Box::pin(linker.instantiate_async(&mut store, &component));
    assert!(PollOnce::new(future).await.is_err());

    // Using the store afterwards shouldn't panic...
    store.data_mut().block = false;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    store
        .run_concurrent(async |a| run.call_concurrent(a, ()).await)
        .await??;
    run.call_async(&mut store, ()).await?;

    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
enum BlockingCallHook {
    Calling,
    Returning,
}

struct BlockingCallHookHandler(Arc<Mutex<Option<BlockingCallHook>>>);

#[async_trait::async_trait]
impl<T: Send> wasmtime::CallHookHandler<T> for BlockingCallHookHandler {
    async fn handle_call_event(
        &self,
        _: wasmtime::StoreHookState<'_, T>,
        ch: wasmtime::CallHook,
    ) -> Result<()> {
        let which = match ch {
            wasmtime::CallHook::CallingWasm => BlockingCallHook::Calling,
            wasmtime::CallHook::ReturningFromWasm => BlockingCallHook::Returning,
            _ => return Ok(()),
        };
        if *self.0.lock().unwrap() == Some(which) {
            std::future::pending::<()>().await;
        }
        Ok(())
    }
}

// Dropping `run_concurrent` while a guest call is blocked in an async call
// hook abandons that call partway through, so the store must be poisoned
// rather than left with a stale current guest task.
async fn drop_run_concurrent_blocked_in_call_hook(which: BlockingCallHook) -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, DROP_MID_FLIGHT_WAT)?;
    let mut store = Store::new(&engine, ());
    let block = Arc::new(Mutex::new(None));
    store.call_hook_async(BlockingCallHookHandler(block.clone()));
    let linker = Linker::new(&engine);
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    *block.lock().unwrap() = Some(which);
    let future = Box::pin(store.run_concurrent(async |a| run.call_concurrent(a, ()).await));
    assert!(PollOnce::new(future).await.is_err());
    *block.lock().unwrap() = None;

    let err = store
        .run_concurrent(async |a| run.call_concurrent(a, ()).await)
        .await?
        .unwrap_err();
    assert_eq!(
        err.downcast_ref::<Trap>(),
        Some(&Trap::CannotEnterComponent)
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn drop_run_concurrent_blocked_in_calling_hook() -> Result<()> {
    drop_run_concurrent_blocked_in_call_hook(BlockingCallHook::Calling).await
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn drop_run_concurrent_blocked_in_returning_hook() -> Result<()> {
    drop_run_concurrent_blocked_in_call_hook(BlockingCallHook::Returning).await
}

// Dropping `instantiate_async` while a core module's start function is blocked
// in an async call hook abandons the start function partway through, so the
// store must be poisoned.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn drop_instantiate_async_blocked_in_start_call_hook() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let with_start = Component::new(
        &engine,
        r#"
(component
  (core module $m
    (func $start)
    (start $start))
  (core instance $i (instantiate $m))
)
        "#,
    )?;
    let component = Component::new(&engine, DROP_MID_FLIGHT_WAT)?;
    let mut store = Store::new(&engine, ());
    let block = Arc::new(Mutex::new(Some(BlockingCallHook::Calling)));
    store.call_hook_async(BlockingCallHookHandler(block.clone()));
    let linker = Linker::new(&engine);

    let future = Box::pin(linker.instantiate_async(&mut store, &with_start));
    assert!(PollOnce::new(future).await.is_err());
    *block.lock().unwrap() = None;

    let instance = linker.instantiate_async(&mut store, &component).await?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    let err = store
        .run_concurrent(async |a| run.call_concurrent(a, ()).await)
        .await?
        .unwrap_err();
    assert_eq!(
        err.downcast_ref::<Trap>(),
        Some(&Trap::CannotEnterComponent)
    );

    Ok(())
}

// A call hook failing while `ResourceAny::resource_drop_async` enters the
// guest to run a destructor must poison the store.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn resource_drop_call_hook_error() -> Result<()> {
    let engine = Engine::default();
    let component = Component::new(
        &engine,
        r#"
(component
  (core module $d (func (export "dtor") (param i32)))
  (core instance $d (instantiate $d))
  (type $r (resource (rep i32) (dtor (core func $d "dtor"))))
  (core func $new (canon resource.new $r))
  (core module $m
    (import "" "new" (func $new (param i32) (result i32)))
    (func (export "mk") (result i32) (call $new (i32.const 7)))
    (func (export "nop")))
  (core instance $i (instantiate $m
    (with "" (instance (export "new" (func $new))))))
  (export $r2 "r" (type $r))
  (func (export "mk") (result (own $r2)) (canon lift (core func $i "mk")))
  (func (export "nop") (canon lift (core func $i "nop")))
)
        "#,
    )?;
    let mut store = Store::new(&engine, false);
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let mk = instance.get_typed_func::<(), (ResourceAny,)>(&mut store, "mk")?;
    let nop = instance.get_typed_func::<(), ()>(&mut store, "nop")?;
    let (r,) = mk.call_async(&mut store, ()).await?;

    store.call_hook(|store, hook| {
        if *store.data() && matches!(hook, wasmtime::CallHook::CallingWasm) {
            wasmtime::bail!("hook error");
        }
        Ok(())
    });
    *store.data_mut() = true;
    let err = r.resource_drop_async(&mut store).await.unwrap_err();
    assert_eq!(err.to_string(), "hook error");
    *store.data_mut() = false;

    let err = store
        .run_concurrent(async |a| nop.call_concurrent(a, ()).await)
        .await
        .and_then(|r| r)
        .unwrap_err();
    assert_eq!(
        err.downcast_ref::<Trap>(),
        Some(&Trap::CannotEnterComponent),
        "unexpected error: {err:?}"
    );
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn cancel_not_started() -> Result<()> {
    let engine = Engine::default();
    let component = Component::new(
        &engine,
        r#"
        (component
            (import "a" (func $a))
            (core func $a (canon lower (func $a)))

            (core module $a
                (import "" "a" (func $a))
                (import "" "task.return" (func $task.return))
                (func (export "a") (result i32)
                    (call $a)
                    (call $task.return)
                    (i32.const 0 (; EXIT ;))
                )

                (func (export "a-callback") (param i32 i32 i32) (result i32)
                    unreachable
                )
            )
            (canon task.return (core func $task.return))
            (core instance $a (instantiate $a
                (with "" (instance
                    (export "a" (func $a))
                    (export "task.return" (func $task.return))
                ))
            ))
            (func (export "a") async (canon lift (core func $a "a") async (callback (core func $a "a-callback"))))
        )
    "#,
    )?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker.root().func_wrap_async("a", move |_, ()| {
        Box::new(async move {
            for _ in 0..5 {
                tokio::task::yield_now().await;
            }

            Ok(())
        })
    })?;

    let instance = linker.instantiate_async(&mut store, &component).await?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "a")?;

    // Start a call, which will push a work item on the high-priority task
    // queue:
    func.start_call_concurrent(&mut store, ())?;

    // Start and poll an async call once, which will push another work item on
    // the high-priority task queue, then try to drain the queue.  However, the
    // above call will call our host function, which will yield back to us here.
    // Then we'll drop the future for the second call, which will not have
    // started because it's still waiting in the queue.
    //
    // Previously, that would have caused a panic in the runtime.
    let mut call = pin!(func.call_async(&mut store, ()));
    assert!(
        call.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(call);
    Ok(())
}

// Dropping a `call_async` future while the start of its task is queued in the
// event loop's run queue (rather than blocked in the instance's pending set)
// must cancel the task cleanly rather than panicking in the drop guard.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn drop_call_async_with_queued_start() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;

    let component = Component::new(
        &engine,
        r#"
(component
  (type $f (future))
  (core func $future.new (canon future.new $f))
  (core func $future.read (canon future.read $f async))
  (core func $future.write (canon future.write $f async))
  (core func $new (canon waitable-set.new))
  (core func $join (canon waitable.join))
  (core func $task-return (canon task.return))
  (core module $m
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
    (import "" "new" (func $new (result i32)))
    (import "" "join" (func $join (param i32 i32)))
    (import "" "task-return" (func $task-return))
    (func (export "a") (result i32)
      (local $t i64) (local $r i32) (local $w i32) (local $s i32)
      (call $task-return)
      (local.set $t (call $future.new))
      (local.set $r (i32.wrap_i64 (local.get $t)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $t) (i64.const 32))))
      (drop (call $future.read (local.get $r) (i32.const 0)))
      (local.set $s (call $new))
      (call $join (local.get $r) (local.get $s))
      (drop (call $future.write (local.get $w) (i32.const 0)))
      ;; WAIT on a set which already has a ready event, queueing a callback
      ;; invocation in the event loop.
      (i32.or (i32.const 2) (i32.shl (local.get $s) (i32.const 4))))
    (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
    (func (export "b") (result i32)
      (call $task-return)
      (i32.const 0)))
  (core instance $i (instantiate $m (with "" (instance
    (export "future.new" (func $future.new))
    (export "future.read" (func $future.read))
    (export "future.write" (func $future.write))
    (export "new" (func $new))
    (export "join" (func $join))
    (export "task-return" (func $task-return))))))
  (func (export "a") async (canon lift (core func $i "a") async (callback (core func $i "cb"))))
  (func (export "b") async (canon lift (core func $i "b") async (callback (core func $i "cb"))))
)
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let a = instance.get_typed_func::<(), ()>(&mut store, "a")?;
    let b = instance.get_typed_func::<(), ()>(&mut store, "b")?;
    a.call_async(&mut store, ()).await?;

    // The start of `b` is queued behind the pending callback for `a`, which
    // traps, so `b` never starts and its task is cancelled when the future
    // returned by `call_async` is dropped.
    let err = b.call_async(&mut store, ()).await.unwrap_err();
    assert_eq!(
        err.downcast_ref::<Trap>(),
        Some(&Trap::UnreachableCodeReached)
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn host_to_host_stream_buffer_type_mismatch() -> wasmtime::Result<()> {
    // A component that echoes a `stream<list<u8>>` back to the caller.
    const WAT: &str = r#"
(component
  (type $s (stream (list u8)))
  (core module $m
    (func (export "echo") (param i32) (result i32) local.get 0))
  (core instance $i (instantiate $m))
  (func (export "echo-stream") (param "s" $s) (result $s)
    (canon lift (core func $i "echo")))
)
"#;

    let engine = Engine::default();
    let component = Component::new(&engine, WAT)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;

    // Created as `Vec<u8>` items, read back as `WasmList<u8>` items.
    let echo = instance.get_typed_func::<(StreamReader<Vec<u8>>,), (StreamReader<WasmList<u8>>,)>(
        &mut store,
        "echo-stream",
    )?;
    let input = vec![1u8, 2, 3];
    let reader = StreamReader::new(&mut store, vec![input.clone()])?;
    let (reader,) = echo.call_async(&mut store, (reader,)).await?;

    struct Consumer(Arc<Mutex<Vec<WasmList<u8>>>>);

    impl StreamConsumer<()> for Consumer {
        type Item = WasmList<u8>;

        fn poll_consume(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            mut store: StoreContextMut<()>,
            mut source: Source<'_, WasmList<u8>>,
            _finish: bool,
        ) -> Poll<Result<StreamResult>> {
            let mut buffer = self.0.try_lock().unwrap();
            buffer.reserve(source.remaining(&mut store)?);
            source.read(store, buffer.deref_mut())?;
            Poll::Ready(Ok(StreamResult::Completed))
        }
    }

    let buffer = Arc::new(Mutex::new(Vec::new()));
    reader.pipe(&mut store, Consumer(buffer.clone()))?;

    let error = store
        .run_concurrent(async |_| {
            while buffer.try_lock().unwrap().is_empty() {
                tokio::task::yield_now().await
            }
        })
        .await
        .unwrap_err();

    assert!(
        error.to_string().contains("host stream item type mismatch"),
        "{error}"
    );

    Ok(())
}

/// Regression test for case which previously panicked when finishing a call
/// which was started before an intervening, failing call.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn call_between_start_and_finish() -> wasmtime::Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(
        &engine,
        r#"
(component
  (core func $thread.yield (canon thread.yield))
  (core func $task.return (canon task.return))
  (core module $m
    (import "" "thread.yield" (func $thread.yield (result i32)))
    (import "" "task.return" (func $task.return))
    ;; sync-typed export which yields
    (func (export "s") (drop (call $thread.yield)))
    ;; async-typed, callback-lifted export
    (func (export "f") (result i32) (call $task.return) (i32.const 1)) ;; YIELD
    (func (export "f-cb") (param i32 i32 i32) (result i32) (i32.const 0))
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "thread.yield" (func $thread.yield))
      (export "task.return" (func $task.return))))))
  (func (export "s") (canon lift (core func $i "s")))
  (func (export "f") async
    (canon lift (core func $i "f") async (callback (core func $i "f-cb"))))
)
"#,
    )?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &component)?;
    let s = instance.get_typed_func::<(), ()>(&mut store, "s")?;
    let f = instance.get_typed_func::<(), ()>(&mut store, "f")?;

    let err = store
        .run_concurrent(async |accessor| {
            // Queue a call to `f`; its start is now a queued work item for the
            // instance.
            let call = accessor.with(|s| f.start_call_concurrent(s, ()))?;
            // Synchronously call `s`, which yields. Since `s` is sync-typed and
            // there's a queued work item for its instance, Wasmtime promotes it
            // and tries to suspend, but we're not on a fiber.
            let _r = accessor.with(|mut st| s.call(&mut st, ()));
            // Now keep driving the event loop.
            f.finish_call_concurrent(accessor, call).await?;
            wasmtime::Result::<()>::Ok(())
        })
        .await
        .and_then(|r| r)
        .unwrap_err();

    assert_eq!(
        err.downcast_ref::<Trap>(),
        Some(&Trap::CannotEnterComponent),
        "unexpected error: {err:?}"
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn zero_length_reads_and_writes() -> Result<()> {
    let engine = Engine::default();
    let component = Component::new(
        &engine,
        r#"
        (component
            (core module $libc (memory (export "mem") 1))
            (core instance $libc (instantiate $libc))

            (core module $a
                (import "" "stream.new" (func $stream.new (result i64)))
                (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
                (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
                (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
                (import "" "waitable.join" (func $waitable.join (param i32 i32)))
                (import "" "task.return" (func $task.return (param i32)))

                (func (export "a") (param $stream i32) (result i32)
                    (local $set i32)
                    (local $tmp i64)
                    (local $stream-writer i32)
                    (local $stream-reader i32)
                    (local $ret i32)

                    (local.set $ret (call $stream.read (local.get $stream) (i32.const 0xFFFFFF) (i32.const 0)))
                    (if (i32.ne (local.get $ret) (i32.const 1 (; DROPPED ;))) (then unreachable))

                    (local.set $tmp (call $stream.new))
                    (local.set $stream-writer (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
                    (local.set $stream-reader (i32.wrap_i64 (local.get $tmp)))

                    (call $task.return (local.get $stream-reader))

                    (local.set $ret (call $stream.write (local.get $stream-writer) (i32.const 0xFFFFFF) (i32.const 0)))
                    (if (i32.ne (local.get $ret) (i32.const -1 (; BLOCKED ;))) (then unreachable))

                    (local.set $set (call $waitable-set.new))
                    (call $waitable.join (local.get $stream-writer) (local.get $set))
                    (i32.or (i32.const 2 (; WAIT ;)) (i32.shl (local.get $set) (i32.const 4)))
                )

                (func (export "a-callback") (param i32 i32 i32) (result i32)
                    (i32.const 0 (; EXIT ;))
                )
            )

            (type $stream (stream u8))
            (canon stream.new $stream (core func $stream.new))
            (canon stream.read $stream async (memory (core memory $libc "mem")) (core func $stream.read))
            (canon stream.write $stream async (memory (core memory $libc "mem")) (core func $stream.write))
            (canon waitable-set.new (core func $waitable-set.new))
            (canon waitable.join (core func $waitable.join))
            (canon task.return (result $stream) (core func $task.return))

            (core instance $a (instantiate $a
                (with "" (instance
                    (export "stream.new" (func $stream.new))
                    (export "stream.read" (func $stream.read))
                    (export "stream.write" (func $stream.write))
                    (export "waitable-set.new" (func $waitable-set.new))
                    (export "waitable.join" (func $waitable.join))
                    (export "task.return" (func $task.return))
                ))
            ))

            (func (export "a") async (param "stream" $stream) (result $stream)
                (canon lift (core func $a "a") async (callback (core func $a "a-callback")))
            )
        )
    "#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let func =
        instance.get_typed_func::<(StreamReader<u8>,), (StreamReader<u8>,)>(&mut store, "a")?;

    struct Producer;

    impl StreamProducer<()> for Producer {
        type Item = u8;
        type Buffer = Option<u8>;

        fn poll_produce<'a>(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            store: StoreContextMut<'a, ()>,
            destination: Destination<'a, u8, Option<u8>>,
            _finish: bool,
        ) -> Poll<Result<StreamResult>> {
            // This used to panic on zero-length reads with out-of-bounds
            // addresses:
            _ = destination.as_direct(store, 1).remaining();
            Poll::Ready(Ok(StreamResult::Dropped))
        }
    }

    struct Consumer;

    impl StreamConsumer<()> for Consumer {
        type Item = u8;

        fn poll_consume(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            store: StoreContextMut<()>,
            source: Source<'_, u8>,
            _finish: bool,
        ) -> Poll<Result<StreamResult>> {
            // This used to panic on zero-length reads with out-of-bounds
            // addresses:
            _ = source.as_direct(store).remaining();
            Poll::Ready(Ok(StreamResult::Dropped))
        }
    }

    let input = StreamReader::<u8>::new(&mut store, Producer)?;
    store
        .run_concurrent(async |accessor| {
            let (output,) = func.call_concurrent(accessor, (input,)).await?;

            accessor.with(|access| output.pipe(access, Consumer))?;

            future::poll_fn(|cx| accessor.poll_no_interesting_tasks(cx)).await;

            Ok(())
        })
        .await?
}

#[test]
#[cfg_attr(miri, ignore)]
fn sync_call_async_typed() -> Result<()> {
    const WAT: &str = r#"
(component
  (core func $task.return (canon task.return (result u32)))
  (core module $m
    (import "" "task.return" (func $task.return (param i32)))
    (func (export "run") (result i32)
      (call $task.return (i32.const 42))
      (i32.const 0 (; EXIT ;)))
    (func (export "run-cb") (param i32 i32 i32) (result i32)
      unreachable))
  (core instance $i (instantiate $m
    (with "" (instance (export "task.return" (func $task.return))))))
  (func (export "run") async (result u32)
    (canon lift (core func $i "run") async (callback (core func $i "run-cb"))))
)
"#;

    let engine = Engine::default();
    let component = Component::new(&engine, WAT)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;

    let func = instance.get_func(&mut store, "run").unwrap();
    let mut results = [Val::U32(0)];
    let error = func.call(&mut store, &[], &mut results).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("cannot synchronously call an async-lifted export"),
        "{error}"
    );

    let func = instance
        .get_typed_func::<(), (u32,)>(&mut store, "run")
        .unwrap();
    let error = func.call(&mut store, ()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("cannot synchronously call an async-lifted export"),
        "{error}"
    );

    Ok(())
}

/// This is a regression test for
/// https://github.com/bytecodealliance/wasmtime/issues/14504, in which the host
/// panicked due to a too-early deletion of a task group.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn task_group_deletion() -> Result<()> {
    struct Hook;

    impl TaskGroupHook for Hook {
        fn handle_start(&mut self, _id: TaskGroupId) -> Result<()> {
            Ok(())
        }
        fn handle_enter(&mut self, _id: TaskGroupId) -> Result<()> {
            Ok(())
        }
        fn handle_exit(&mut self, _id: TaskGroupId) -> Result<()> {
            Ok(())
        }
        fn handle_finish(&mut self, _id: TaskGroupId) -> Result<()> {
            Ok(())
        }
    }

    const WAT: &str = r#"
(component
  (core module $m
    (import "" "task.return" (func $task.return))
    (import "" "backpressure.inc" (func $backpressure.inc))
    ;; `bg`: turn on backpressure, return a result, then keep running in the
    ;; background by yielding; the next callback invocation traps.
    (func (export "bg") (result i32)
      (call $backpressure.inc)
      (call $task.return)
      (i32.const 1 (; YIELD ;)))
    (func (export "bg-cb") (param i32 i32 i32) (result i32)
      (i32.const 0 (; EXIT ;)))
    ;; `foo`: never gets to run because of the backpressure.
    (func (export "foo") (result i32)
      (call $task.return)
      (i32.const 0 (; EXIT ;)))
    (func (export "foo-cb") (param i32 i32 i32) (result i32)
      unreachable)
  )
  (core func $task.return (canon task.return))
  (core func $backpressure.inc (canon backpressure.inc))
  (core instance $i (instantiate $m (with "" (instance
    (export "task.return" (func $task.return))
    (export "backpressure.inc" (func $backpressure.inc))))))
  (func (export "bg") async
    (canon lift (core func $i "bg") async (callback (core func $i "bg-cb"))))
  (func (export "foo") async
    (canon lift (core func $i "foo") async (callback (core func $i "foo-cb"))))
)
"#;

    _ = env_logger::try_init();

    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, WAT)?;
    let mut store = Store::new(&engine, ());
    store.task_group_hook(Hook);
    let linker = Linker::new(&engine);
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let bg = instance.get_typed_func::<(), ()>(&mut store, "bg")?;
    let foo = instance.get_typed_func::<(), ()>(&mut store, "foo")?;

    bg.call_async(&mut store, ()).await?;
    let result = foo.call_async(&mut store, ()).await.unwrap_err();

    assert!(
        matches!(result.downcast_ref::<Trap>(), Some(Trap::AsyncDeadlock)),
        "{result:?}"
    );

    Ok(())
}

/// A guest's `future.write` to a host consumer which never takes the value
/// blocks, and once cancelled the write may be retried.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn retry_cancelled_future_write_to_host() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(
        &engine,
        r#"
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (type $f (future u32))
  (core func $future.new (canon future.new $f))
  (core func $future.write (canon future.write $f async (memory (core memory $libc "memory"))))
  (core func $future.cancel-write (canon future.cancel-write $f))
  (core func $task.return (canon task.return))
  (core module $m
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
    (import "" "future.cancel-write" (func $future.cancel-write (param i32) (result i32)))
    (import "" "task.return" (func $task.return))
    (global $w (mut i32) (i32.const 0))
    (func (export "new") (result i32)
      (local $pair i64)
      (local.set $pair (call $future.new))
      (global.set $w (i32.wrap_i64 (i64.shr_u (local.get $pair) (i64.const 32))))
      (i32.wrap_i64 (local.get $pair)))
    (func $write-then-cancel
      (if (i32.ne (call $future.write (global.get $w) (i32.const 0)) (i32.const -1 (; BLOCKED ;)))
        (then unreachable))
      (if (i32.ne (call $future.cancel-write (global.get $w)) (i32.const 2 (; CANCELLED ;)))
        (then unreachable)))
    (func (export "run") (result i32)
      (call $write-then-cancel)
      (call $write-then-cancel)
      (call $task.return)
      (i32.const 0 (; EXIT ;)))
    (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
  )
  (core instance $i (instantiate $m (with "" (instance
    (export "future.new" (func $future.new))
    (export "future.write" (func $future.write))
    (export "future.cancel-write" (func $future.cancel-write))
    (export "task.return" (func $task.return))))))
  (func (export "new") (result $f) (canon lift (core func $i "new")))
  (func (export "run") async
    (canon lift (core func $i "run") async (callback (core func $i "cb"))))
)
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let new = instance.get_typed_func::<(), (FutureReader<u32>,)>(&mut store, "new")?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    let (reader,) = new.call_async(&mut store, ()).await?;
    reader.pipe(&mut store, NeverConsume)?;
    run.call_async(&mut store, ()).await?;
    return Ok(());

    /// Never takes the value; acknowledges cancellation when asked to finish.
    struct NeverConsume;

    impl FutureConsumer<()> for NeverConsume {
        type Item = u32;

        fn poll_consume(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _store: StoreContextMut<()>,
            _source: Source<'_, u32>,
            finish: bool,
        ) -> Poll<Result<()>> {
            if finish {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }
    }
}

const THUNK_COMPONENT: &str = r#"
    (component
        (core module $m
            (func (export "thunk"))
        )
        (core instance $i (instantiate $m))
        (func (export "thunk") async
            (canon lift (core func $i "thunk"))
        )
    )
"#;

/// Dropping the result of `start_call_concurrent` outside of the store's event
/// loop should not panic.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn drop_start_call_concurrent_outside_event_loop() -> Result<()> {
    let engine = super::async_engine();
    let component = Component::new(&engine, THUNK_COMPONENT)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;

    let thunk = instance.get_typed_func::<(), ()>(&mut store, "thunk")?;

    let call = thunk.start_call_concurrent(&mut store, ())?;
    drop(call);

    Ok(())
}

/// An error returned from `TaskGroupHook::handle_finish` for a
/// `call_concurrent` should not panic.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn task_group_hook_finish_error_call_concurrent() -> Result<()> {
    _ = env_logger::try_init();
    struct FailFinish;

    impl TaskGroupHook for FailFinish {
        fn handle_start(&mut self, _: TaskGroupId) -> Result<()> {
            Ok(())
        }
        fn handle_enter(&mut self, _: TaskGroupId) -> Result<()> {
            Ok(())
        }
        fn handle_exit(&mut self, _: TaskGroupId) -> Result<()> {
            Ok(())
        }
        fn handle_finish(&mut self, _: TaskGroupId) -> Result<()> {
            wasmtime::bail!("handle_finish failed")
        }
    }

    let engine = super::async_engine();
    let component = Component::new(&engine, THUNK_COMPONENT)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let thunk = instance.get_typed_func::<(), ()>(&mut store, "thunk")?;
    store.task_group_hook(FailFinish);

    let result = store
        .run_concurrent(async |store| thunk.call_concurrent(store, ()).await)
        .await;
    assert!(result.is_err() || result.unwrap().is_err());

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn cancel_read_while_cancelling() -> Result<()> {
    let component = r#"
        (component
            (core module $libc (memory (export "memory") 1))
            (core instance $libc (instantiate $libc))
            (type $s (stream u8))
            (core func $stream.read (canon stream.read $s async (memory (core memory $libc "memory"))))
            (core func $stream.cancel-read (canon stream.cancel-read $s async))
            (core module $m
                (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
                (import "" "stream.cancel-read" (func $stream.cancel-read (param i32) (result i32)))
                (func (export "run") (param $s i32)
                    ;; Both the read and its cancellation block on the host.
                    (if (i32.ne (call $stream.read (local.get $s) (i32.const 0) (i32.const 1))
                                (i32.const -1))
                        (then unreachable))
                    (if (i32.ne (call $stream.cancel-read (local.get $s)) (i32.const -1))
                        (then unreachable))
                    ;; This must trap since a cancellation is already pending.
                    (drop (call $stream.cancel-read (local.get $s)))
                    unreachable)
            )
            (core instance $i (instantiate $m (with "" (instance
                (export "stream.read" (func $stream.read))
                (export "stream.cancel-read" (func $stream.cancel-read))
            ))))
            (func (export "run") (param "s" $s)
                (canon lift (core func $i "run")))
        )
    "#;

    let mut config = Config::new();
    config.wasm_component_model_async(true);
    config.wasm_component_model_more_async_builtins(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let run = instance.get_typed_func::<(StreamReader<u8>,), ()>(&mut store, "run")?;
    let reader = StreamReader::new(&mut store, Pending)?;
    let err = run.call_async(&mut store, (reader,)).await.unwrap_err();
    assert!(
        format!("{err:?}").contains("when a cancellation is already pending"),
        "{err:?}"
    );
    return Ok(());

    /// A producer which never produces anything.
    struct Pending;

    impl StreamProducer<()> for Pending {
        type Item = u8;
        type Buffer = Option<u8>;

        fn poll_produce<'a>(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _store: StoreContextMut<'a, ()>,
            _destination: Destination<'a, Self::Item, Self::Buffer>,
            _finish: bool,
        ) -> Poll<Result<StreamResult>> {
            Poll::Pending
        }
    }
}

/// A component whose export `loop` calls `B`, which calls `C`, which calls
/// `D`, `n` times, all callback-lifted: each core function makes its async
/// calls in a loop within a single activation. It also exports `D`'s export
/// as `nop`.
const NESTED_CALLBACK_CALLS: &str = r#"(component
    (component $D
        (core func $task.return (canon task.return))
        (core module $m
            (import "" "task.return" (func $task.return))
            (func (export "f") (result i32)
                (call $task.return)
                (i32.const 0 (; EXIT ;)))
            (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
        (core instance $i (instantiate $m
            (with "" (instance (export "task.return" (func $task.return))))))
        (func (export "f") async
            (canon lift (core func $i "f") async (callback (core func $i "cb"))))
    )
    (component $Caller
        (import "next" (func $next async))
        (core func $next (canon lower (func $next) async))
        (core func $task.return (canon task.return))
        (core module $m
            (import "" "next" (func $next (result i32)))
            (import "" "task.return" (func $task.return))
            (func (export "f") (param $n i32) (result i32)
                (loop $l
                    (if (i32.ne (call $next) (i32.const 2 (; RETURNED ;)))
                        (then unreachable))
                    (local.tee $n (i32.sub (local.get $n) (i32.const 1)))
                    br_if $l)
                (call $task.return)
                (i32.const 0 (; EXIT ;)))
            (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
        (core instance $i (instantiate $m (with "" (instance
            (export "next" (func $next))
            (export "task.return" (func $task.return))))))
        (func (export "f") async (param "n" u32)
            (canon lift (core func $i "f") async (callback (core func $i "cb"))))
    )
    (component $Once
        (import "next" (func $next async (param "n" u32)))
        (core func $next (canon lower (func $next) async))
        (core func $task.return (canon task.return))
        (core module $m
            (import "" "next" (func $next (param i32) (result i32)))
            (import "" "task.return" (func $task.return))
            (func (export "f") (result i32)
                (if (i32.ne (call $next (i32.const 1)) (i32.const 2 (; RETURNED ;)))
                    (then unreachable))
                (call $task.return)
                (i32.const 0 (; EXIT ;)))
            (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
        (core instance $i (instantiate $m (with "" (instance
            (export "next" (func $next))
            (export "task.return" (func $task.return))))))
        (func (export "f") async
            (canon lift (core func $i "f") async (callback (core func $i "cb"))))
    )
    (instance $d (instantiate $D))
    (instance $c (instantiate $Caller (with "next" (func $d "f"))))
    (instance $b (instantiate $Once (with "next" (func $c "f"))))
    (instance $a (instantiate $Caller (with "next" (func $b "f"))))
    (export "loop" (func $a "f"))
    (export "nop" (func $d "f"))
)"#;

/// A component exporting `nop`, an async callback-lifted export which returns
/// immediately.
const NOP: &str = r#"(component
    (core func $task.return (canon task.return))
    (core module $m
        (import "" "task.return" (func $task.return))
        (func (export "nop") (result i32)
            (call $task.return)
            (i32.const 0 (; EXIT ;)))
        (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m
        (with "" (instance (export "task.return" (func $task.return))))))
    (func (export "nop") async
        (canon lift (core func $i "nop") async (callback (core func $i "cb"))))
)"#;

/// Instantiates `NESTED_CALLBACK_CALLS` and `other` in a new store, returning
/// the store, `NESTED_CALLBACK_CALLS`'s `loop` and `nop`, and `other`'s
/// instance.
async fn instantiate_nested_calls(
    engine: &Engine,
    linker: &Linker<()>,
    other: &str,
) -> Result<(
    Store<()>,
    TypedFunc<(u32,), ()>,
    TypedFunc<(), ()>,
    Instance,
)> {
    let mut store = Store::new(engine, ());
    let nested = Component::new(engine, NESTED_CALLBACK_CALLS)?;
    let nested = Linker::new(engine)
        .instantiate_async(&mut store, &nested)
        .await?;
    let lp = nested.get_typed_func::<(u32,), ()>(&mut store, "loop")?;
    let nop = nested.get_typed_func::<(), ()>(&mut store, "nop")?;
    let other = Component::new(engine, other)?;
    let other = linker.instantiate_async(&mut store, &other).await?;
    Ok((store, lp, nop, other))
}

/// Dropping a `call_async` future whose call hasn't started, while the event
/// loop has yielded to the executor in the middle of a guest's async call,
/// doesn't panic. The dropped call fails once the event loop reaches it.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn drop_call_async_while_switch_pending() -> Result<()> {
    use futures::FutureExt as _;

    let engine = Engine::new(Config::new().wasm_component_model_async(true))?;
    let linker = Linker::new(&engine);
    let mut dropped = 0;
    // Enough rounds that the event loop yields to the executor partway
    // through them.
    for n in [32, 64, 128, 256] {
        let (mut store, lp, nop, _) = instantiate_nested_calls(&engine, &linker, NOP).await?;
        drop(lp.start_call_concurrent(&mut store, (n,))?);
        if nop.call_async(&mut store, ()).now_or_never().is_some() {
            continue;
        }
        dropped += 1;
        let err = nop.call_async(&mut store, ()).await.unwrap_err();
        assert!(
            format!("{err:?}").contains("dropped before it started"),
            "unexpected error: {err:?}"
        );
    }
    assert!(dropped > 0);
    Ok(())
}

/// A component exporting `f`, which passes its string parameter to the
/// `observe` import.
const OBSERVE_STRING: &str = r#"(component
    (import "observe" (func $observe (param "s" string)))
    (core module $libc
        (memory (export "mem") 1)
        (global $next (mut i32) (i32.const 1024))
        (func (export "realloc") (param i32 i32 i32 i32) (result i32)
            (local $ret i32)
            (local.set $ret (global.get $next))
            (global.set $next (i32.add (global.get $next) (local.get 3)))
            (local.get $ret)))
    (core instance $libc (instantiate $libc))
    (core func $observe
        (canon lower (func $observe) (memory (core memory $libc "mem"))))
    (core module $m
        (import "" "observe" (func $observe (param i32 i32)))
        (func (export "f") (param i32 i32)
            (call $observe (local.get 0) (local.get 1))))
    (core instance $i (instantiate $m
        (with "" (instance (export "observe" (func $observe))))))
    (func (export "f") (param "s" string)
        (canon lift (core func $i "f")
            (memory (core memory $libc "mem"))
            (realloc (core func $libc "realloc"))))
)"#;

/// The parameters of a `call_async` whose future is dropped before the call
/// starts are never lowered, so they may be borrowed from data which is freed
/// right afterwards: the call fails instead of running.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn dropped_call_async_never_lowers_params() -> Result<()> {
    use futures::FutureExt as _;

    let engine = Engine::new(Config::new().wasm_component_model_async(true))?;
    let observed = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut linker = Linker::new(&engine);
    linker.root().func_wrap("observe", {
        let observed = observed.clone();
        move |_, (s,): (String,)| {
            observed.lock().unwrap().push(s);
            Ok(())
        }
    })?;

    let (mut store, lp, nop, instance) =
        instantiate_nested_calls(&engine, &linker, OBSERVE_STRING).await?;
    drop(lp.start_call_concurrent(&mut store, (64,))?);
    let f = instance.get_typed_func::<(&str,), ()>(&mut store, "f")?;
    let s = "hello".to_string();
    assert!(f.call_async(&mut store, (&s,)).now_or_never().is_none());
    drop(s);
    let err = nop.call_async(&mut store, ()).await.unwrap_err();
    assert!(
        format!("{err:?}").contains("dropped before it started"),
        "unexpected error: {err:?}"
    );
    assert!(observed.lock().unwrap().is_empty());
    Ok(())
}

/// A `TaskGroupHook` which fails `handle_enter` once it's told to.
#[derive(Clone, Default)]
struct FailingHook(Arc<Mutex<bool>>);

impl TaskGroupHook for FailingHook {
    fn handle_start(&mut self, _: TaskGroupId) -> Result<()> {
        Ok(())
    }
    fn handle_enter(&mut self, _: TaskGroupId) -> Result<()> {
        if mem::take(&mut *self.0.lock().unwrap()) {
            wasmtime::bail!("failed to enter");
        }
        Ok(())
    }
    fn handle_exit(&mut self, _: TaskGroupId) -> Result<()> {
        Ok(())
    }
    fn handle_finish(&mut self, _: TaskGroupId) -> Result<()> {
        Ok(())
    }
}

/// A `call_async` whose start fails because no fiber stack is available
/// returns that error rather than panicking when its future is dropped.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn call_async_start_fails_without_fiber_stack() -> Result<()> {
    let mut pool = crate::small_pool_config();
    pool.total_memories(4);
    pool.total_stacks(4);
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    config.allocation_strategy(wasmtime::InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config)?;

    let component = Component::new(
        &engine,
        r#"(component
            (component $Blocker
                (core module $libc (memory (export "mem") 1))
                (core instance $libc (instantiate $libc))
                (core func $waitable-set.new (canon waitable-set.new))
                (core func $waitable-set.wait
                    (canon waitable-set.wait (memory (core memory $libc "mem"))))
                (core module $m
                    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
                    (import "" "waitable-set.wait"
                        (func $waitable-set.wait (param i32 i32) (result i32)))
                    ;; Blocks forever, holding a fiber.
                    (func (export "block")
                        (drop (call $waitable-set.wait (call $waitable-set.new) (i32.const 0)))
                        unreachable))
                (core instance $i (instantiate $m (with "" (instance
                    (export "waitable-set.new" (func $waitable-set.new))
                    (export "waitable-set.wait" (func $waitable-set.wait))))))
                (func (export "block") async (canon lift (core func $i "block"))))
            (component $Nop
                (core module $m (func (export "nop")))
                (core instance $i (instantiate $m))
                (func (export "nop") async (canon lift (core func $i "nop"))))
            (instance $b1 (instantiate $Blocker))
            (instance $b2 (instantiate $Blocker))
            (instance $b3 (instantiate $Blocker))
            (instance $b4 (instantiate $Blocker))
            (instance $n (instantiate $Nop))
            (export "block1" (func $b1 "block"))
            (export "block2" (func $b2 "block"))
            (export "block3" (func $b3 "block"))
            (export "block4" (func $b4 "block"))
            (export "nop" (func $n "nop"))
        )"#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    for name in ["block1", "block2", "block3", "block4"] {
        let f = instance.get_typed_func::<(), ()>(&mut store, name)?;
        drop(f.start_call_concurrent(&mut store, ())?);
    }
    let nop = instance.get_typed_func::<(), ()>(&mut store, "nop")?;
    let err = nop.call_async(&mut store, ()).await.unwrap_err();
    assert!(
        format!("{err:?}").contains("maximum concurrent limit of 4 for fibers reached"),
        "unexpected error: {err:?}"
    );
    Ok(())
}

/// A `call_async` whose start fails because a `TaskGroupHook` fails to enter
/// its task group returns the hook's error rather than panicking when its
/// future is dropped.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn task_group_hook_fails_to_enter_call_async() -> Result<()> {
    let engine = Engine::new(Config::new().wasm_component_model_async(true))?;
    let component = Component::new(&engine, NOP)?;
    let hook = FailingHook::default();
    let mut store = Store::new(&engine, ());
    store.task_group_hook(hook.clone());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let nop = instance.get_typed_func::<(), ()>(&mut store, "nop")?;

    *hook.0.lock().unwrap() = true;
    let err = nop.call_async(&mut store, ()).await.unwrap_err();
    assert!(
        format!("{err:?}").contains("failed to enter"),
        "unexpected error: {err:?}"
    );
    Ok(())
}

/// `call_async` from within a host function, while the event loop is already
/// running, fails, and dropping that call's future (whose start has already
/// been queued) doesn't panic. The dropped call fails once the event loop
/// reaches it, here in the next call.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn call_async_from_host_function() -> Result<()> {
    use futures::FutureExt as _;

    let engine = Engine::new(Config::new().wasm_component_model_async(true))?;
    let component = Component::new(
        &engine,
        r#"(component
            (import "host" (func $host))
            (component $Nop
                (core func $task.return (canon task.return))
                (core module $m
                    (import "" "task.return" (func $task.return))
                    (func (export "nop") (result i32)
                        (call $task.return)
                        (i32.const 0 (; EXIT ;)))
                    (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
                (core instance $i (instantiate $m
                    (with "" (instance (export "task.return" (func $task.return))))))
                (func (export "nop") async
                    (canon lift (core func $i "nop") async (callback (core func $i "cb")))))
            (instance $nop (instantiate $Nop))
            (core func $host (canon lower (func $host)))
            (core module $m
                (import "" "host" (func $host))
                (func (export "run") (call $host)))
            (core instance $i (instantiate $m (with "" (instance (export "host" (func $host))))))
            (export "nop" (func $nop "nop"))
            (func (export "run") (canon lift (core func $i "run")))
        )"#,
    )?;
    let nop = Arc::new(Mutex::new(None::<TypedFunc<(), ()>>));
    let mut linker = Linker::new(&engine);
    linker.root().func_wrap("host", {
        let nop = nop.clone();
        move |mut store, ()| {
            let nop = nop.lock().unwrap().unwrap();
            let result = nop.call_async(&mut store, ()).now_or_never();
            assert!(matches!(result, Some(Err(_))));
            Ok(())
        }
    })?;
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate_async(&mut store, &component).await?;
    *nop.lock().unwrap() = Some(instance.get_typed_func::<(), ()>(&mut store, "nop")?);
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    run.call_async(&mut store, ()).await?;
    let nop = nop.lock().unwrap().unwrap();
    let err = nop.call_async(&mut store, ()).await.unwrap_err();
    assert!(
        format!("{err:?}").contains("dropped before it started"),
        "unexpected error: {err:?}"
    );
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn yield_callback_after_dropped_event_loop() -> Result<()> {
    let component = r#"
(component
  (core func $task.return (canon task.return))
  (core module $m
    (import "" "task.return" (func $task.return))
    (global $n (mut i32) (i32.const 0))
    (func (export "run") (result i32) (i32.const 1 (; YIELD ;)))
    (func (export "run-cb") (param i32 i32 i32) (result i32)
      (global.set $n (i32.add (global.get $n) (i32.const 1)))
      (if (i32.ge_u (global.get $n) (i32.const 3))
        (then
          (call $task.return)
          (return (i32.const 0 (; EXIT ;)))))
      (i32.const 1 (; YIELD ;)))
  )
  (core instance $i (instantiate $m (with "" (instance
    (export "task.return" (func $task.return))
  ))))
  (func (export "run") async
    (canon lift (core func $i "run") async (callback (core func $i "run-cb"))))
)
"#;

    let engine = super::async_engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    // Drop the call when the event loop yields before calling the callback
    // again, then check that the next event loop finishes the task.
    let future = run.call_async(&mut store, ());
    assert!(PollOnce::new(Box::pin(future)).await.is_err());
    store
        .run_concurrent(async |_| yield_times(10).await)
        .await?;
    store.assert_concurrent_state_empty();
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn start_call_from_inline_stream_producer() -> Result<()> {
    // `read`'s `stream.read` polls the host producer below inline, and the
    // producer starts a concurrent call to `thunk`.
    let component = r#"
(component
  (type $s (stream u8))
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core func $stream.read (canon stream.read $s async (memory (core memory $libc "memory"))))
  (core module $m
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (func (export "thunk"))
    (func (export "read") (param $reader i32) (result i32)
      (call $stream.read (local.get $reader) (i32.const 0) (i32.const 1)))
  )
  (core instance $i (instantiate $m (with "" (instance
    (export "stream.read" (func $stream.read))
  ))))
  (func (export "thunk") (canon lift (core func $i "thunk")))
  (func (export "read") (param "s" $s) (result u32) (canon lift (core func $i "read")))
)
"#;

    struct Producer;

    impl StreamProducer<Option<TypedFunc<(), ()>>> for Producer {
        type Item = u8;
        type Buffer = Option<u8>;

        fn poll_produce<'a>(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            mut store: StoreContextMut<'a, Option<TypedFunc<(), ()>>>,
            mut destination: Destination<'a, u8, Option<u8>>,
            _finish: bool,
        ) -> Poll<Result<StreamResult>> {
            let thunk = store.data().unwrap();
            // This at least shouldn't panic...
            _ = thunk.start_call_concurrent(&mut store, ())?;
            destination.set_buffer(Some(0));
            Poll::Ready(Ok(StreamResult::Completed))
        }
    }

    let engine = super::async_engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, None);
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    *store.data_mut() = Some(instance.get_typed_func(&mut store, "thunk")?);
    let read = instance.get_typed_func::<(StreamReader<u8>,), (u32,)>(&mut store, "read")?;
    let reader = StreamReader::new(&mut store, Producer)?;
    let (code,) = read.call_async(&mut store, (reader,)).await?;
    assert_eq!(code, 1 << 4 /* COMPLETED, 1 item */);
    Ok(())
}

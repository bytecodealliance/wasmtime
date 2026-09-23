use crate::async_functions::{PollOnce, execute_across_threads};
use futures::stream::{FuturesUnordered, TryStreamExt as _};
use std::collections::HashMap;
use std::iter;
use std::mem;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use wasmtime::Result;
use wasmtime::component::{TaskGroupHook, TaskGroupId};
use wasmtime::{AsContextMut, Config, Engine, Store, StoreContextMut, Trap, component::*};
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
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;

    let mut store = Store::new(&engine, ());
    let component = Component::new(
        &engine,
        r#"(component
            (import "f" (func $f async))

            (core module $m
                (import "" "f" (func $f (result i32)))
                (import "" "cancel" (func $cancel (param i32) (result i32)))
                (import "" "drop" (func $drop (param i32)))
                (func (export "run")
                    (local i32)

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

                    ;; cancel the subtask asserting it's `RETURN_CANCELLED`
                    local.get 0
                    call $cancel
                    i32.const 4 ;; RETURN_CANCELLED
                    i32.ne
                    if unreachable end

                    ;; drop the subtask
                    local.get 0
                    call $drop
                )
            )
            (core func $f (canon lower (func $f) async))
            (core func $cancel (canon subtask.cancel))
            (core func $drop (canon subtask.drop))
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "f" (func $f))
                    (export "cancel" (func $cancel))
                    (export "drop" (func $drop))
                ))
            ))

            (func (export "f") async
                (canon lift (core func $i "run")))


        )"#,
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
    store
        .run_concurrent(async |store| -> wasmtime::Result<()> {
            func.call_concurrent(store, ()).await?;

            for _ in 0..5 {
                tokio::task::yield_now().await;
            }
            Ok(())
        })
        .await??;

    // The host task was cancelled, nothing should remain.
    store.assert_concurrent_state_empty();

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn sync_lower_async_host_does_not_leak() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;

    let mut store = Store::new(&engine, 0);
    let component = Component::new(
        &engine,
        r#"(component
            (import "f" (func $f async))

            (core module $m
                (import "" "f" (func $f))
                (func (export "run")
                    (local $c i32)

                    ;; call the host 100 times
                    loop
                      call $f
                      (local.tee $c (i32.add (local.get $c) (i32.const 1)))
                      i32.const 100
                      i32.ne
                      if br 1 end
                    end
                )
            )
            (core func $f (canon lower (func $f) ))
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "f" (func $f))
                ))
            ))

            (func (export "f") async
                (canon lift (core func $i "run")))


        )"#,
    )?;

    let mut linker = Linker::<usize>::new(&engine);
    linker
        .root()
        .func_wrap_concurrent("f", |accessor, (): ()| {
            Box::pin(async move {
                // Ensure that this doesn't hit the fast path of "ready on
                // first poll"
                for _ in 0..5 {
                    tokio::task::yield_now().await;
                }

                // Keep track of the maximum size of the table in
                // concurrent_state.
                accessor.with(|mut s| {
                    let cur = s.as_context_mut().concurrent_state_table_size();
                    let max = s.data_mut();
                    *max = (*max).max(cur);
                });
                Ok(())
            })
        })?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "f")?;
    func.call_async(&mut store, ()).await?;

    // First-level assertion: nothing should remain after the guest has exited.
    store.assert_concurrent_state_empty();

    // Second-level assertion: state should be incrementally cleaned up along
    // the way as the guest calls the host. Things shouldn't leak until the
    // guest exits at the end, for example.
    assert!(
        *store.data() < 100,
        "the store peaked at over 100 items in the concurrent table which \
         indicates that something isn't getting cleaned up between executions \
         of the host"
    );

    Ok(())
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

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn task_group_hook() -> Result<()> {
    _ = env_logger::try_init();
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;

    let component = Component::new(
        &engine,
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

                (func (export "a-callback") (param $event_code i32) (param $waitable i32) (param $payload i32) (result i32)
                    (if (i32.ne (local.get $event_code) (i32.const 1 (; SUBTASK ;)))
                        (then unreachable))
                    (if (i32.ne (local.get $payload) (i32.const 2 (; RETURNED ;)))
                        (then unreachable))
                    (call $waitable.join (local.get $waitable) (i32.const 0))
                    (call $subtask.drop (local.get $waitable))
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
    "#,
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

    let hook = Hook(Arc::new(Mutex::new(HookInner::default())));
    let yield_count = 5;

    let mut linker = Linker::new(&engine);
    linker.root().func_wrap_concurrent("a", {
        let hook = hook.clone();
        move |_, ()| {
            let hook = hook.clone();
            Box::pin(async move {
                let mut previous_id = None;
                let mut on_poll = move || {
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

                Ok(())
            })
        }
    })?;

    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "a")?;

    store.task_group_hook(hook.clone());

    let call_count = 3;

    // Call the guest export `call_count` times concurrently, recording the
    // events in `hook`:
    store
        .run_concurrent(async |store| {
            iter::repeat_with(|| func.call_concurrent(store, ()))
                .take(call_count)
                .collect::<FuturesUnordered<_>>()
                .try_collect::<()>()
                .await
        })
        .await??;

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

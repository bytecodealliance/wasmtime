use crate::async_functions::{PollOnce, execute_across_threads};
use std::future::{self, Future};
use std::ops::DerefMut;
use std::pin::{Pin, pin};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use wasmtime::Result;
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
async fn async_call_stack() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;

    let component = Component::new(
        &engine,
        r#"
        (component
            (import "a" (func $a))
            (core func $a (canon lower (func $a)))

            (core module $a
                (import "" "a" (func $a))
                (func (export "a") call $a)
            )
            (core instance $a (instantiate $a
                (with "" (instance (export "a" (func $a))))
            ))
            (func (export "a") async (canon lift (core func $a "a")))
        )
    "#,
    )?;

    let mut linker = Linker::new(&engine);
    linker.root().func_wrap(
        "a",
        |mut store: StoreContextMut<Option<GuestTaskId>>, (): ()| {
            let stack = store.async_call_stack()?.collect::<Vec<_>>();
            assert_eq!(stack, [store.data().unwrap()]);
            Ok(())
        },
    )?;
    let mut store = Store::new(&engine, None);
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "a")?;

    let call = func.start_call_concurrent(&mut store, ())?;
    *store.data_mut() = Some(call.task());
    store
        .run_concurrent(async |store| func.finish_call_concurrent(store, call).await)
        .await??;

    let component = Component::new(
        &engine,
        r#"
        (component
            (import "a" (func $a))
            (component $a
                (import "a" (func $a))
                (core func $a (canon lower (func $a)))

                (core module $a
                    (import "" "a" (func $a))
                    (func (export "a") call $a)
                )
                (core instance $a (instantiate $a
                    (with "" (instance (export "a" (func $a))))
                ))
                (func (export "a") (canon lift (core func $a "a")))
            )

            (instance $a (instantiate $a (with "a" (func $a))))
            (instance $b (instantiate $a (with "a" (func $a "a"))))
            (export "a" (func $b "a"))
        )
    "#,
    )?;

    let mut linker = Linker::new(&engine);
    linker.root().func_wrap(
        "a",
        |mut store: StoreContextMut<Option<GuestTaskId>>, (): ()| {
            let stack = store.async_call_stack()?.collect::<Vec<_>>();
            assert_eq!(stack.len(), 2);
            assert_eq!(stack.last(), store.data().as_ref());
            Ok(())
        },
    )?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "a")?;

    let call = func.start_call_concurrent(&mut store, ())?;
    *store.data_mut() = Some(call.task());
    store
        .run_concurrent(async |store| func.finish_call_concurrent(store, call).await)
        .await??;
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
async fn lower_stream_payload_type_mismatch() -> wasmtime::Result<()> {
    // A component that echoes streams and futures of streams and futures of
    // strings back to the caller, creates a stream of streams of strings, and
    // accepts the same with `u32` instead of `string`.
    const WAT: &str = r#"
(component
  (type $s (stream string))
  (type $ss (stream $s))
  (type $f (future string))
  (type $ff (future $f))
  (type $u (stream u32))
  (type $su (stream $u))
  (type $fu32 (future u32))
  (type $fu (future $fu32))
  (core func $stream.new (canon stream.new $ss))
  (core module $m
    (import "" "stream.new" (func $stream.new (result i64)))
    (func (export "echo") (param i32) (result i32) local.get 0)
    (func (export "new-stream") (result i32)
      (i32.wrap_i64 (call $stream.new)))
    (func (export "take") (param i32)))
  (core instance $i (instantiate $m
    (with "" (instance (export "stream.new" (func $stream.new))))))
  (func (export "echo-stream") (param "s" $ss) (result $ss)
    (canon lift (core func $i "echo")))
  (func (export "echo-future") (param "f" $ff) (result $ff)
    (canon lift (core func $i "echo")))
  (func (export "new-stream") (result $ss)
    (canon lift (core func $i "new-stream")))
  (func (export "take-stream-string") (param "s" $ss)
    (canon lift (core func $i "take")))
  (func (export "take-stream-u32") (param "s" $su)
    (canon lift (core func $i "take")))
  (func (export "take-future-u32") (param "f" $fu)
    (canon lift (core func $i "take")))
)
"#;

    let engine = Engine::default();
    let component = Component::new(&engine, WAT)?;

    // An error while lowering parameters leaves the instance unusable, so each
    // case below uses a new store and instance.
    let instantiate = async || -> Result<(Store<()>, Instance)> {
        let mut store = Store::new(&engine, ());
        let instance = Linker::new(&engine)
            .instantiate_async(&mut store, &component)
            .await?;
        Ok((store, instance))
    };

    // Created with `StreamReader<String>` items, which can't be lowered as
    // `stream<u32>` items, even though `StreamAny` statically matches it. The
    // host still owns the stream afterwards, so it can close it.
    let (mut store, instance) = instantiate().await?;
    let echo = instance
        .get_typed_func::<(StreamReader<StreamReader<String>>,), (StreamReader<StreamAny>,)>(
            &mut store,
            "echo-stream",
        )?;
    let take = instance
        .get_typed_func::<(&StreamReader<StreamAny>,), ()>(&mut store, "take-stream-u32")?;
    let inner = StreamReader::new(&mut store, vec!["hello".to_string()])?;
    let outer = StreamReader::new(&mut store, vec![inner])?;
    let (mut outer,) = echo.call_async(&mut store, (outer,)).await?;
    assert!(take.call_async(&mut store, (&outer,)).await.is_err());
    outer.close(&mut store)?;

    // The same through `Val`.
    let (mut store, instance) = instantiate().await?;
    let echo = instance
        .get_typed_func::<(StreamReader<StreamReader<String>>,), (StreamReader<StreamAny>,)>(
            &mut store,
            "echo-stream",
        )?;
    let take = instance.get_func(&mut store, "take-stream-u32").unwrap();
    let inner = StreamReader::new(&mut store, vec!["hello".to_string()])?;
    let outer = StreamReader::new(&mut store, vec![inner])?;
    let (outer,) = echo.call_async(&mut store, (outer,)).await?;
    let outer = outer.try_into_stream_any(&mut store)?;
    assert!(
        take.call_async(&mut store, &[Val::Stream(outer)], &mut [])
            .await
            .is_err()
    );

    // The same for futures.
    let (mut store, instance) = instantiate().await?;
    let echo = instance
        .get_typed_func::<(FutureReader<FutureReader<String>>,), (FutureReader<FutureAny>,)>(
            &mut store,
            "echo-future",
        )?;
    let take =
        instance.get_typed_func::<(FutureReader<FutureAny>,), ()>(&mut store, "take-future-u32")?;
    let inner = FutureReader::new(&mut store, async {
        wasmtime::error::Ok("hello".to_string())
    })?;
    let outer = FutureReader::new(&mut store, async move { wasmtime::error::Ok(inner) })?;
    let (outer,) = echo.call_async(&mut store, (outer,)).await?;
    assert!(take.call_async(&mut store, (outer,)).await.is_err());

    // The same for a stream created by a guest.
    let (mut store, instance) = instantiate().await?;
    let new =
        instance.get_typed_func::<(), (StreamReader<StreamAny>,)>(&mut store, "new-stream")?;
    let take =
        instance.get_typed_func::<(StreamReader<StreamAny>,), ()>(&mut store, "take-stream-u32")?;
    let (stream,) = new.call_async(&mut store, ()).await?;
    assert!(take.call_async(&mut store, (stream,)).await.is_err());

    // A matching payload type is still accepted.
    let (mut store, instance) = instantiate().await?;
    let echo = instance
        .get_typed_func::<(StreamReader<StreamReader<String>>,), (StreamReader<StreamAny>,)>(
            &mut store,
            "echo-stream",
        )?;
    let take = instance
        .get_typed_func::<(StreamReader<StreamAny>,), ()>(&mut store, "take-stream-string")?;
    let inner = StreamReader::new(&mut store, vec!["hello".to_string()])?;
    let outer = StreamReader::new(&mut store, vec![inner])?;
    let (outer,) = echo.call_async(&mut store, (outer,)).await?;
    take.call_async(&mut store, (outer,)).await?;

    let (mut store, instance) = instantiate().await?;
    let new =
        instance.get_typed_func::<(), (StreamReader<StreamAny>,)>(&mut store, "new-stream")?;
    let take = instance
        .get_typed_func::<(StreamReader<StreamAny>,), ()>(&mut store, "take-stream-string")?;
    let (stream,) = new.call_async(&mut store, ()).await?;
    take.call_async(&mut store, (stream,)).await?;

    Ok(())
}

/// Regression test for case which previously panicked when finishing a call
/// which was started before an intervening, failing call.
#[tokio::test]
#[cfg_attr(miri, ignore)]
// TODO: currently panics (but should not) in debug mode:
#[cfg_attr(
    debug_assertions,
    should_panic = "BUG: suspend reason unexpectedly already set for store"
)]
async fn call_between_start_and_finish() {
    async {
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

        store
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
            .await?
    }
    .await
    .unwrap()
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
            buffer.reserve(source.remaining(store.as_context_mut()));
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

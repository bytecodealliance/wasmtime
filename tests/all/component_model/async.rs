use crate::async_functions::{PollOnce, execute_across_threads};
use anyhow::Result;
use wasmtime::{AsContextMut, Config, component::*};
use wasmtime::{Engine, Store, StoreContextMut, Trap};
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
    thunk.post_return_async(&mut store).await?;

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
    thunk.post_return_async(&mut store).await?;

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
    config.async_support(true);
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
                        (memory $libc "memory")
                        (realloc (func $libc "realloc"))
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
        Ok::<_, anyhow::Error>(())
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
    config.async_support(true);
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
                                (memory $i "memory")
                                (realloc (func $i "realloc"))))
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
            Ok::<_, anyhow::Error>(())
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
        Ok::<_, anyhow::Error>(())
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
        f.post_return_async(&mut store).await?;
        Ok::<_, anyhow::Error>(())
    })
    .await?;

    assert_eq!("after yield", *drop_status.lock().unwrap());

    Ok(())
}

/// Test task deletion in three situations, for every combination of lift/lower/(guest/host):
/// 1. An explicit thread calls task.return
/// 2. An explicit thread suspends indefinitely
/// 3. An explicit thread yield loops indefinitely
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn task_deletion() -> Result<()> {
    let mut config = Config::new();
    config.async_support(true);
    config.wasm_component_model_async(true);
    config.wasm_component_model_threading(true);
    config.wasm_component_model_async_stackful(true);
    config.wasm_component_model_async_builtins(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(
        &engine,
        r#"(component
    (component $C
        (core module $Memory (memory (export "mem") 1))
        (core instance $memory (instantiate $Memory))
        ;; Defines the table for the thread start functions
        (core module $libc
            (table (export "__indirect_function_table") 3 funcref))
        (core module $CM
            (import "" "mem" (memory 1))
            (import "" "task.return" (func $task-return (param i32)))
            (import "" "task.cancel" (func $task-cancel))
            (import "" "thread.new-indirect" (func $thread-new-indirect (param i32 i32) (result i32)))
            (import "" "thread.suspend" (func $thread-suspend (result i32)))
            (import "" "thread.suspend-cancellable" (func $thread-suspend-cancellable (result i32)))
            (import "" "thread.yield-to" (func $thread-yield-to (param i32) (result i32)))
            (import "" "thread.yield-to-cancellable" (func $thread-yield-to-cancellable (param i32) (result i32)))
            (import "" "thread.switch-to" (func $thread-switch-to (param i32) (result i32)))
            (import "" "thread.switch-to-cancellable" (func $thread-switch-to-cancellable (param i32) (result i32)))
            (import "" "thread.yield" (func $thread-yield (result i32)))
            (import "" "thread.yield-cancellable" (func $thread-yield-cancellable (result i32)))
            (import "" "thread.index" (func $thread-index (result i32)))
            (import "" "thread.resume-later" (func $thread-resume-later (param i32)))
            (import "" "waitable.join" (func $waitable.join (param i32 i32)))
            (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
            (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
            (import "libc" "__indirect_function_table" (table $indirect-function-table 3 funcref))

            ;; Indices into the function table for the thread start functions
            (global $call-return-ftbl-idx i32 (i32.const 0))
            (global $suspend-ftbl-idx i32 (i32.const 1))
            (global $yield-loop-ftbl-idx i32 (i32.const 2))

            (func $call-return (param i32)
                (call $task-return (local.get 0)))

            (func $suspend (param i32)
                (drop (call $thread-suspend)))

            (func $yield-loop (param i32)
                (loop $top
                    (drop (call $thread-yield))
                    (br $top)))

            (func (export "explicit-thread-calls-return-stackful")
                (call $thread-resume-later
                    (call $thread-new-indirect (global.get $call-return-ftbl-idx) (i32.const 42))))

            (func (export "explicit-thread-calls-return-stackless") (result i32)
                (call $thread-resume-later
                    (call $thread-new-indirect (global.get $call-return-ftbl-idx) (i32.const 42)))
                (i32.const 0 (; EXIT ;)))

            (func (export "cb") (param i32 i32 i32) (result i32)
                (unreachable))

            (func (export "explicit-thread-suspends-sync") (result i32)
                (call $thread-resume-later
                    (call $thread-new-indirect (global.get $suspend-ftbl-idx) (i32.const 42)))
                (i32.const 42))

            (func (export "explicit-thread-suspends-stackful")
                (call $thread-resume-later
                    (call $thread-new-indirect (global.get $suspend-ftbl-idx) (i32.const 42)))
                (call $task-return (i32.const 42)))

            (func (export "explicit-thread-suspends-stackless") (result i32)
                (call $thread-resume-later
                    (call $thread-new-indirect (global.get $suspend-ftbl-idx) (i32.const 42)))
                (call $task-return (i32.const 42))
                (i32.const 0))

            (func (export "explicit-thread-yield-loops-sync") (result i32)
                (call $thread-resume-later
                    (call $thread-new-indirect (global.get $yield-loop-ftbl-idx) (i32.const 42)))
                (i32.const 42))

            (func (export "explicit-thread-yield-loops-stackful")
                (call $thread-resume-later
                    (call $thread-new-indirect (global.get $yield-loop-ftbl-idx) (i32.const 42)))
                (call $task-return (i32.const 42)))

            (func (export "explicit-thread-yield-loops-stackless") (result i32)
                (call $thread-resume-later
                    (call $thread-new-indirect (global.get $suspend-ftbl-idx) (i32.const 42)))
                (call $task-return (i32.const 42))
                (i32.const 0 (; EXIT ;)))

            ;; Initialize the function table that will be used by thread.new-indirect
            (elem (table $indirect-function-table) (i32.const 0 (; call-return-ftbl-idx ;)) func $call-return)
            (elem (table $indirect-function-table) (i32.const 1 (; suspend-ftbl-idx ;)) func $suspend)
            (elem (table $indirect-function-table) (i32.const 2 (; yield-loop-ftbl-idx ;)) func $yield-loop)
        )

        ;; Instantiate the libc module to get the table
        (core instance $libc (instantiate $libc))
        ;; Get access to `thread.new-indirect` that uses the table from libc
        (core type $start-func-ty (func (param i32)))
        (alias core export $libc "__indirect_function_table" (core table $indirect-function-table))

        (core func $task-return (canon task.return (result u32)))
        (core func $task-cancel (canon task.cancel))
        (core func $thread-new-indirect
            (canon thread.new-indirect $start-func-ty (table $indirect-function-table)))
        (core func $thread-yield (canon thread.yield))
        (core func $thread-yield-cancellable (canon thread.yield cancellable))
        (core func $thread-index (canon thread.index))
        (core func $thread-yield-to (canon thread.yield-to))
        (core func $thread-yield-to-cancellable (canon thread.yield-to cancellable))
        (core func $thread-resume-later (canon thread.resume-later))
        (core func $thread-switch-to (canon thread.switch-to))
        (core func $thread-switch-to-cancellable (canon thread.switch-to cancellable))
        (core func $thread-suspend (canon thread.suspend))
        (core func $thread-suspend-cancellable (canon thread.suspend cancellable))
        (core func $waitable-set.new (canon waitable-set.new))
        (core func $waitable.join (canon waitable.join))
        (core func $waitable-set.wait (canon waitable-set.wait (memory $memory "mem")))

        ;; Instantiate the main module
        (core instance $cm (
            instantiate $CM
                (with "" (instance
                    (export "mem" (memory $memory "mem"))
                    (export "task.return" (func $task-return))
                    (export "task.cancel" (func $task-cancel))
                    (export "thread.new-indirect" (func $thread-new-indirect))
                    (export "thread.index" (func $thread-index))
                    (export "thread.yield-to" (func $thread-yield-to))
                    (export "thread.yield-to-cancellable" (func $thread-yield-to-cancellable))
                    (export "thread.yield" (func $thread-yield))
                    (export "thread.yield-cancellable" (func $thread-yield-cancellable))
                    (export "thread.switch-to" (func $thread-switch-to))
                    (export "thread.switch-to-cancellable" (func $thread-switch-to-cancellable))
                    (export "thread.suspend" (func $thread-suspend))
                    (export "thread.suspend-cancellable" (func $thread-suspend-cancellable))
                    (export "thread.resume-later" (func $thread-resume-later))
                    (export "waitable.join" (func $waitable.join))
                    (export "waitable-set.wait" (func $waitable-set.wait))
                    (export "waitable-set.new" (func $waitable-set.new))))
                (with "libc" (instance $libc))))

        (func (export "explicit-thread-calls-return-stackful") (result u32)
            (canon lift (core func $cm "explicit-thread-calls-return-stackful") async))
        (func (export "explicit-thread-calls-return-stackless") (result u32)
            (canon lift (core func $cm "explicit-thread-calls-return-stackless") async (callback (func $cm "cb"))))
        (func (export "explicit-thread-suspends-sync") (result u32)
            (canon lift (core func $cm "explicit-thread-suspends-sync")))
        (func (export "explicit-thread-suspends-stackful") (result u32)
            (canon lift (core func $cm "explicit-thread-suspends-stackful") async))
        (func (export "explicit-thread-suspends-stackless") (result u32)
            (canon lift (core func $cm "explicit-thread-suspends-stackless") async (callback (func $cm "cb"))))
        (func (export "explicit-thread-yield-loops-sync") (result u32)
            (canon lift (core func $cm "explicit-thread-yield-loops-sync")))
        (func (export "explicit-thread-yield-loops-stackful") (result u32)
            (canon lift (core func $cm "explicit-thread-yield-loops-stackful") async))
        (func (export "explicit-thread-yield-loops-stackless") (result u32)
            (canon lift (core func $cm "explicit-thread-yield-loops-stackless") async (callback (func $cm "cb"))))
    )

    (component $D
        (import "explicit-thread-calls-return-stackful" (func $explicit-thread-calls-return-stackful (result u32)))
        (import "explicit-thread-calls-return-stackless" (func $explicit-thread-calls-return-stackless (result u32)))
        (import "explicit-thread-suspends-sync" (func $explicit-thread-suspends-sync (result u32)))
        (import "explicit-thread-suspends-stackful" (func $explicit-thread-suspends-stackful (result u32)))
        (import "explicit-thread-suspends-stackless" (func $explicit-thread-suspends-stackless (result u32)))
        (import "explicit-thread-yield-loops-sync" (func $explicit-thread-yield-loops-sync (result u32)))
        (import "explicit-thread-yield-loops-stackful" (func $explicit-thread-yield-loops-stackful (result u32)))
        (import "explicit-thread-yield-loops-stackless" (func $explicit-thread-yield-loops-stackless (result u32)))

        (core module $Memory (memory (export "mem") 1))
        (core instance $memory (instantiate $Memory))
        (core module $DM
            (import "" "mem" (memory 1))
            (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
            ;; sync lowered
            (import "" "explicit-thread-calls-return-stackful" (func $explicit-thread-calls-return-stackful (result i32)))
            (import "" "explicit-thread-calls-return-stackless" (func $explicit-thread-calls-return-stackless (result i32)))
            (import "" "explicit-thread-suspends-sync" (func $explicit-thread-suspends-sync (result i32)))
            (import "" "explicit-thread-suspends-stackful" (func $explicit-thread-suspends-stackful (result i32)))
            (import "" "explicit-thread-suspends-stackless" (func $explicit-thread-suspends-stackless (result i32)))
            (import "" "explicit-thread-yield-loops-sync" (func $explicit-thread-yield-loops-sync (result i32)))
            (import "" "explicit-thread-yield-loops-stackful" (func $explicit-thread-yield-loops-stackful (result i32)))
            (import "" "explicit-thread-yield-loops-stackless" (func $explicit-thread-yield-loops-stackless (result i32)))
            ;; async lowered
            (import "" "explicit-thread-calls-return-stackful-async" (func $explicit-thread-calls-return-stackful-async (param i32) (result i32)))
            (import "" "explicit-thread-calls-return-stackless-async" (func $explicit-thread-calls-return-stackless-async (param i32) (result i32)))
            (import "" "explicit-thread-suspends-sync-async" (func $explicit-thread-suspends-sync-async (param i32) (result i32)))
            (import "" "explicit-thread-suspends-stackful-async" (func $explicit-thread-suspends-stackful-async (param i32) (result i32)))
            (import "" "explicit-thread-suspends-stackless-async" (func $explicit-thread-suspends-stackless-async (param i32) (result i32)))
            (import "" "explicit-thread-yield-loops-sync-async" (func $explicit-thread-yield-loops-sync-async (param i32) (result i32)))
            (import "" "explicit-thread-yield-loops-stackful-async" (func $explicit-thread-yield-loops-stackful-async (param i32) (result i32)))
            (import "" "explicit-thread-yield-loops-stackless-async" (func $explicit-thread-yield-loops-stackless-async (param i32) (result i32)))
            (import "" "waitable.join" (func $waitable.join (param i32 i32)))
            (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
            (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
            (import "" "thread.yield" (func $thread-yield (result i32)))

            (func $check (param i32)
                (if (i32.ne (local.get 0) (i32.const 42))
                    (then unreachable))
            )

            (func $check-async (param i32)
                (local $retp i32) (local $ws i32) (local $ws-retp i32)
                (local.set $retp (i32.const 8))
                (local.set $ws-retp (i32.const 16))
                (local.set $ws (call $waitable-set.new))

                (if (i32.eq (i32.and (local.get 0) (i32.const 0xF)) (i32.const 2 (; RETURNED ;)))
                    (then (call $check (i32.load (local.get $retp))))
                    (else
                        (call $waitable.join (i32.shr_u (local.get 0) (i32.const 4)) (local.get $ws))
                        (drop (call $waitable-set.wait (local.get $ws) (local.get $ws-retp)))
                        (call $check (i32.load (local.get $retp)))))
            )

            (func $run (export "run") (result i32)
                (local $retp i32)
                (local.set $retp (i32.const 8))
                (call $check (call $explicit-thread-calls-return-stackless))
                (call $check (call $explicit-thread-calls-return-stackful))
                (call $check (call $explicit-thread-suspends-sync))
                (call $check (call $explicit-thread-suspends-stackful))
                (call $check (call $explicit-thread-suspends-stackless))
                (call $check (call $explicit-thread-yield-loops-sync))
                (call $check (call $explicit-thread-yield-loops-stackful))
                (call $check (call $explicit-thread-yield-loops-stackless))

                (call $check-async (call $explicit-thread-calls-return-stackless-async (local.get $retp)))
                (call $check-async (call $explicit-thread-calls-return-stackful-async (local.get $retp)))
                (call $check-async (call $explicit-thread-suspends-sync-async (local.get $retp)))
                (call $check-async (call $explicit-thread-suspends-stackful-async (local.get $retp)))
                (call $check-async (call $explicit-thread-suspends-stackless-async (local.get $retp)))
                (call $check-async (call $explicit-thread-yield-loops-sync-async (local.get $retp)))
                (call $check-async (call $explicit-thread-yield-loops-stackful-async (local.get $retp)))
                (call $check-async (call $explicit-thread-yield-loops-stackless-async (local.get $retp)))

                (i32.const 42)
            )
        )

        (core func $waitable-set.new (canon waitable-set.new))
        (core func $waitable-set.wait (canon waitable-set.wait (memory $memory "mem")))
        (core func $waitable.join (canon waitable.join))
        (core func $subtask.cancel (canon subtask.cancel async))
        (core func $thread.yield (canon thread.yield))
        ;; sync lowered
        (canon lower (func $explicit-thread-calls-return-stackful) (memory $memory "mem") (core func $explicit-thread-calls-return-stackful'))
        (canon lower (func $explicit-thread-calls-return-stackless) (memory $memory "mem") (core func $explicit-thread-calls-return-stackless'))
        (canon lower (func $explicit-thread-suspends-sync) (memory $memory "mem") (core func $explicit-thread-suspends-sync'))
        (canon lower (func $explicit-thread-suspends-stackful) (memory $memory "mem") (core func $explicit-thread-suspends-stackful'))
        (canon lower (func $explicit-thread-suspends-stackless) (memory $memory "mem") (core func $explicit-thread-suspends-stackless'))
        (canon lower (func $explicit-thread-yield-loops-sync) (memory $memory "mem") (core func $explicit-thread-yield-loops-sync'))
        (canon lower (func $explicit-thread-yield-loops-stackful) (memory $memory "mem") (core func $explicit-thread-yield-loops-stackful'))
        (canon lower (func $explicit-thread-yield-loops-stackless) (memory $memory "mem") (core func $explicit-thread-yield-loops-stackless'))
        ;; async lowered
        (canon lower (func $explicit-thread-calls-return-stackful) async (memory $memory "mem") (core func $explicit-thread-calls-return-stackful-async'))
        (canon lower (func $explicit-thread-calls-return-stackless) async (memory $memory "mem") (core func $explicit-thread-calls-return-stackless-async'))
        (canon lower (func $explicit-thread-suspends-sync) async (memory $memory "mem") (core func $explicit-thread-suspends-sync-async'))
        (canon lower (func $explicit-thread-suspends-stackful) async (memory $memory "mem") (core func $explicit-thread-suspends-stackful-async'))
        (canon lower (func $explicit-thread-suspends-stackless) async (memory $memory "mem") (core func $explicit-thread-suspends-stackless-async'))
        (canon lower (func $explicit-thread-yield-loops-sync) async (memory $memory "mem") (core func $explicit-thread-yield-loops-sync-async'))
        (canon lower (func $explicit-thread-yield-loops-stackful) async (memory $memory "mem") (core func $explicit-thread-yield-loops-stackful-async'))
        (canon lower (func $explicit-thread-yield-loops-stackless) async (memory $memory "mem") (core func $explicit-thread-yield-loops-stackless-async'))
        (core instance $dm (instantiate $DM (with "" (instance
            (export "mem" (memory $memory "mem"))
            (export "explicit-thread-calls-return-stackful" (func $explicit-thread-calls-return-stackful'))
            (export "explicit-thread-calls-return-stackless" (func $explicit-thread-calls-return-stackless'))
            (export "explicit-thread-suspends-sync" (func $explicit-thread-suspends-sync'))
            (export "explicit-thread-suspends-stackful" (func $explicit-thread-suspends-stackful'))
            (export "explicit-thread-suspends-stackless" (func $explicit-thread-suspends-stackless'))
            (export "explicit-thread-yield-loops-sync" (func $explicit-thread-yield-loops-sync'))
            (export "explicit-thread-yield-loops-stackful" (func $explicit-thread-yield-loops-stackful'))
            (export "explicit-thread-yield-loops-stackless" (func $explicit-thread-yield-loops-stackless'))
            (export "explicit-thread-calls-return-stackful-async" (func $explicit-thread-calls-return-stackful-async'))
            (export "explicit-thread-calls-return-stackless-async" (func $explicit-thread-calls-return-stackless-async'))
            (export "explicit-thread-suspends-sync-async" (func $explicit-thread-suspends-sync-async'))
            (export "explicit-thread-suspends-stackful-async" (func $explicit-thread-suspends-stackful-async'))
            (export "explicit-thread-suspends-stackless-async" (func $explicit-thread-suspends-stackless-async'))
            (export "explicit-thread-yield-loops-sync-async" (func $explicit-thread-yield-loops-sync-async'))
            (export "explicit-thread-yield-loops-stackful-async" (func $explicit-thread-yield-loops-stackful-async'))
            (export "explicit-thread-yield-loops-stackless-async" (func $explicit-thread-yield-loops-stackless-async'))
            (export "waitable.join" (func $waitable.join))
            (export "waitable-set.new" (func $waitable-set.new))
            (export "waitable-set.wait" (func $waitable-set.wait))
            (export "subtask.cancel" (func $subtask.cancel))
            (export "thread.yield" (func $thread.yield))
        ))))
        (func (export "run") (result u32) (canon lift (core func $dm "run")))
    )

    (instance $c (instantiate $C))
    (instance $d (instantiate $D
        (with "explicit-thread-calls-return-stackful" (func $c "explicit-thread-calls-return-stackful"))
        (with "explicit-thread-calls-return-stackless" (func $c "explicit-thread-calls-return-stackless"))
        (with "explicit-thread-suspends-sync" (func $c "explicit-thread-suspends-sync"))
        (with "explicit-thread-suspends-stackful" (func $c "explicit-thread-suspends-stackful"))
        (with "explicit-thread-suspends-stackless" (func $c "explicit-thread-suspends-stackless"))
        (with "explicit-thread-yield-loops-sync" (func $c "explicit-thread-yield-loops-sync"))
        (with "explicit-thread-yield-loops-stackful" (func $c "explicit-thread-yield-loops-stackful"))
        (with "explicit-thread-yield-loops-stackless" (func $c "explicit-thread-yield-loops-stackless"))
    ))
  (func (export "run") (alias export $d "run"))
  (func (export "explicit-thread-calls-return-stackful") (alias export $c "explicit-thread-calls-return-stackful"))
  (func (export "explicit-thread-calls-return-stackless") (alias export $c "explicit-thread-calls-return-stackless"))
  (func (export "explicit-thread-suspends-sync") (alias export $c "explicit-thread-suspends-sync"))
  (func (export "explicit-thread-suspends-stackful") (alias export $c "explicit-thread-suspends-stackful"))
  (func (export "explicit-thread-suspends-stackless") (alias export $c "explicit-thread-suspends-stackless"))
  (func (export "explicit-thread-yield-loops-sync") (alias export $c "explicit-thread-yield-loops-sync"))
  (func (export "explicit-thread-yield-loops-stackful") (alias export $c "explicit-thread-yield-loops-stackful"))
  (func (export "explicit-thread-yield-loops-stackless") (alias export $c "explicit-thread-yield-loops-stackless"))
)
        "#,
    )?
    .serialize()?;

    let component = unsafe { Component::deserialize(&engine, &component)? };
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let funcs = vec![
        "run",
        "explicit-thread-calls-return-stackful",
        "explicit-thread-calls-return-stackless",
        "explicit-thread-suspends-sync",
        "explicit-thread-suspends-stackful",
        "explicit-thread-suspends-stackless",
        "explicit-thread-yield-loops-sync",
        "explicit-thread-yield-loops-stackful",
        "explicit-thread-yield-loops-stackless",
    ];
    for func in funcs {
        let func = instance.get_typed_func::<(), (u32,)>(&mut store, func)?;
        assert_eq!(func.call_async(&mut store, ()).await?, (42,));
        func.post_return_async(store.as_context_mut()).await?;
    }

    Ok(())
}

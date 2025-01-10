#![cfg(not(miri))]

use crate::async_functions::{PollOnce, execute_across_threads};
use anyhow::Result;
use wasmtime::component::*;
use wasmtime::{Engine, Store, StoreContextMut, Trap};
use wasmtime_component_util::REALLOC_AND_FREE;

/// This is super::func::thunks, except with an async store.
#[tokio::test]
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
async fn resume_separate_thread() -> Result<()> {
    let mut config = component_test_util::config();
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
async fn poll_through_wasm_activation() -> Result<()> {
    let mut config = component_test_util::config();
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

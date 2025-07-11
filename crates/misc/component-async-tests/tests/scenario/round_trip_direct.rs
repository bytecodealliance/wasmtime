use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::util::{config, make_component};
use anyhow::{Result, anyhow};
use component_async_tests::Ctx;
use component_async_tests::util::sleep;
use futures::stream::{FuturesUnordered, TryStreamExt};
use wasmtime::component::{Linker, ResourceTable, Val};
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::WasiCtxBuilder;

#[tokio::test]
pub async fn async_round_trip_direct_stackless() -> Result<()> {
    test_round_trip_direct_uncomposed(
        test_programs_artifacts::ASYNC_ROUND_TRIP_DIRECT_STACKLESS_COMPONENT,
    )
    .await
}

async fn test_round_trip_direct_uncomposed(component: &str) -> Result<()> {
    test_round_trip_direct(
        &[component],
        "hello, world!",
        "hello, world! - entered guest - entered host - exited host - exited guest",
    )
    .await
}

async fn test_round_trip_direct(
    components: &[&str],
    input: &str,
    expected_output: &str,
) -> Result<()> {
    let engine = Engine::new(&config())?;

    let make_store = || {
        Store::new(
            &engine,
            Ctx {
                wasi: WasiCtxBuilder::new().inherit_stdio().build(),
                table: ResourceTable::default(),
                continue_: false,
                wakers: Arc::new(Mutex::new(None)),
            },
        )
    };

    let component = make_component(&engine, components).await?;

    // First, test the `wasmtime-wit-bindgen` static API:
    {
        let mut linker = Linker::new(&engine);

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        component_async_tests::round_trip_direct::bindings::RoundTripDirect::add_to_linker_imports::<
            _,
            Ctx,
        >(&mut linker, |ctx| ctx)?;

        let mut store = make_store();

        let instance = linker.instantiate_async(&mut store, &component).await?;
        let round_trip = component_async_tests::round_trip_direct::bindings::RoundTripDirect::new(
            &mut store, &instance,
        )?;

        instance
            .run_with(&mut store, {
                let input = input.to_owned();
                let expected_output = expected_output.to_owned();
                async move |accessor| {
                    // Start three concurrent calls and then join them all:
                    let mut futures = FuturesUnordered::new();
                    for _ in 0..3 {
                        futures.push(round_trip.call_foo(accessor, input.clone()));
                    }

                    while let Some(value) = futures.try_next().await? {
                        assert_eq!(expected_output, value);
                    }

                    anyhow::Ok(())
                }
            })
            .await??;
    }

    // Now do it again using the dynamic API (except for WASI, where we stick with the static API):
    {
        let mut linker = Linker::new(&engine);

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        linker
            .root()
            .func_new_concurrent("foo", |_, params, results| {
                Box::pin(async move {
                    sleep(Duration::from_millis(10)).await;
                    let Some(Val::String(s)) = params.into_iter().next() else {
                        unreachable!()
                    };
                    results[0] = Val::String(format!("{s} - entered host - exited host"));
                    Ok(())
                })
            })?;

        let mut store = make_store();

        let instance = linker.instantiate_async(&mut store, &component).await?;
        let foo_function = instance
            .get_export_index(&mut store, None, "foo")
            .ok_or_else(|| anyhow!("can't find `foo` in instance"))?;
        let foo_function = instance
            .get_func(&mut store, foo_function)
            .ok_or_else(|| anyhow!("can't find `foo` in instance"))?;

        // Start three concurrent calls and then join them all:
        instance
            .run_with(&mut store, async |store| -> wasmtime::Result<_> {
                let mut futures = FuturesUnordered::new();
                for _ in 0..3 {
                    futures.push(
                        foo_function.call_concurrent(store, vec![Val::String(input.to_owned())]),
                    );
                }

                while let Some(value) = futures.try_next().await? {
                    let Some(Val::String(value)) = value.into_iter().next() else {
                        unreachable!()
                    };
                    assert_eq!(expected_output, &value);
                }
                Ok(())
            })
            .await??;
    }

    Ok(())
}

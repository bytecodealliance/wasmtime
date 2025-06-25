use std::env;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use futures::stream::{FuturesUnordered, TryStreamExt};
use wasmtime::component::{Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::WasiCtxBuilder;

use component_async_tests::util::{config, make_component};

#[tokio::test]
pub async fn async_borrowing_caller() -> Result<()> {
    test_run_bool(
        &[
            test_programs_artifacts::ASYNC_BORROWING_CALLER_COMPONENT,
            test_programs_artifacts::ASYNC_BORROWING_CALLEE_COMPONENT,
        ],
        false,
    )
    .await
}

#[tokio::test]
async fn async_borrowing_caller_misbehave() -> Result<()> {
    let error = format!(
        "{:?}",
        test_run_bool(
            &[
                test_programs_artifacts::ASYNC_BORROWING_CALLER_COMPONENT,
                test_programs_artifacts::ASYNC_BORROWING_CALLEE_COMPONENT
            ],
            true
        )
        .await
        .unwrap_err()
    );
    assert!(error.contains("unknown handle index"), "{error}");
    Ok(())
}

#[tokio::test]
async fn async_borrowing_callee_misbehave() -> Result<()> {
    let error = format!(
        "{:?}",
        test_run_bool(
            &[test_programs_artifacts::ASYNC_BORROWING_CALLEE_COMPONENT],
            true
        )
        .await
        .unwrap_err()
    );
    assert!(error.contains("unknown handle index"), "{error}");
    Ok(())
}

#[tokio::test]
pub async fn async_borrowing_callee() -> Result<()> {
    test_run_bool(
        &[test_programs_artifacts::ASYNC_BORROWING_CALLEE_COMPONENT],
        false,
    )
    .await
}

pub async fn test_run_bool(components: &[&str], v: bool) -> Result<()> {
    let mut config = config();
    // As of this writing, miri/pulley/epochs is a problematic combination, so
    // we don't test it.
    if env::var_os("MIRI_TEST_CWASM_DIR").is_none() {
        config.epoch_interruption(true);
    }

    let engine = Engine::new(&config)?;

    let component = make_component(&engine, components).await?;

    let mut linker = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    component_async_tests::borrowing_host::bindings::local::local::borrowing_types::add_to_linker::<
        _,
        component_async_tests::Ctx,
    >(&mut linker, |ctx| ctx)?;

    let mut store = Store::new(
        &engine,
        component_async_tests::Ctx {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            table: ResourceTable::default(),
            continue_: false,
            wakers: Arc::new(Mutex::new(None)),
        },
    );

    if env::var_os("MIRI_TEST_CWASM_DIR").is_none() {
        store.set_epoch_deadline(1);

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(10));
            engine.increment_epoch();
        });
    }

    let instance = linker.instantiate_async(&mut store, &component).await?;
    let borrowing_host =
        component_async_tests::borrowing_host::bindings::BorrowingHost::new(&mut store, &instance)?;

    // Start three concurrent calls and then join them all:
    let mut futures = FuturesUnordered::new();
    for _ in 0..3 {
        futures.push(
            borrowing_host
                .local_local_run_bool()
                .call_run(&mut store, v),
        );
    }

    while let Some(()) = instance.run(&mut store, futures.try_next()).await?? {
        // continue
    }

    Ok(())
}

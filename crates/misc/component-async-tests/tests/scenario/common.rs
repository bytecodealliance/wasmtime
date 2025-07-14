use std::sync::{Arc, Mutex, Once};
use std::time::Duration;

use anyhow::Result;
use tokio::fs;
use wasm_compose::composer::ComponentComposer;
use wasmtime::component::{Component, Linker, PromisesUnordered, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::WasiCtxBuilder;

use component_async_tests::Ctx;

pub fn annotate<T, F>(val: F) -> F
where
    F: Fn(&mut T) -> &mut T,
{
    val
}

pub fn init_logger() {
    static ONCE: Once = Once::new();
    ONCE.call_once(pretty_env_logger::init);
}

/// Compose two components
///
/// a is the "root" component, and b is composed into it
#[allow(unused)]
pub async fn compose(a: &[u8], b: &[u8]) -> Result<Vec<u8>> {
    let dir = tempfile::tempdir()?;

    let a_file = dir.path().join("a.wasm");
    fs::write(&a_file, a).await?;

    let b_file = dir.path().join("b.wasm");
    fs::write(&b_file, b).await?;

    ComponentComposer::new(
        &a_file,
        &wasm_compose::config::Config {
            dir: dir.path().to_owned(),
            definitions: vec![b_file.to_owned()],
            ..Default::default()
        },
    )
    .compose()
}

#[allow(unused)]
pub async fn test_run(component: &[u8]) -> Result<()> {
    let mut config = config();
    // As of this writing, miri/pulley/epochs is a problematic combination, so
    // we don't test it.
    if env::var_os("MIRI_TEST_CWASM_DIR").is_none() {
        config.epoch_interruption(true);
    }

    let engine = Engine::new(&config)?;

    let component = Component::new(&engine, component)?;

    let mut linker = Linker::new(&engine);

    wasmtime_wasi::add_to_linker_async(&mut linker)?;
    component_async_tests::yield_host::bindings::local::local::continue_::add_to_linker_get_host(
        &mut linker,
        annotate(|ctx| ctx),
    )?;
    component_async_tests::yield_host::bindings::local::local::ready::add_to_linker_get_host(
        &mut linker,
        annotate(|ctx| ctx),
    )?;
    component_async_tests::resource_stream::bindings::local::local::resource_stream::add_to_linker_get_host(
        &mut linker,
        annotate(|ctx| ctx),
    )?;

    let mut store = Store::new(
        &engine,
        Ctx {
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

    let yield_host = component_async_tests::yield_host::bindings::YieldHost::instantiate_async(
        &mut store, &component, &linker,
    )
    .await?;

    // Start three concurrent calls and then join them all:
    let mut promises = PromisesUnordered::new();
    for _ in 0..3 {
        promises.push(yield_host.local_local_run().call_run(&mut store).await?);
    }

    while let Some(()) = promises.next(&mut store).await? {
        // continue
    }

    Ok(())
}

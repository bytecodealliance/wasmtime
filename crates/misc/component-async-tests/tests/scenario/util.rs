use std::collections::HashMap;
use std::env;
use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc, LazyLock, Once};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use component_async_tests::{Ctx, sleep};
use futures::stream::{FuturesUnordered, TryStreamExt};
use tokio::fs;
use tokio::sync::Mutex;
use wasm_compose::composer::ComponentComposer;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::p2::WasiCtxBuilder;

pub fn init_logger() {
    static ONCE: Once = Once::new();
    ONCE.call_once(env_logger::init);
}

pub fn config() -> Config {
    init_logger();

    let mut config = Config::new();
    if env::var_os("MIRI_TEST_CWASM_DIR").is_some() {
        config.target("pulley64").unwrap();
        config.memory_reservation(1 << 20);
        config.memory_guard_size(0);
        config.signals_based_traps(false);
    } else {
        config.cranelift_debug_verifier(true);
    }
    config.wasm_component_model(true);
    config.wasm_component_model_async(true);
    config.wasm_component_model_async_builtins(true);
    config.wasm_component_model_async_stackful(true);
    config.wasm_component_model_error_context(true);
    config.async_support(true);
    config
}

/// Compose two components
///
/// a is the "root" component, and b is composed into it
async fn compose(a: &[u8], b: &[u8]) -> Result<Vec<u8>> {
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

pub async fn make_component(engine: &Engine, components: &[&str]) -> Result<Component> {
    fn cwasm_name(components: &[&str]) -> Result<String> {
        if components.is_empty() {
            Err(anyhow!("expected at least one path"))
        } else {
            let names = components
                .iter()
                .map(|&path| {
                    let path = Path::new(path);
                    if let Some(name) = path.file_name() {
                        Ok(name)
                    } else {
                        Err(anyhow!(
                            "expected path with at least two components; got: {}",
                            path.display()
                        ))
                    }
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(format!(
                "{}.cwasm",
                names
                    .iter()
                    .map(|name| { name.to_str().unwrap() })
                    .collect::<Vec<_>>()
                    .join("+")
            ))
        }
    }

    async fn compile(engine: &Engine, components: &[&str]) -> Result<Vec<u8>> {
        match components {
            [component] => engine.precompile_component(&fs::read(component).await?),
            [a, b] => engine
                .precompile_component(&compose(&fs::read(a).await?, &fs::read(b).await?).await?),
            _ => Err(anyhow!("expected one or two paths")),
        }
    }

    async fn load(engine: &Engine, components: &[&str]) -> Result<Vec<u8>> {
        let cwasm_path = if let Some(cwasm_dir) = &env::var_os("MIRI_TEST_CWASM_DIR") {
            Some(Path::new(cwasm_dir).join(cwasm_name(components)?))
        } else {
            None
        };

        if let Some(cwasm_path) = &cwasm_path {
            if let Ok(compiled) = fs::read(cwasm_path).await {
                return Ok(compiled);
            }
        }

        if cfg!(miri) {
            bail!(
                "Running these tests with miri requires precompiled .cwasm files.\n\
                 Please set the `MIRI_TEST_CWASM_DIR` environment variable to the\n\
                 absolute path of a valid directory, then run the test(s)\n\
                 _without_ miri, and finally run them again _with_ miri."
            )
        }

        let compiled = compile(engine, components).await?;
        if let Some(cwasm_path) = &cwasm_path {
            fs::write(cwasm_path, &compiled).await?;
        }
        Ok(compiled)
    }

    static CACHE: LazyLock<Mutex<HashMap<Vec<String>, Arc<Mutex<Option<Arc<Vec<u8>>>>>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    let compiled = {
        let entry = CACHE
            .lock()
            .await
            .entry(components.iter().map(|&s| s.to_owned()).collect())
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone();

        let mut entry = entry.lock().await;
        if let Some(component) = entry.deref() {
            component.clone()
        } else {
            let component = Arc::new(load(engine, components).await?);
            *entry = Some(component.clone());
            component
        }
    };

    Ok(unsafe { Component::deserialize(&engine, &*compiled)? })
}

pub async fn test_run(components: &[&str]) -> Result<()> {
    test_run_with_count(components, 3).await
}

pub async fn test_run_with_count(components: &[&str], count: usize) -> Result<()> {
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
    component_async_tests::yield_host::bindings::local::local::continue_::add_to_linker::<_, Ctx>(
        &mut linker,
        |ctx| ctx,
    )?;
    component_async_tests::yield_host::bindings::local::local::ready::add_to_linker::<_, Ctx>(
        &mut linker,
        |ctx| ctx,
    )?;
    component_async_tests::resource_stream::bindings::local::local::resource_stream::add_to_linker::<
        _,
        Ctx,
    >(&mut linker, |ctx| ctx)?;
    sleep::local::local::sleep::add_to_linker::<_, Ctx>(&mut linker, |ctx| ctx)?;

    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            table: ResourceTable::default(),
            continue_: false,
            wakers: Arc::new(std::sync::Mutex::new(None)),
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
    let yield_host =
        component_async_tests::yield_host::bindings::YieldHost::new(&mut store, &instance)?;

    // Start `count` concurrent calls and then join them all:
    instance
        .run_concurrent(&mut store, async |store| {
            let mut futures = FuturesUnordered::new();
            for _ in 0..count {
                futures.push(yield_host.local_local_run().call_run(store));
            }

            while let Some(()) = futures.try_next().await? {
                // continue
            }
            anyhow::Ok(())
        })
        .await??;

    Ok(())
}

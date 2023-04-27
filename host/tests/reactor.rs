use anyhow::Result;
use host::WasiCtx;
use std::sync::{Arc, RwLock};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};

lazy_static::lazy_static! {
    static ref ENGINE: Engine = {
        let mut config = Config::new();
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(true);
        config.async_support(true);

        let engine = Engine::new(&config).unwrap();
        engine
    };
}

// uses ENGINE, creates a fn get_component(&str) -> Component
test_programs_macros::reactor_components!();

wasmtime::component::bindgen!({
    path: "../test-programs/reactor-tests/wit",
    world: "test-reactor",
    async: true,
    with: {
       "environment": host::wasi::environment,
       "streams": host::wasi::streams,
       "preopens": host::wasi::preopens,
       "filesystem": host::wasi::filesystem,
       "exit": host::wasi::exit,
    },
});

async fn instantiate(
    component: Component,
    wasi_ctx: WasiCtx,
) -> Result<(Store<WasiCtx>, TestReactor)> {
    let mut linker = Linker::new(&ENGINE);

    // All of the imports available to the world are provided by the host crate:
    host::wasi::filesystem::add_to_linker(&mut linker, |x| x)?;
    host::wasi::streams::add_to_linker(&mut linker, |x| x)?;
    host::wasi::environment::add_to_linker(&mut linker, |x| x)?;
    host::wasi::preopens::add_to_linker(&mut linker, |x| x)?;
    host::wasi::exit::add_to_linker(&mut linker, |x| x)?;

    let mut store = Store::new(&ENGINE, wasi_ctx);

    let (testreactor, _instance) =
        TestReactor::instantiate_async(&mut store, &component, &linker).await?;
    Ok((store, testreactor))
}

#[test_log::test(tokio::test)]
async fn reactor_tests() -> Result<()> {
    let wasi = WasiCtxBuilder::new().build()?;

    let (mut store, reactor) = instantiate(get_component("reactor_tests"), wasi).await?;

    store
        .data_mut()
        .env
        .push(("GOOD_DOG".to_owned(), "gussie".to_owned()));

    let r = reactor
        .call_add_strings(&mut store, &["hello", "$GOOD_DOG"])
        .await?;
    assert_eq!(r, 2);

    // Redefine the env, show that the adapter only fetches it once
    // even if the libc ctors copy it in multiple times:
    store.data_mut().env.clear();
    store
        .data_mut()
        .env
        .push(("GOOD_DOG".to_owned(), "cody".to_owned()));
    // Cody is indeed good but this should be "hello again" "gussie"
    let r = reactor
        .call_add_strings(&mut store, &["hello again", "$GOOD_DOG"])
        .await?;
    assert_eq!(r, 4);

    let contents = reactor.call_get_strings(&mut store).await?;
    assert_eq!(contents, &["hello", "gussie", "hello again", "gussie"]);

    // Show that we can pass in a resource type whose impls are defined in the
    // `host` and `wasi-common` crate.
    // Note, this works because of the add_to_linker invocations using the
    // `host` crate for `streams`, not because of `with` in the bindgen macro.
    let write_dest: Arc<RwLock<Vec<u8>>> = Arc::new(RwLock::new(Vec::new()));
    let writepipe = wasi_common::pipe::WritePipe::from_shared(write_dest.clone());
    let table_ix = store
        .data_mut()
        .table_mut()
        .push_output_stream(Box::new(writepipe))?;
    let r = reactor.call_write_strings_to(&mut store, table_ix).await?;
    assert_eq!(r, Ok(()));

    assert_eq!(*write_dest.read().unwrap(), b"hellogussiehello againgussie");

    // Show that the `with` invocation in the macro means we get to re-use the
    // type definitions from inside the `host` crate for these structures:
    let ds = host::wasi::filesystem::DescriptorStat {
        data_access_timestamp: host::wasi::wall_clock::Datetime {
            nanoseconds: 123,
            seconds: 45,
        },
        data_modification_timestamp: host::wasi::wall_clock::Datetime {
            nanoseconds: 789,
            seconds: 10,
        },
        device: 0,
        inode: 0,
        link_count: 0,
        size: 0,
        status_change_timestamp: host::wasi::wall_clock::Datetime {
            nanoseconds: 0,
            seconds: 1,
        },
        type_: host::wasi::filesystem::DescriptorType::Unknown,
    };
    let expected = format!("{ds:?}");
    let got = reactor.call_pass_an_imported_record(&mut store, ds).await?;
    assert_eq!(expected, got);

    Ok(())
}

#![cfg(all(feature = "test_programs", not(skip_wasi_http_tests)))]
use futures::future;
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
use wasmtime_wasi::preview2::{
    command::{add_to_linker, Command},
    Table, WasiCtx, WasiCtxBuilder, WasiView,
};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

use test_programs::http_server;

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
// uses ENGINE, creates a fn get_module(&str) -> Module
include!(concat!(env!("OUT_DIR"), "/wasi_http_tests_components.rs"));

struct Ctx {
    table: Table,
    wasi: WasiCtx,
    http: WasiHttpCtx,
}

impl WasiView for Ctx {
    fn table(&self) -> &Table {
        &self.table
    }
    fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }
    fn ctx(&self) -> &WasiCtx {
        &self.wasi
    }
    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl WasiHttpView for Ctx {
    fn http_ctx(&self) -> &WasiHttpCtx {
        &self.http
    }
    fn http_ctx_mut(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }
}

async fn instantiate_component(
    component: Component,
    ctx: Ctx,
) -> Result<(Store<Ctx>, Command), anyhow::Error> {
    let mut linker = Linker::new(&ENGINE);
    add_to_linker(&mut linker)?;
    wasmtime_wasi_http::add_to_component_linker(&mut linker)?;

    let mut store = Store::new(&ENGINE, ctx);

    let (command, _instance) = Command::instantiate_async(&mut store, &component, &linker).await?;
    Ok((store, command))
}

async fn run(name: &str) -> anyhow::Result<()> {
    let mut table = Table::new();
    let component = get_component(name);

    // Create our wasi context.
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .arg(name)
        .build(&mut table)?;
    let http = WasiHttpCtx::new();

    let (mut store, command) = instantiate_component(component, Ctx { table, wasi, http }).await?;
    command
        .call_run(&mut store)
        .await
        .map_err(|e| anyhow::anyhow!("wasm failed with {e:?}"))?
        .map_err(|e| anyhow::anyhow!("command returned with failing exit status {e:?}"))
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn outbound_request() {
    let (_, result) = future::join(http_server::run_server(), run("outbound_request")).await;
    result.unwrap();
}

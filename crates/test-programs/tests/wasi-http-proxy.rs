#![cfg(all(feature = "test_programs", not(skip_wasi_http_tests)))]
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
use wasmtime_wasi::preview2::{
    self, pipe::MemoryOutputPipe, IsATTY, Table, WasiCtx, WasiCtxBuilder, WasiView,
};
use wasmtime_wasi_http::{bindings, proxy::Proxy, types, WasiHttpCtx, WasiHttpView};

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
include!(concat!(
    env!("OUT_DIR"),
    "/wasi_http_proxy_tests_components.rs"
));

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
    fn table(&mut self) -> &mut Table {
        &mut self.table
    }
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }
}

async fn instantiate(component: Component, ctx: Ctx) -> Result<(Store<Ctx>, Proxy), anyhow::Error> {
    let mut linker = Linker::new(&ENGINE);
    wasmtime_wasi_http::proxy::add_to_linker(&mut linker)?;

    // due to the preview1 adapter
    preview2::bindings::filesystem::types::add_to_linker(&mut linker, |l| l)?;
    preview2::bindings::filesystem::preopens::add_to_linker(&mut linker, |l| l)?;
    preview2::bindings::cli::environment::add_to_linker(&mut linker, |l| l)?;
    preview2::bindings::cli::exit::add_to_linker(&mut linker, |l| l)?;
    preview2::bindings::cli::terminal_input::add_to_linker(&mut linker, |l| l)?;
    preview2::bindings::cli::terminal_output::add_to_linker(&mut linker, |l| l)?;
    preview2::bindings::cli::terminal_stdin::add_to_linker(&mut linker, |l| l)?;
    preview2::bindings::cli::terminal_stdout::add_to_linker(&mut linker, |l| l)?;
    preview2::bindings::cli::terminal_stderr::add_to_linker(&mut linker, |l| l)?;

    let mut store = Store::new(&ENGINE, ctx);

    let (proxy, _instance) = Proxy::instantiate_async(&mut store, &component, &linker).await?;
    Ok((store, proxy))
}

#[test_log::test(tokio::test)]
async fn wasi_http_proxy_tests() -> anyhow::Result<()> {
    let stdout = MemoryOutputPipe::new(4096);
    let stderr = MemoryOutputPipe::new(4096);

    let mut table = Table::new();
    let component = get_component("wasi_http_proxy_tests");

    // Create our wasi context.
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone(), IsATTY::No);
    builder.stderr(stderr.clone(), IsATTY::No);
    for (var, val) in test_programs::wasi_tests_environment() {
        builder.env(var, val);
    }
    let wasi = builder.build(&mut table)?;
    let http = WasiHttpCtx;

    let mut ctx = Ctx { table, wasi, http };

    let (mut store, proxy) = instantiate(component, ctx).await?;

    let req = store
        .data_mut()
        .new_incoming_request(types::HostIncomingRequest {
            method: bindings::http::types::Method::Get,
        })?;

    let out = store.data_mut().new_response_outparam()?;

    proxy
        .wasi_http_incoming_handler()
        .call_handle(&mut store, req, out)
        .await?;

    let resp = store.data_mut().take_response_outparam(out)?;

    let resp = match resp {
        Some(Ok(resp)) => resp,
        Some(Err(e)) => panic!("Error given in response: {e:?}"),
        None => panic!("No response given for request!"),
    };

    let stdout = stdout.contents();
    if !stdout.is_empty() {
        println!("[guest] stdout:\n{}\n===", String::from_utf8_lossy(&stdout));
    }
    let stderr = stderr.contents();
    if !stderr.is_empty() {
        println!("[guest] stderr:\n{}\n===", String::from_utf8_lossy(&stderr));
    }

    Ok(())
}

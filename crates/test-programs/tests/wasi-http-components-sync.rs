#![cfg(all(feature = "test_programs", not(skip_wasi_http_tests)))]
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
use wasmtime_wasi::preview2::{
    command::sync::{add_to_linker, Command},
    pipe::MemoryOutputPipe,
    Table, WasiCtx, WasiCtxBuilder, WasiView,
};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

use test_programs::http_server::{setup_http1_sync, setup_http2_sync};

lazy_static::lazy_static! {
    static ref ENGINE: Engine = {
        let mut config = Config::new();
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(true);
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
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }

    fn table(&mut self) -> &mut Table {
        &mut self.table
    }
}

fn instantiate_component(
    component: Component,
    ctx: Ctx,
) -> Result<(Store<Ctx>, Command), anyhow::Error> {
    let mut linker = Linker::new(&ENGINE);
    add_to_linker(&mut linker)?;
    wasmtime_wasi_http::proxy::add_to_linker(&mut linker)?;

    let mut store = Store::new(&ENGINE, ctx);

    let (command, _instance) = Command::instantiate(&mut store, &component, &linker)?;
    Ok((store, command))
}

fn run(name: &str) -> anyhow::Result<()> {
    let stdout = MemoryOutputPipe::new(4096);
    let stderr = MemoryOutputPipe::new(4096);
    let r = {
        let table = Table::new();
        let component = get_component(name);

        // Create our wasi context.
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.stderr(stderr.clone());
        builder.arg(name);
        for (var, val) in test_programs::wasi_tests_environment() {
            builder.env(var, val);
        }
        let wasi = builder.build();
        let http = WasiHttpCtx {};

        let (mut store, command) = instantiate_component(component, Ctx { table, wasi, http })?;
        command
            .wasi_cli_run()
            .call_run(&mut store)?
            .map_err(|()| anyhow::anyhow!("run returned a failure"))?;
        Ok(())
    };
    r.map_err(move |trap: anyhow::Error| {
        let stdout = stdout.try_into_inner().expect("single ref to stdout");
        if !stdout.is_empty() {
            println!("[guest] stdout:\n{}\n===", String::from_utf8_lossy(&stdout));
        }
        let stderr = stderr.try_into_inner().expect("single ref to stderr");
        if !stderr.is_empty() {
            println!("[guest] stderr:\n{}\n===", String::from_utf8_lossy(&stderr));
        }
        trap.context(format!(
            "error while testing wasi-tests {} with http-components-sync",
            name
        ))
    })?;
    Ok(())
}

#[test_log::test]
#[cfg_attr(
    windows,
    ignore = "test is currently flaky in ci and needs to be debugged"
)]
fn outbound_request_get() {
    setup_http1_sync(|| run("outbound_request_get")).unwrap();
}

#[test_log::test]
#[cfg_attr(
    windows,
    ignore = "test is currently flaky in ci and needs to be debugged"
)]
fn outbound_request_post() {
    setup_http1_sync(|| run("outbound_request_post")).unwrap();
}

#[test_log::test]
#[cfg_attr(
    windows,
    ignore = "test is currently flaky in ci and needs to be debugged"
)]
fn outbound_request_large_post() {
    setup_http1_sync(|| run("outbound_request_large_post")).unwrap();
}

#[test_log::test]
#[cfg_attr(
    windows,
    ignore = "test is currently flaky in ci and needs to be debugged"
)]
fn outbound_request_put() {
    setup_http1_sync(|| run("outbound_request_put")).unwrap();
}

#[test_log::test]
#[cfg_attr(
    windows,
    ignore = "test is currently flaky in ci and needs to be debugged"
)]
fn outbound_request_invalid_version() {
    setup_http2_sync(|| run("outbound_request_invalid_version")).unwrap();
}

#[test_log::test]
fn outbound_request_unknown_method() {
    run("outbound_request_unknown_method").unwrap();
}

#[test_log::test]
fn outbound_request_unsupported_scheme() {
    run("outbound_request_unsupported_scheme").unwrap();
}

#[test_log::test]
fn outbound_request_invalid_port() {
    run("outbound_request_invalid_port").unwrap();
}

#[test_log::test]
#[cfg_attr(
    windows,
    ignore = "test is currently flaky in ci and needs to be debugged"
)]
fn outbound_request_invalid_dnsname() {
    run("outbound_request_invalid_dnsname").unwrap();
}

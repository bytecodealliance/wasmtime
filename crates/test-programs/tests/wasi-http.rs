#![cfg(all(feature = "test_programs", not(skip_wasi_http_tests)))]
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
use wasmtime_wasi::preview2::{
    command::{add_to_linker, Command},
    Table, WasiCtx, WasiCtxBuilder, WasiView,
};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::{body::Bytes, service::service_fn, Request, Response};
use std::{error::Error, net::SocketAddr};
use tokio::{net::TcpListener, runtime::Handle};

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

async fn test(
    req: Request<hyper::body::Incoming>,
) -> http::Result<Response<BoxBody<Bytes, hyper::Error>>> {
    let method = req.method().to_string();
    Response::builder()
        .status(http::StatusCode::OK)
        .header("x-wasmtime-test-method", method)
        .header("x-wasmtime-test-uri", req.uri().to_string())
        .body(req.into_body().boxed())
}

async fn async_run_serve() -> Result<(), Box<dyn Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, service_fn(test))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

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

async fn instantiate(
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
    let _thread = Handle::current().spawn(async move {
        async_run_serve()
            .await
            .map_err(|_| anyhow::anyhow!("error while running test server"))
            .unwrap();
    });

    let mut table = Table::new();
    let component = get_component(name);

    // Create our wasi context.
    let http = WasiHttpCtx::new();
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .push_arg(name)
        .build(&mut table)?;

    let (mut store, command) = instantiate(component, Ctx { table, wasi, http }).await?;
    command
        .call_run(&mut store)
        .await
        .map_err(|e| anyhow::anyhow!("wasm failed with {e:?}"))?
        .map_err(|e| anyhow::anyhow!("command returned with failing exit status {e:?}"))
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn wasi_http_tests() {
    run("wasi_http_tests").await.unwrap()
}

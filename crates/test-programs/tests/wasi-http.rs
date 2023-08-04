#![cfg(all(feature = "test_programs", not(skip_wasi_http_tests)))]
use wasmtime::{Config, Engine, Linker, Store};
use wasmtime_wasi::{sync::WasiCtxBuilder, WasiCtx};
use wasmtime_wasi_http::{
    http_acl::{acl::HttpRequestMethod, HttpAcl},
    WasiHttp,
};

use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::{body::Bytes, service::service_fn, Request, Response};
use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};
use tokio::net::TcpListener;

lazy_static::lazy_static! {
    static ref ENGINE: Engine = {
        let mut config = Config::new();
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        let engine = Engine::new(&config).unwrap();
        engine
    };
}
// uses ENGINE, creates a fn get_module(&str) -> Module
include!(concat!(env!("OUT_DIR"), "/wasi_http_tests_modules.rs"));

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

fn run_server() -> Result<(), Box<dyn Error + Send + Sync>> {
    let rt = tokio::runtime::Runtime::new()?;
    let _ent = rt.enter();

    rt.block_on(async_run_serve())?;
    Ok(())
}

pub fn run(name: &str) -> anyhow::Result<()> {
    let _thread = std::thread::spawn(|| {
        run_server().unwrap();
    });

    let module = get_module(name);
    let mut linker = Linker::new(&ENGINE);

    struct Ctx {
        wasi: WasiCtx,
        http: WasiHttp,
    }

    wasmtime_wasi::sync::add_to_linker(&mut linker, |cx: &mut Ctx| &mut cx.wasi)?;
    wasmtime_wasi_http::add_to_linker(&mut linker, |cx: &mut Ctx| &mut cx.http)?;

    // Create our wasi context.
    let builder = WasiCtxBuilder::new().inherit_stdio().arg(name)?;
    let acl = HttpAcl::builder()
        .clear_allowed_methods()
        .add_allowed_method(HttpRequestMethod::GET)
        .unwrap()
        .add_allowed_method(HttpRequestMethod::POST)
        .unwrap()
        .add_allowed_method(HttpRequestMethod::PUT)
        .unwrap()
        .add_allowed_host("localhost".to_string())
        .unwrap()
        .add_allowed_port_range(3000..=3000)
        .unwrap()
        .add_allowed_ip_range(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)).into())
        .unwrap()
        .add_allowed_ip_range(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)).into())
        .unwrap()
        .build();
    let wasi_http = WasiHttp::new_with_acl(acl);

    let mut store = Store::new(
        &ENGINE,
        Ctx {
            wasi: builder.build(),
            http: wasi_http,
        },
    );

    let instance = linker.instantiate(&mut store, &module)?;
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    start.call(&mut store, ())
}

#[test_log::test]
fn outbound_request() {
    run("outbound_request").unwrap()
}

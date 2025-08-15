use bytes::Bytes;
use http_body::Body;
use http_body_util::{Collected, Empty};
use wasmtime::Store;
use wasmtime::component::{Component, Linker};
use wasmtime_wasi_http::p3::bindings::Proxy;
use wasmtime_wasi_http::p3::bindings::http::types::ErrorCode;

use super::Ctx;

#[expect(unused)] // TODO: implement
pub async fn run_wasi_http<E: Into<ErrorCode> + 'static>(
    component_filename: &str,
    req: http::Request<impl Body<Data = Bytes, Error = E> + Send + Sync + 'static>,
) -> anyhow::Result<Result<http::Response<Collected<Bytes>>, Option<ErrorCode>>> {
    let engine = test_programs_artifacts::engine(|config| {
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.async_support(true);
        config.wasm_component_model_async(true);
    });
    let component = Component::from_file(&engine, component_filename)?;

    let mut store = Store::new(&engine, Ctx::default());

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    wasmtime_wasi_http::p3::add_to_linker(&mut linker)?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let proxy = Proxy::new(&mut store, &instance)?;
    todo!("not implemented yet")
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test)]
async fn wasi_http_proxy_tests() -> anyhow::Result<()> {
    let req = http::Request::builder()
        .uri("http://example.com:8080/test-path")
        .method(http::Method::GET);

    let resp = run_wasi_http(
        test_programs_artifacts::P3_API_PROXY_COMPONENT,
        req.body(Empty::new())?,
    )
    .await?;

    match resp {
        Ok(resp) => println!("response: {resp:?}"),
        Err(e) => panic!("Error given in response: {e:?}"),
    };

    Ok(())
}

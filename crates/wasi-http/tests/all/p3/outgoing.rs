use super::*;
use crate::http_server::Server;
use anyhow::{Context as _, anyhow};
use test_programs_artifacts::*;
use wasmtime_wasi::p3::bindings::Command;

foreach_p3_http!(assert_test_exists);

use super::proxy::{p3_http_echo, p3_http_middleware, p3_http_middleware_with_chain};

async fn run(path: &str, server: &Server) -> anyhow::Result<()> {
    let engine = test_programs_artifacts::engine(|config| {
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.async_support(true);
        config.wasm_component_model_async(true);
    });
    let component = Component::from_file(&engine, path)?;
    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: wasmtime_wasi::WasiCtx::builder()
                .env("HTTP_SERVER", server.addr())
                .build(),
            ..Ctx::default()
        },
    );
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)
        .context("failed to link `wasi:cli@0.2.x`")?;
    wasmtime_wasi::p3::add_to_linker(&mut linker).context("failed to link `wasi:cli@0.3.x`")?;
    wasmtime_wasi_http::p3::add_to_linker(&mut linker)?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let command = Command::new(&mut store, &instance)?;
    instance
        .run_concurrent(store, async |store| {
            command.wasi_cli_run().call_run(store).await
        })
        .await
        .context("failed to call `wasi:cli/run#run`")?
        .context("guest trapped")?
        .map_err(|()| anyhow!("`wasi:cli/run#run` failed"))
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_get() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run(P3_HTTP_OUTBOUND_REQUEST_GET_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_timeout() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run(P3_HTTP_OUTBOUND_REQUEST_TIMEOUT_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_post() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run(P3_HTTP_OUTBOUND_REQUEST_POST_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_large_post() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run(P3_HTTP_OUTBOUND_REQUEST_LARGE_POST_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_put() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run(P3_HTTP_OUTBOUND_REQUEST_PUT_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_invalid_version() -> anyhow::Result<()> {
    let server = Server::http2(1)?;
    run(P3_HTTP_OUTBOUND_REQUEST_INVALID_VERSION_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_invalid_header() -> anyhow::Result<()> {
    let server = Server::http2(1)?;
    run(P3_HTTP_OUTBOUND_REQUEST_INVALID_HEADER_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_unknown_method() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run(P3_HTTP_OUTBOUND_REQUEST_UNKNOWN_METHOD_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_unsupported_scheme() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run(
        P3_HTTP_OUTBOUND_REQUEST_UNSUPPORTED_SCHEME_COMPONENT,
        &server,
    )
    .await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_invalid_port() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run(P3_HTTP_OUTBOUND_REQUEST_INVALID_PORT_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_invalid_dnsname() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run(P3_HTTP_OUTBOUND_REQUEST_INVALID_DNSNAME_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_response_build() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run(P3_HTTP_OUTBOUND_REQUEST_RESPONSE_BUILD_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_content_length() -> anyhow::Result<()> {
    let server = Server::http1(3)?;
    run(P3_HTTP_OUTBOUND_REQUEST_CONTENT_LENGTH_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_missing_path_and_query() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run(
        P3_HTTP_OUTBOUND_REQUEST_MISSING_PATH_AND_QUERY_COMPONENT,
        &server,
    )
    .await
}

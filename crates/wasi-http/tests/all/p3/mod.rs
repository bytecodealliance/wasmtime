use crate::http_server::Server;
use anyhow::Result;
use anyhow::{Context as _, anyhow};
use bytes::Bytes;
use flate2::Compression;
use flate2::write::{DeflateDecoder, DeflateEncoder};
use futures::SinkExt;
use http::HeaderValue;
use http_body::Body;
use http_body_util::{BodyExt as _, Collected, Empty};
use std::io::Write;
use std::path::Path;
use test_programs_artifacts::*;
use tokio::{fs, spawn};
use wasm_compose::composer::ComponentComposer;
use wasm_compose::config::{Config, Dependency, Instantiation, InstantiationArg};
use wasmtime::Store;
use wasmtime::component::{AccessorTask as _, Component, Linker, ResourceTable};
use wasmtime_wasi::p3::bindings::Command;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_http::p3::bindings::Proxy;
use wasmtime_wasi_http::p3::bindings::http::types::ErrorCode;
use wasmtime_wasi_http::p3::{DefaultWasiHttpCtx, WasiHttpCtxView, WasiHttpView};

foreach_p3_http!(assert_test_exists);

struct Ctx {
    table: ResourceTable,
    wasi: WasiCtx,
    http: DefaultWasiHttpCtx,
}

impl Default for Ctx {
    fn default() -> Self {
        Self {
            table: ResourceTable::default(),
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            http: DefaultWasiHttpCtx::default(),
        }
    }
}

impl WasiView for Ctx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for Ctx {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
        }
    }
}

async fn run_cli(path: &str, server: &Server) -> anyhow::Result<()> {
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
    wasmtime_wasi_http::p3::add_to_linker(&mut linker)
        .context("failed to link `wasi:http@0.3.x`")?;
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

async fn run_http<E: Into<ErrorCode> + 'static>(
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
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)
        .context("failed to link `wasi:cli@0.2.x`")?;
    wasmtime_wasi::p3::add_to_linker(&mut linker).context("failed to link `wasi:cli@0.3.x`")?;
    wasmtime_wasi_http::p3::add_to_linker(&mut linker)
        .context("failed to link `wasi:http@0.3.x`")?;
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let proxy = Proxy::new(&mut store, &instance)?;
    let res = match instance
        .run_concurrent(&mut store, async |store| proxy.handle(store, req).await)
        .await??
    {
        Ok(res) => res,
        Err(err) => return Ok(Err(Some(err))),
    };
    let (res, io) = res.into_http()?;
    let (parts, body) = res.into_parts();
    let body = spawn(body.collect());
    if let Some(io) = io {
        let io = io.consume(async { Ok(()) });
        instance
            .run_concurrent(store, async |store| io.run(store).await)
            .await??;
    }
    let body = body
        .await
        .context("failed to join task")?
        .context("failed to collect body")?;
    Ok(Ok(http::Response::from_parts(parts, body)))
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_get() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_GET_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_timeout() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_TIMEOUT_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_post() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_POST_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_large_post() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_LARGE_POST_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_put() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_PUT_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_invalid_version() -> anyhow::Result<()> {
    let server = Server::http2(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_INVALID_VERSION_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_invalid_header() -> anyhow::Result<()> {
    let server = Server::http2(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_INVALID_HEADER_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_unknown_method() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_UNKNOWN_METHOD_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_unsupported_scheme() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(
        P3_HTTP_OUTBOUND_REQUEST_UNSUPPORTED_SCHEME_COMPONENT,
        &server,
    )
    .await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_invalid_port() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_INVALID_PORT_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_invalid_dnsname() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_INVALID_DNSNAME_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_response_build() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_RESPONSE_BUILD_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_content_length() -> anyhow::Result<()> {
    let server = Server::http1(3)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_CONTENT_LENGTH_COMPONENT, &server).await
}

#[ignore = "unimplemented"] // TODO: implement
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_missing_path_and_query() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(
        P3_HTTP_OUTBOUND_REQUEST_MISSING_PATH_AND_QUERY_COMPONENT,
        &server,
    )
    .await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn wasi_http_proxy_tests() -> anyhow::Result<()> {
    let req = http::Request::builder()
        .uri("http://example.com:8080/test-path")
        .method(http::Method::GET);

    let res = run_http(P3_API_PROXY_COMPONENT, req.body(Empty::new())?).await?;

    match res {
        Ok(res) => println!("response: {res:?}"),
        Err(err) => panic!("Error given in response: {err:?}"),
    };

    Ok(())
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_echo() -> Result<()> {
    test_http_echo(P3_HTTP_ECHO_COMPONENT, false).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_middleware() -> Result<()> {
    let tempdir = tempfile::tempdir()?;
    let echo = &fs::read(P3_HTTP_ECHO_COMPONENT).await?;
    let middleware = &fs::read(P3_HTTP_MIDDLEWARE_COMPONENT).await?;

    let path = tempdir.path().join("temp.wasm");
    fs::write(&path, compose(middleware, echo).await?).await?;
    test_http_echo(&path.to_str().unwrap(), true).await
}

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

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_middleware_with_chain() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("temp.wasm");

    fs::copy(P3_HTTP_ECHO_COMPONENT, &dir.path().join("chain-http.wasm")).await?;

    let bytes = ComponentComposer::new(
        Path::new(P3_HTTP_MIDDLEWARE_WITH_CHAIN_COMPONENT),
        &Config {
            dir: dir.path().to_owned(),
            definitions: Vec::new(),
            search_paths: Vec::new(),
            skip_validation: false,
            import_components: false,
            disallow_imports: false,
            dependencies: [(
                "local:local/chain-http".to_owned(),
                Dependency {
                    path: P3_HTTP_ECHO_COMPONENT.into(),
                },
            )]
            .into_iter()
            .collect(),
            instantiations: [(
                "root".to_owned(),
                Instantiation {
                    dependency: Some("local:local/chain-http".to_owned()),
                    arguments: [(
                        "local:local/chain-http".to_owned(),
                        InstantiationArg {
                            instance: "local:local/chain-http".into(),
                            export: Some("wasi:http/handler@0.3.0-rc-2025-08-15".into()),
                        },
                    )]
                    .into_iter()
                    .collect(),
                },
            )]
            .into_iter()
            .collect(),
        },
    )
    .compose()?;
    fs::write(&path, &bytes).await?;

    test_http_echo(&path.to_str().unwrap(), true).await
}

async fn test_http_echo(component: &str, use_compression: bool) -> Result<()> {
    let body = b"And the mome raths outgrabe";

    // Prepare the raw body, optionally compressed if that's what we're
    // testing.
    let raw_body = if use_compression {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(body).unwrap();
        Bytes::from(encoder.finish().unwrap())
    } else {
        Bytes::copy_from_slice(body)
    };

    // Prepare the http_body body, modeled here as a channel with the body
    // chunk above buffered up followed by some trailers. Note that trailers
    // are always here to test that code paths throughout the components.
    let (mut body_tx, body_rx) = futures::channel::mpsc::channel::<Result<_, ErrorCode>>(2);
    body_tx
        .send(Ok(http_body::Frame::data(raw_body)))
        .await
        .unwrap();
    body_tx
        .send(Ok(http_body::Frame::trailers({
            let mut trailers = http::HeaderMap::new();
            assert!(
                trailers
                    .insert("fizz", http::HeaderValue::from_static("buzz"))
                    .is_none()
            );
            trailers
        })))
        .await
        .unwrap();

    // Build the `http::Request`, optionally specifying compression-related
    // headers.
    let mut request = http::Request::builder()
        .uri("http://localhost/")
        .method(http::Method::GET)
        .header("foo", "bar");
    if use_compression {
        request = request
            .header("content-encoding", "deflate")
            .header("accept-encoding", "nonexistent-encoding, deflate");
    }

    // Send this request to wasm and assert that success comes back.
    //
    // Note that this will read the entire body internally and wait for
    // everything to get collected before proceeding to below.
    let response = run_http(
        component,
        request.body(http_body_util::StreamBody::new(body_rx))?,
    )
    .await?
    .unwrap();
    assert!(response.status().as_u16() == 200);

    // Our input header should be echo'd back.
    assert_eq!(
        response.headers().get("foo"),
        Some(&HeaderValue::from_static("bar"))
    );

    // The compression headers should be set if `use_compression` was turned
    // on.
    if use_compression {
        assert_eq!(
            response.headers().get("content-encoding"),
            Some(&HeaderValue::from_static("deflate"))
        );
        assert!(response.headers().get("content-length").is_none());
    }

    // Trailers should be echo'd back as well.
    let trailers = response.body().trailers().expect("trailers missing");
    assert_eq!(
        trailers.get("fizz"),
        Some(&HeaderValue::from_static("buzz"))
    );

    // And our body should match our original input body as well.
    let (_, collected_body) = response.into_parts();
    let collected_body = collected_body.to_bytes();

    let response_body = if use_compression {
        let mut decoder = DeflateDecoder::new(Vec::new());
        decoder.write_all(&collected_body)?;
        decoder.finish()?
    } else {
        collected_body.to_vec()
    };
    assert_eq!(response_body, body.as_slice());
    Ok(())
}

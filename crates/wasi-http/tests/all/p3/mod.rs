use crate::http_server::Server;
use anyhow::Result;
use anyhow::{Context as _, anyhow};
use bytes::Bytes;
use flate2::Compression;
use flate2::write::{DeflateDecoder, DeflateEncoder};
use futures::SinkExt;
use futures::channel::oneshot;
use http::HeaderValue;
use http_body::Body;
use http_body_util::{BodyExt as _, Collected, Empty, combinators::UnsyncBoxBody};
use std::io::Write;
use std::path::Path;
use test_programs_artifacts::*;
use tokio::{fs, try_join};
use wasm_compose::composer::ComponentComposer;
use wasm_compose::config::{Config, Dependency, Instantiation, InstantiationArg};
use wasmtime::Store;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime_wasi::p3::bindings::Command;
use wasmtime_wasi::{TrappableError, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_http::p3::bindings::Proxy;
use wasmtime_wasi_http::p3::bindings::http::types::ErrorCode;
use wasmtime_wasi_http::p3::{
    self, Request, RequestOptions, WasiHttpCtx, WasiHttpCtxView, WasiHttpView,
};
use wasmtime_wasi_http::types::DEFAULT_FORBIDDEN_HEADERS;

foreach_p3_http!(assert_test_exists);

struct TestHttpCtx {
    request_body_tx: Option<oneshot::Sender<UnsyncBoxBody<Bytes, ErrorCode>>>,
}

impl WasiHttpCtx for TestHttpCtx {
    fn is_forbidden_header(&mut self, name: &http::header::HeaderName) -> bool {
        name.as_str() == "custom-forbidden-header" || DEFAULT_FORBIDDEN_HEADERS.contains(name)
    }

    fn send_request(
        &mut self,
        request: http::Request<UnsyncBoxBody<Bytes, ErrorCode>>,
        options: Option<RequestOptions>,
        fut: Box<dyn Future<Output = Result<(), ErrorCode>> + Send>,
    ) -> Box<
        dyn Future<
                Output = Result<
                    (
                        http::Response<UnsyncBoxBody<Bytes, ErrorCode>>,
                        Box<dyn Future<Output = Result<(), ErrorCode>> + Send>,
                    ),
                    TrappableError<ErrorCode>,
                >,
            > + Send,
    > {
        _ = fut;
        if let Some("p3-test") = request.uri().authority().map(|v| v.as_str()) {
            _ = self
                .request_body_tx
                .take()
                .unwrap()
                .send(request.into_body());
            Box::new(async {
                Ok((
                    http::Response::new(Default::default()),
                    Box::new(async { Ok(()) }) as Box<dyn Future<Output = _> + Send>,
                ))
            })
        } else {
            Box::new(async move {
                use http_body_util::BodyExt;

                let (res, io) = p3::default_send_request(request, options).await?;
                Ok((
                    res.map(BodyExt::boxed_unsync),
                    Box::new(io) as Box<dyn Future<Output = _> + Send>,
                ))
            })
        }
    }
}

struct Ctx {
    table: ResourceTable,
    wasi: WasiCtx,
    http: TestHttpCtx,
}

impl Ctx {
    fn new(request_body_tx: oneshot::Sender<UnsyncBoxBody<Bytes, ErrorCode>>) -> Self {
        Self {
            table: ResourceTable::default(),
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            http: TestHttpCtx {
                request_body_tx: Some(request_body_tx),
            },
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
            ..Ctx::new(oneshot::channel().0)
        },
    );
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)
        .context("failed to link `wasi:cli@0.2.x`")?;
    wasmtime_wasi::p3::add_to_linker(&mut linker).context("failed to link `wasi:cli@0.3.x`")?;
    wasmtime_wasi_http::p3::add_to_linker(&mut linker)
        .context("failed to link `wasi:http@0.3.x`")?;
    let command = Command::instantiate_async(&mut store, &component, &linker).await?;
    store
        .run_concurrent(async |store| command.wasi_cli_run().call_run(store).await)
        .await
        .context("failed to call `wasi:cli/run#run`")?
        .context("guest trapped")?
        .0
        .map_err(|()| anyhow!("`wasi:cli/run#run` failed"))
}

async fn run_http<E: Into<ErrorCode> + 'static>(
    component_filename: &str,
    req: http::Request<impl Body<Data = Bytes, Error = E> + Send + Sync + 'static>,
    request_body_tx: oneshot::Sender<UnsyncBoxBody<Bytes, ErrorCode>>,
) -> anyhow::Result<Result<http::Response<Collected<Bytes>>, Option<ErrorCode>>> {
    let engine = test_programs_artifacts::engine(|config| {
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.async_support(true);
        config.wasm_component_model_async(true);
    });
    let component = Component::from_file(&engine, component_filename)?;

    let mut store = Store::new(&engine, Ctx::new(request_body_tx));

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)
        .context("failed to link `wasi:cli@0.2.x`")?;
    wasmtime_wasi::p3::add_to_linker(&mut linker).context("failed to link `wasi:cli@0.3.x`")?;
    wasmtime_wasi_http::p3::add_to_linker(&mut linker)
        .context("failed to link `wasi:http@0.3.x`")?;
    let proxy = Proxy::instantiate_async(&mut store, &component, &linker).await?;
    let (req, io) = Request::from_http(req);
    let (tx, rx) = tokio::sync::oneshot::channel();
    let ((handle_result, ()), res) = try_join!(
        async move {
            store
                .run_concurrent(async |store| {
                    try_join!(
                        async {
                            let (res, task) = match proxy.handle(store, req).await? {
                                Ok(pair) => pair,
                                Err(err) => return Ok(Err(Some(err))),
                            };
                            _ = tx
                                .send(store.with(|store| res.into_http(store, async { Ok(()) }))?);
                            task.block(store).await;
                            Ok(Ok(()))
                        },
                        async { io.await.context("failed to consume request body") }
                    )
                })
                .await?
        },
        async move {
            let res = rx.await?;
            let (parts, body) = res.into_parts();
            let body = body.collect().await.context("failed to collect body")?;
            anyhow::Ok(http::Response::from_parts(parts, body))
        }
    )?;

    Ok(handle_result.map(|()| res))
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_get() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_GET_COMPONENT, &server).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_timeout() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_TIMEOUT_COMPONENT, &server).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_post() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_POST_COMPONENT, &server).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_large_post() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_LARGE_POST_COMPONENT, &server).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_put() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_PUT_COMPONENT, &server).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_invalid_version() -> anyhow::Result<()> {
    let server = Server::http2(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_INVALID_VERSION_COMPONENT, &server).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_invalid_header() -> anyhow::Result<()> {
    let server = Server::http2(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_INVALID_HEADER_COMPONENT, &server).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_unknown_method() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_UNKNOWN_METHOD_COMPONENT, &server).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_unsupported_scheme() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(
        P3_HTTP_OUTBOUND_REQUEST_UNSUPPORTED_SCHEME_COMPONENT,
        &server,
    )
    .await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_invalid_port() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_INVALID_PORT_COMPONENT, &server).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_invalid_dnsname() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_INVALID_DNSNAME_COMPONENT, &server).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_response_build() -> anyhow::Result<()> {
    let server = Server::http1(1)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_RESPONSE_BUILD_COMPONENT, &server).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_outbound_request_content_length() -> anyhow::Result<()> {
    let server = Server::http1(3)?;
    run_cli(P3_HTTP_OUTBOUND_REQUEST_CONTENT_LENGTH_COMPONENT, &server).await
}

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

    let res = run_http(
        P3_API_PROXY_COMPONENT,
        req.body(Empty::new())?,
        oneshot::channel().0,
    )
    .await?;

    match res {
        Ok(res) => println!("response: {res:?}"),
        Err(err) => panic!("Error given in response: {err:?}"),
    };

    Ok(())
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_echo() -> Result<()> {
    test_http_echo(P3_HTTP_ECHO_COMPONENT, false, false).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_echo_host_to_host() -> Result<()> {
    test_http_echo(P3_HTTP_ECHO_COMPONENT, false, true).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_middleware() -> Result<()> {
    test_http_middleware(false).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_middleware_host_to_host() {
    let error = format!("{:?}", test_http_middleware(true).await.unwrap_err());

    let expected = "cannot read from and write to intra-component future with non-numeric payload";

    assert!(
        error.contains(expected),
        "expected `{expected}`; got `{error}`"
    );
}

async fn test_http_middleware(host_to_host: bool) -> Result<()> {
    let tempdir = tempfile::tempdir()?;
    let echo = &fs::read(P3_HTTP_ECHO_COMPONENT).await?;
    let middleware = &fs::read(P3_HTTP_MIDDLEWARE_COMPONENT).await?;

    let path = tempdir.path().join("temp.wasm");
    fs::write(&path, compose(middleware, echo).await?).await?;
    test_http_echo(&path.to_str().unwrap(), true, host_to_host).await
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
    test_http_middleware_with_chain(false).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_middleware_with_chain_host_to_host() -> Result<()> {
    test_http_middleware_with_chain(true).await
}

async fn test_http_middleware_with_chain(host_to_host: bool) -> Result<()> {
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
                            export: Some("wasi:http/handler@0.3.0-rc-2025-09-16".into()),
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

    test_http_echo(&path.to_str().unwrap(), true, host_to_host).await
}

async fn test_http_echo(component: &str, use_compression: bool, host_to_host: bool) -> Result<()> {
    _ = env_logger::try_init();

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
    let (mut body_tx, body_rx) = futures::channel::mpsc::channel::<Result<_, ErrorCode>>(1);

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
    if host_to_host {
        request = request.header("x-host-to-host", "true");
    }

    // Send this request to wasm and assert that success comes back.
    //
    // Note that this will read the entire body internally and wait for
    // everything to get collected before proceeding to below.
    let response = futures::join!(
        run_http(
            component,
            request.body(http_body_util::StreamBody::new(body_rx))?,
            oneshot::channel().0
        ),
        async {
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
            drop(body_tx);
        }
    )
    .0?
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

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_http_proxy() -> Result<()> {
    let body = b"And the mome raths outgrabe";

    let raw_body = Bytes::copy_from_slice(body);

    let (mut body_tx, body_rx) = futures::channel::mpsc::channel::<Result<_, ErrorCode>>(1);

    // Tell the guest to forward the request to `http://p3-test/`, which we
    // handle specially in `TestHttpCtx::send_request` above, sending the
    // request body to the oneshot sender we specify below and then immediately
    // returning a dummy response.  We won't start sending the request body
    // until after the guest has exited and we've dropped the store.

    let request = http::Request::builder()
        .uri("http://localhost/")
        .method(http::Method::GET)
        .header("url", "http://p3-test/");

    let (request_body_tx, request_body_rx) = oneshot::channel();
    let response = run_http(
        P3_HTTP_PROXY_COMPONENT,
        request.body(http_body_util::StreamBody::new(body_rx))?,
        request_body_tx,
    )
    .await?
    .unwrap();
    assert!(response.status().as_u16() == 200);

    // The guest has exited and the store has been dropped; now we finally send
    // the request body and assert that we've received the entire thing.

    let ((), request_body) = futures::join!(
        async {
            body_tx
                .send(Ok(http_body::Frame::data(raw_body)))
                .await
                .unwrap();
            drop(body_tx);
        },
        async {
            request_body_rx
                .await
                .unwrap()
                .collect()
                .await
                .unwrap()
                .to_bytes()
        }
    );

    assert_eq!(request_body, body.as_slice());
    Ok(())
}

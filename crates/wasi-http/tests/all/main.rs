use crate::http_server::Server;
use anyhow::{anyhow, Context, Result};
use futures::{channel::oneshot, future, stream, FutureExt};
use http_body::Frame;
use http_body_util::{combinators::BoxBody, BodyExt, Collected, Empty, StreamBody};
use hyper::{body::Bytes, server::conn::http1, service::service_fn, Method, StatusCode};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, iter, net::Ipv4Addr, str, sync::Arc};
use tokio::task;
use wasmtime::{
    component::{Component, Linker, ResourceTable},
    Config, Engine, Store,
};
use wasmtime_wasi::{self, pipe::MemoryOutputPipe, WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::{
    bindings::http::types::ErrorCode,
    body::HyperOutgoingBody,
    io::TokioIo,
    types::{self, HostFutureIncomingResponse, IncomingResponse, OutgoingRequestConfig},
    HttpResult, WasiHttpCtx, WasiHttpView,
};

mod http_server;

type RequestSender = Arc<
    dyn Fn(hyper::Request<HyperOutgoingBody>, OutgoingRequestConfig) -> HostFutureIncomingResponse
        + Send
        + Sync,
>;

struct Ctx {
    table: ResourceTable,
    wasi: WasiCtx,
    http: WasiHttpCtx,
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
    send_request: Option<RequestSender>,
    rejected_authority: Option<String>,
}

impl WasiView for Ctx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl WasiHttpView for Ctx {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn send_request(
        &mut self,
        request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> HttpResult<HostFutureIncomingResponse> {
        if let Some(rejected_authority) = &self.rejected_authority {
            let authority = request.uri().authority().map(ToString::to_string).unwrap();
            if &authority == rejected_authority {
                return Err(ErrorCode::HttpRequestDenied.into());
            }
        }
        if let Some(send_request) = self.send_request.clone() {
            Ok(send_request(request, config))
        } else {
            Ok(types::default_send_request(request, config))
        }
    }

    fn is_forbidden_header(&mut self, name: &hyper::header::HeaderName) -> bool {
        name.as_str() == "custom-forbidden-header"
    }
}

fn store(engine: &Engine, server: &Server) -> Store<Ctx> {
    let stdout = MemoryOutputPipe::new(4096);
    let stderr = MemoryOutputPipe::new(4096);

    // Create our wasi context.
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    builder.stderr(stderr.clone());
    builder.env("HTTP_SERVER", server.addr().to_string());
    let ctx = Ctx {
        table: ResourceTable::new(),
        wasi: builder.build(),
        http: WasiHttpCtx::new(),
        stderr,
        stdout,
        send_request: None,
        rejected_authority: None,
    };

    Store::new(&engine, ctx)
}

impl Drop for Ctx {
    fn drop(&mut self) {
        let stdout = self.stdout.contents();
        if !stdout.is_empty() {
            println!("[guest] stdout:\n{}\n===", String::from_utf8_lossy(&stdout));
        }
        let stderr = self.stderr.contents();
        if !stderr.is_empty() {
            println!("[guest] stderr:\n{}\n===", String::from_utf8_lossy(&stderr));
        }
    }
}

// Assert that each of `sync` and `async` below are testing everything through
// assertion of the existence of the test function itself.
macro_rules! assert_test_exists {
    ($name:ident) => {
        #[allow(unused_imports)]
        use self::$name as _;
    };
}

mod async_;
mod sync;

async fn run_wasi_http(
    component_filename: &str,
    req: hyper::Request<BoxBody<Bytes, hyper::Error>>,
    send_request: Option<RequestSender>,
    rejected_authority: Option<String>,
) -> anyhow::Result<Result<hyper::Response<Collected<Bytes>>, ErrorCode>> {
    let stdout = MemoryOutputPipe::new(4096);
    let stderr = MemoryOutputPipe::new(4096);
    let table = ResourceTable::new();

    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, component_filename)?;

    // Create our wasi context.
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    builder.stderr(stderr.clone());
    let wasi = builder.build();
    let http = WasiHttpCtx::new();
    let ctx = Ctx {
        table,
        wasi,
        http,
        stderr,
        stdout,
        send_request,
        rejected_authority,
    };
    let mut store = Store::new(&engine, ctx);

    let mut linker = Linker::new(&engine);
    wasmtime_wasi_http::add_to_linker_async(&mut linker)?;
    let proxy =
        wasmtime_wasi_http::bindings::Proxy::instantiate_async(&mut store, &component, &linker)
            .await?;

    let req = store.data_mut().new_incoming_request(req)?;

    let (sender, receiver) = tokio::sync::oneshot::channel();
    let out = store.data_mut().new_response_outparam(sender)?;

    let handle = wasmtime_wasi::runtime::spawn(async move {
        proxy
            .wasi_http_incoming_handler()
            .call_handle(&mut store, req, out)
            .await?;

        Ok::<_, anyhow::Error>(())
    });

    let resp = match receiver.await {
        Ok(Ok(resp)) => {
            use http_body_util::BodyExt;
            let (parts, body) = resp.into_parts();
            let collected = BodyExt::collect(body).await?;
            Some(Ok(hyper::Response::from_parts(parts, collected)))
        }
        Ok(Err(e)) => Some(Err(e)),

        // Fall through below to the `resp.expect(...)` which will hopefully
        // return a more specific error from `handle.await`.
        Err(_) => None,
    };

    // Now that the response has been processed, we can wait on the wasm to
    // finish without deadlocking.
    handle.await.context("Component execution")?;

    Ok(resp.expect("wasm never called set-response-outparam"))
}

#[test_log::test(tokio::test)]
async fn wasi_http_proxy_tests() -> anyhow::Result<()> {
    let req = hyper::Request::builder()
        .header("custom-forbidden-header", "yes")
        .uri("http://example.com:8080/test-path")
        .method(http::Method::GET);

    let resp = run_wasi_http(
        test_programs_artifacts::API_PROXY_COMPONENT,
        req.body(body::empty())?,
        None,
        None,
    )
    .await?;

    match resp {
        Ok(resp) => println!("response: {resp:?}"),
        Err(e) => panic!("Error given in response: {e:?}"),
    };

    Ok(())
}

#[test_log::test(tokio::test)]
async fn wasi_http_hash_all() -> Result<()> {
    do_wasi_http_hash_all(false).await
}

#[test_log::test(tokio::test)]
async fn wasi_http_hash_all_with_override() -> Result<()> {
    do_wasi_http_hash_all(true).await
}

async fn do_wasi_http_hash_all(override_send_request: bool) -> Result<()> {
    let bodies = Arc::new(
        [
            ("/a", "’Twas brillig, and the slithy toves"),
            ("/b", "Did gyre and gimble in the wabe:"),
            ("/c", "All mimsy were the borogoves,"),
            ("/d", "And the mome raths outgrabe."),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>(),
    );

    let listener = tokio::net::TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 0)).await?;

    let prefix = format!("http://{}", listener.local_addr()?);

    let (_tx, rx) = oneshot::channel::<()>();

    let handle = {
        let bodies = bodies.clone();

        move |request: http::request::Parts| {
            if let (Method::GET, Some(body)) = (request.method, bodies.get(request.uri.path())) {
                Ok::<_, anyhow::Error>(hyper::Response::new(body::full(Bytes::copy_from_slice(
                    body.as_bytes(),
                ))))
            } else {
                Ok(hyper::Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .body(body::empty())?)
            }
        }
    };

    let send_request = if override_send_request {
        Some(Arc::new(
            move |request: hyper::Request<HyperOutgoingBody>,
                  OutgoingRequestConfig {
                      between_bytes_timeout,
                      ..
                  }| {
                let response = handle(request.into_parts().0).map(|resp| {
                    Ok(IncomingResponse {
                        resp: resp.map(|body| {
                            body.map_err(wasmtime_wasi_http::hyper_response_error)
                                .boxed()
                        }),
                        worker: None,
                        between_bytes_timeout,
                    })
                });
                HostFutureIncomingResponse::ready(response)
            },
        ) as RequestSender)
    } else {
        let server = async move {
            loop {
                let (stream, _) = listener.accept().await?;
                let stream = TokioIo::new(stream);
                let handle = handle.clone();
                task::spawn(async move {
                    if let Err(e) = http1::Builder::new()
                        .keep_alive(true)
                        .serve_connection(
                            stream,
                            service_fn(move |request| {
                                let handle = handle.clone();
                                async move { handle(request.into_parts().0) }
                            }),
                        )
                        .await
                    {
                        eprintln!("error serving connection: {e:?}");
                    }
                });

                // Help rustc with type inference:
                if false {
                    return Ok::<_, anyhow::Error>(());
                }
            }
        }
        .then(|result| {
            if let Err(e) = result {
                eprintln!("error listening for connections: {e:?}");
            }
            future::ready(())
        })
        .boxed();

        task::spawn(async move {
            drop(future::select(server, rx).await);
        });

        None
    };

    let mut request = hyper::Request::builder()
        .method(http::Method::GET)
        .uri("http://example.com:8080/hash-all");
    for path in bodies.keys() {
        request = request.header("url", format!("{prefix}{path}"));
    }
    let request = request.body(body::empty())?;

    let response = run_wasi_http(
        test_programs_artifacts::API_PROXY_STREAMING_COMPONENT,
        request,
        send_request,
        None,
    )
    .await??;

    assert_eq!(StatusCode::OK, response.status());
    let body = response.into_body().to_bytes();
    let body = str::from_utf8(&body)?;
    for line in body.lines() {
        let (url, hash) = line
            .split_once(": ")
            .ok_or_else(|| anyhow!("expected string of form `<url>: <sha-256>`; got {line}"))?;

        let path = url
            .strip_prefix(&prefix)
            .ok_or_else(|| anyhow!("expected string with prefix {prefix}; got {url}"))?;

        let mut hasher = Sha256::new();
        hasher.update(
            bodies
                .get(path)
                .ok_or_else(|| anyhow!("unexpected path: {path}"))?,
        );

        use base64::Engine;
        assert_eq!(
            hash,
            base64::engine::general_purpose::STANDARD_NO_PAD.encode(hasher.finalize())
        );
    }

    Ok(())
}

// ensure the runtime rejects the outgoing request
#[test_log::test(tokio::test)]
async fn wasi_http_hash_all_with_reject() -> Result<()> {
    let request = hyper::Request::builder()
        .method(http::Method::GET)
        .uri("http://example.com:8080/hash-all");
    let request = request.header("url", format!("http://forbidden.com"));
    let request = request.header("url", format!("http://localhost"));
    let request = request.body(body::empty())?;

    let response = run_wasi_http(
        test_programs_artifacts::API_PROXY_STREAMING_COMPONENT,
        request,
        None,
        Some("forbidden.com".to_string()),
    )
    .await??;

    let body = response.into_body().to_bytes();
    let body = str::from_utf8(&body).unwrap();
    for line in body.lines() {
        println!("{}", line);
        if line.contains("forbidden.com") {
            assert!(line.contains("HttpRequestDenied"));
        }
        if line.contains("localhost") {
            assert!(!line.contains("HttpRequestDenied"));
        }
    }

    Ok(())
}

#[test_log::test(tokio::test)]
async fn wasi_http_echo() -> Result<()> {
    do_wasi_http_echo("echo", None).await
}

#[test_log::test(tokio::test)]
async fn wasi_http_double_echo() -> Result<()> {
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 0)).await?;

    let prefix = format!("http://{}", listener.local_addr()?);

    let (_tx, rx) = oneshot::channel::<()>();

    let server = async move {
        loop {
            let (stream, _) = listener.accept().await?;
            let stream = TokioIo::new(stream);
            task::spawn(async move {
                if let Err(e) = http1::Builder::new()
                    .keep_alive(true)
                    .serve_connection(
                        stream,
                        service_fn(
                            move |request: hyper::Request<hyper::body::Incoming>| async move {
                                use http_body_util::BodyExt;

                                if let (&Method::POST, "/echo") =
                                    (request.method(), request.uri().path())
                                {
                                    Ok::<_, anyhow::Error>(hyper::Response::new(
                                        request.into_body().boxed(),
                                    ))
                                } else {
                                    Ok(hyper::Response::builder()
                                        .status(StatusCode::METHOD_NOT_ALLOWED)
                                        .body(BoxBody::new(
                                            Empty::new().map_err(|_| unreachable!()),
                                        ))?)
                                }
                            },
                        ),
                    )
                    .await
                {
                    eprintln!("error serving connection: {e:?}");
                }
            });

            // Help rustc with type inference:
            if false {
                return Ok::<_, anyhow::Error>(());
            }
        }
    }
    .then(|result| {
        if let Err(e) = result {
            eprintln!("error listening for connections: {e:?}");
        }
        future::ready(())
    })
    .boxed();

    task::spawn(async move {
        drop(future::select(server, rx).await);
    });

    do_wasi_http_echo("double-echo", Some(&format!("{prefix}/echo"))).await
}

async fn do_wasi_http_echo(uri: &str, url_header: Option<&str>) -> Result<()> {
    let body = {
        // A sorta-random-ish megabyte
        let mut n = 0_u8;
        iter::repeat_with(move || {
            n = n.wrapping_add(251);
            n
        })
        .take(1024 * 1024)
        .collect::<Vec<_>>()
    };

    let mut request = hyper::Request::builder()
        .method(http::Method::POST)
        .uri(format!("http://example.com:8080/{uri}"))
        .header("content-type", "application/octet-stream");

    if let Some(url_header) = url_header {
        request = request.header("url", url_header);
    }

    let request = request.body(BoxBody::new(StreamBody::new(stream::iter(
        body.chunks(16 * 1024)
            .map(|chunk| Ok::<_, hyper::Error>(Frame::data(Bytes::copy_from_slice(chunk))))
            .collect::<Vec<_>>(),
    ))))?;

    let response = run_wasi_http(
        test_programs_artifacts::API_PROXY_STREAMING_COMPONENT,
        request,
        None,
        None,
    )
    .await??;

    assert_eq!(StatusCode::OK, response.status());
    assert_eq!(
        response.headers()["content-type"],
        "application/octet-stream"
    );
    let received = Vec::from(response.into_body().to_bytes());
    if body != received {
        panic!(
            "body content mismatch (expected length {}; actual length {})",
            body.len(),
            received.len()
        );
    }

    Ok(())
}

#[test_log::test(tokio::test)]
// test uses TLS but riscv/s390x don't support that yet
#[cfg_attr(any(target_arch = "riscv64", target_arch = "s390x"), ignore)]
async fn wasi_http_without_port() -> Result<()> {
    let req = hyper::Request::builder()
        .method(http::Method::GET)
        .uri("https://httpbin.org/get");

    let _response: hyper::Response<_> = run_wasi_http(
        test_programs_artifacts::API_PROXY_FORWARD_REQUEST_COMPONENT,
        req.body(body::empty())?,
        None,
        None,
    )
    .await??;

    // NB: don't test the actual return code of `response`. This is testing a
    // live http request against a live server and things happen. If we got this
    // far it's already successful that the request was made and the lack of
    // port in the URI was handled.

    Ok(())
}

mod body {
    use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
    use hyper::body::Bytes;
    use hyper::Error;

    pub fn full(bytes: Bytes) -> BoxBody<Bytes, Error> {
        BoxBody::new(Full::new(bytes).map_err(|_| unreachable!()))
    }

    pub fn empty() -> BoxBody<Bytes, Error> {
        BoxBody::new(Empty::new().map_err(|_| unreachable!()))
    }
}

use anyhow::Context;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{body::Bytes, service::service_fn, Request, Response};
use std::{
    future::Future,
    net::{SocketAddr, TcpListener},
    sync::{mpsc, OnceLock},
    time::Duration,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(50);

async fn test(
    mut req: Request<hyper::body::Incoming>,
) -> http::Result<Response<BoxBody<Bytes, std::convert::Infallible>>> {
    tracing::debug!("preparing mocked response",);
    let method = req.method().to_string();
    let body = req.body_mut().collect().await.unwrap();
    let buf = body.to_bytes();
    tracing::trace!("hyper request body size {:?}", buf.len());

    Response::builder()
        .status(http::StatusCode::OK)
        .header("x-wasmtime-test-method", method)
        .header("x-wasmtime-test-uri", req.uri().to_string())
        .body(Full::<Bytes>::from(buf).boxed())
}

struct ServerHttp1 {
    receiver: mpsc::Receiver<anyhow::Result<()>>,
}

impl ServerHttp1 {
    fn new() -> Self {
        tracing::debug!("initializing http1 server");
        static CELL_HTTP1: OnceLock<TcpListener> = OnceLock::new();
        let listener = CELL_HTTP1.get_or_init(|| {
            let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
            tracing::debug!("preparing tcp listener at localhost:3000");
            TcpListener::bind(addr).unwrap()
        });
        let (sender, receiver) = mpsc::channel::<anyhow::Result<()>>();
        std::thread::spawn(move || {
            tracing::debug!("dedicated thread to start listening");
            match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => {
                    tracing::debug!("using tokio runtime");
                    sender
                        .send(rt.block_on(async move {
                            tracing::debug!("preparing to accept connection");
                            let (stream, _) = listener.accept().map_err(anyhow::Error::from)?;
                            tracing::trace!("tcp stream {:?}", stream);

                            let mut builder = hyper::server::conn::http1::Builder::new();
                            let http = builder.keep_alive(false).pipeline_flush(true);
                            let io = tokio::net::TcpStream::from_std(stream)
                                .map_err(anyhow::Error::from)?;

                            tracing::debug!("preparing to bind connection to service");
                            let conn = http.serve_connection(io, service_fn(test)).await;
                            tracing::trace!("connection result {:?}", conn);
                            conn.map_err(anyhow::Error::from)
                        }))
                        .expect("value sent from http1 server dedicated thread");
                }
                Err(e) => {
                    tracing::debug!("unable to start tokio runtime");
                    sender.send(Err(anyhow::Error::from(e))).unwrap()
                }
            };
        });
        Self { receiver }
    }

    fn shutdown(self) -> anyhow::Result<()> {
        tracing::debug!("shutting down http1 server");
        self.receiver
            .recv_timeout(DEFAULT_TIMEOUT)
            .context("value received from http1 server dedicated thread")?
    }
}

#[derive(Clone)]
/// An Executor that uses the tokio runtime.
pub struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}

struct ServerHttp2 {
    receiver: mpsc::Receiver<anyhow::Result<()>>,
}

impl ServerHttp2 {
    fn new() -> Self {
        tracing::debug!("initializing http2 server");
        static CELL_HTTP2: OnceLock<TcpListener> = OnceLock::new();
        let listener = CELL_HTTP2.get_or_init(|| {
            let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
            tracing::debug!("preparing tcp listener at localhost:3001");
            TcpListener::bind(addr).unwrap()
        });
        let (sender, receiver) = mpsc::channel::<anyhow::Result<()>>();
        std::thread::spawn(move || {
            tracing::debug!("dedicated thread to start listening");
            match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => {
                    tracing::debug!("using tokio runtime");
                    sender
                        .send(rt.block_on(async move {
                            tracing::debug!("preparing to accept incoming connection");
                            let (stream, _) = listener.accept().map_err(anyhow::Error::from)?;
                            tracing::trace!("tcp stream {:?}", stream);

                            let mut builder =
                                hyper::server::conn::http2::Builder::new(TokioExecutor);
                            let http = builder.max_concurrent_streams(20);
                            let io = tokio::net::TcpStream::from_std(stream)
                                .map_err(anyhow::Error::from)?;

                            tracing::debug!("preparing to bind connection to service");
                            let conn = http.serve_connection(io, service_fn(test)).await;
                            tracing::trace!("connection result {:?}", conn);
                            if let Err(e) = &conn {
                                let message = e.to_string();
                                if message.contains("connection closed before reading preface")
                                    || message.contains("unspecific protocol error detected")
                                {
                                    return Ok(());
                                }
                            }
                            conn.map_err(anyhow::Error::from)
                        }))
                        .expect("value sent from http2 server dedicated thread");
                }
                Err(e) => {
                    tracing::debug!("unable to start tokio runtime");
                    sender.send(Err(anyhow::Error::from(e))).unwrap()
                }
            };
        });
        Self { receiver }
    }

    fn shutdown(self) -> anyhow::Result<()> {
        tracing::debug!("shutting down http2 server");
        self.receiver
            .recv_timeout(DEFAULT_TIMEOUT)
            .context("value received from http2 server dedicated thread")?
    }
}

pub async fn setup_http1(f: impl Future<Output = anyhow::Result<()>>) -> anyhow::Result<()> {
    tracing::debug!("preparing http1 server asynchronously");
    let server = ServerHttp1::new();

    tracing::debug!("running inner function (future)");
    let result = f.await;

    if let Err(err) = server.shutdown() {
        tracing::error!("[host/server] failure {:?}", err);
    }
    result
}

pub fn setup_http1_sync<F>(f: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()> + Send + 'static,
{
    tracing::debug!("preparing http1 server synchronously");
    let server = ServerHttp1::new();

    let (tx, rx) = mpsc::channel::<anyhow::Result<()>>();
    tracing::debug!("running inner function in a dedicated thread");
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    let result = rx
        .recv_timeout(DEFAULT_TIMEOUT)
        .context("value received from request dedicated thread");

    if let Err(err) = server.shutdown() {
        tracing::error!("[host/server] failure {:?}", err);
    }
    result?
}

pub async fn setup_http2(f: impl Future<Output = anyhow::Result<()>>) -> anyhow::Result<()> {
    tracing::debug!("preparing http2 server asynchronously");
    let server = ServerHttp2::new();

    tracing::debug!("running inner function (future)");
    let result = f.await;

    if let Err(err) = server.shutdown() {
        tracing::error!("[host/server] Failure: {:?}", err);
    }
    result
}

pub fn setup_http2_sync<F>(f: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()> + Send + 'static,
{
    tracing::debug!("preparing http2 server synchronously");
    let server = ServerHttp2::new();

    let (tx, rx) = mpsc::channel::<anyhow::Result<()>>();
    tracing::debug!("running inner function in a dedicated thread");
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    let result = rx
        .recv_timeout(DEFAULT_TIMEOUT)
        .context("value received from request dedicated thread");

    if let Err(err) = server.shutdown() {
        tracing::error!("[host/server] failure {:?}", err);
    }
    result?
}

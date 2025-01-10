use anyhow::{Context, Result};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::{Request, Response, body::Bytes, service::service_fn};
use std::{
    future::Future,
    net::{SocketAddr, TcpStream},
    thread::JoinHandle,
};
use tokio::net::TcpListener;
use wasmtime_wasi_http::io::TokioIo;

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

pub struct Server {
    addr: SocketAddr,
    worker: Option<JoinHandle<Result<()>>>,
}

impl Server {
    fn new<F>(
        run: impl FnOnce(TokioIo<tokio::net::TcpStream>) -> F + Send + 'static,
    ) -> Result<Self>
    where
        F: Future<Output = Result<()>>,
    {
        let thread = std::thread::spawn(|| -> Result<_> {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("failed to start tokio runtime")?;
            let listener = rt.block_on(async move {
                let addr = SocketAddr::from(([127, 0, 0, 1], 0));
                TcpListener::bind(addr).await.context("failed to bind")
            })?;
            Ok((rt, listener))
        });
        let (rt, listener) = thread.join().unwrap()?;
        let addr = listener.local_addr().context("failed to get local addr")?;
        let worker = std::thread::spawn(move || {
            tracing::debug!("dedicated thread to start listening");
            rt.block_on(async move {
                tracing::debug!("preparing to accept connection");
                let (stream, _) = listener.accept().await.map_err(anyhow::Error::from)?;
                run(TokioIo::new(stream)).await
            })
        });
        Ok(Self {
            worker: Some(worker),
            addr,
        })
    }

    pub fn http1() -> Result<Self> {
        tracing::debug!("initializing http1 server");
        Self::new(|io| async move {
            let mut builder = hyper::server::conn::http1::Builder::new();
            let http = builder.keep_alive(false).pipeline_flush(true);

            tracing::debug!("preparing to bind connection to service");
            let conn = http.serve_connection(io, service_fn(test)).await;
            tracing::trace!("connection result {:?}", conn);
            conn?;
            Ok(())
        })
    }

    pub fn http2() -> Result<Self> {
        tracing::debug!("initializing http2 server");
        Self::new(|io| async move {
            let mut builder = hyper::server::conn::http2::Builder::new(TokioExecutor);
            let http = builder.max_concurrent_streams(20);

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
            conn?;
            Ok(())
        })
    }

    pub fn addr(&self) -> String {
        format!("localhost:{}", self.addr.port())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        tracing::debug!("shutting down http1 server");
        // Force a connection to happen in case one hasn't happened already.
        let _ = TcpStream::connect(&self.addr);

        // If the worker fails with an error, report it here but don't panic.
        // Some tests don't make a connection so the error will be that the tcp
        // stream created above is closed immediately afterwards. Let the test
        // independently decide if it failed or not, and this should be in the
        // logs to assist with debugging if necessary.
        let worker = self.worker.take().unwrap();
        if let Err(e) = worker.join().unwrap() {
            eprintln!("worker failed with error {e:?}");
        }
    }
}

#[derive(Clone)]
/// An Executor that uses the tokio runtime.
struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}

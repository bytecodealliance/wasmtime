use anyhow::{Context, Result};
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{body::Bytes, service::service_fn, Request, Response};
use std::{
    future::Future,
    net::{SocketAddr, TcpListener, TcpStream},
    thread::JoinHandle,
};

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
    fn new<F>(run: impl FnOnce(tokio::net::TcpStream) -> F + Send + Sync + 'static) -> Result<Self>
    where
        F: Future<Output = Result<()>>,
    {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr).context("failed to bind")?;
        let addr = listener.local_addr().context("failed to get local addr")?;

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("failed to start tokio runtime")?;
        let worker = std::thread::spawn(move || {
            tracing::debug!("dedicated thread to start listening");
            rt.block_on(async move {
                tracing::debug!("preparing to accept connection");
                let (stream, _) = listener.accept().map_err(anyhow::Error::from)?;
                let io = tokio::net::TcpStream::from_std(stream).map_err(anyhow::Error::from)?;
                run(io).await
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

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        tracing::debug!("shutting down http1 server");
        // Force a connection to happen in case one hasn't happened already.
        let _ = TcpStream::connect(&self.addr);
        let worker = self.worker.take().unwrap();
        worker.join().unwrap().unwrap();
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

use http::header::CONTENT_LENGTH;
use hyper::service::service_fn;
use hyper::{Request, Response};
use std::future::Future;
use std::net::{SocketAddr, TcpStream};
use std::thread::JoinHandle;
use tokio::net::TcpListener;
use tracing::{debug, trace, warn};
use wasmtime::{Result, error::Context as _};
use wasmtime_wasi_http::io::TokioIo;

async fn test(
    req: Request<hyper::body::Incoming>,
) -> http::Result<Response<hyper::body::Incoming>> {
    debug!(?req, "preparing mocked response for request");
    let method = req.method().to_string();
    let uri = req.uri().to_string();
    let resp = Response::builder()
        .header("x-wasmtime-test-method", method)
        .header("x-wasmtime-test-uri", uri);
    let resp = if let Some(content_length) = req.headers().get(CONTENT_LENGTH) {
        resp.header(CONTENT_LENGTH, content_length)
    } else {
        resp
    };
    let body = req.into_body();
    resp.body(body)
}

pub struct Server {
    conns: usize,
    addr: SocketAddr,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    fn new<F>(
        conns: usize,
        run: impl Fn(TokioIo<tokio::net::TcpStream>) -> F + Send + 'static,
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
            debug!("dedicated thread to start listening");
            rt.block_on(async move {
                for i in 0..conns {
                    debug!(i, "preparing to accept connection");
                    match listener.accept().await {
                        Ok((stream, ..)) => {
                            debug!(i, "accepted connection");
                            if let Err(err) = run(TokioIo::new(stream)).await {
                                warn!(i, ?err, "failed to serve connection");
                            }
                        }
                        Err(err) => {
                            warn!(i, ?err, "failed to accept connection");
                        }
                    };
                }
            })
        });
        Ok(Self {
            conns,
            worker: Some(worker),
            addr,
        })
    }

    pub fn http1(conns: usize) -> Result<Self> {
        debug!("initializing http1 server");
        Self::new(conns, |io| async move {
            let mut builder = hyper::server::conn::http1::Builder::new();
            let http = builder.keep_alive(false).pipeline_flush(true);

            debug!("preparing to bind connection to service");
            let conn = http.serve_connection(io, service_fn(test)).await;
            trace!("connection result {:?}", conn);
            conn?;
            Ok(())
        })
    }

    pub fn http2(conns: usize) -> Result<Self> {
        debug!("initializing http2 server");
        Self::new(conns, |io| async move {
            let mut builder = hyper::server::conn::http2::Builder::new(TokioExecutor);
            let http = builder.max_concurrent_streams(20);

            debug!("preparing to bind connection to service");
            let conn = http.serve_connection(io, service_fn(test)).await;
            trace!("connection result {:?}", conn);
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
        debug!("shutting down http1 server");
        for _ in 0..self.conns {
            // Force a connection to happen in case one hasn't happened already.
            let _ = TcpStream::connect(&self.addr);
        }
        self.worker.take().unwrap().join().unwrap();
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

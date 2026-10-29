use http::header::CONTENT_LENGTH;
use hyper::service::service_fn;
use hyper::{Request, Response};
use std::future::Future;
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
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
        run: impl Fn(tokio::net::TcpStream) -> F + Send + 'static,
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
                            if let Err(err) = run(stream).await {
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
        Self::new(conns, |stream| async move {
            let io = TokioIo::new(stream);
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
        Self::new(conns, |stream| async move {
            let io = TokioIo::new(stream);
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

    /// Test-only: a raw HTTP/1.1 server which doesn't go through `hyper`, for
    /// sending responses `hyper` would refuse to produce.
    ///
    /// For each connection the request head (everything up to and including
    /// the blank line) is read and passed to `respond`, whose return value is
    /// written back verbatim before the connection is closed. Request heads
    /// are recorded and can be retrieved with [`RawRequests::get`]; request
    /// bodies are neither read nor recorded.
    pub fn raw(
        conns: usize,
        respond: impl Fn(&[u8]) -> Vec<u8> + Send + Sync + 'static,
    ) -> Result<(Self, RawRequests)> {
        debug!("initializing raw http1 server");
        let requests = RawRequests::default();
        let recorded = requests.clone();
        let respond = Arc::new(respond);
        let server = Self::new(conns, move |mut stream| {
            let recorded = recorded.clone();
            let respond = respond.clone();
            async move {
                let mut head = Vec::new();
                let mut buf = [0; 1024];
                loop {
                    // Any body bytes read along with the head are dropped.
                    if let Some(end) = head.windows(4).position(|w| w == b"\r\n\r\n") {
                        head.truncate(end + 4);
                        break;
                    }
                    let n = stream.read(&mut buf).await?;
                    if n == 0 {
                        break;
                    }
                    head.extend_from_slice(&buf[..n]);
                }
                // `Drop for Server` connects without sending anything to
                // unblock the accept loop; don't record those.
                if head.is_empty() {
                    return Ok(());
                }
                trace!(head = %String::from_utf8_lossy(&head), "raw request");
                let response = respond(&head);
                recorded.0.lock().unwrap().push(head);
                stream.write_all(&response).await?;
                stream.shutdown().await?;
                Ok(())
            }
        })?;
        Ok((server, requests))
    }

    pub fn addr(&self) -> String {
        format!("localhost:{}", self.addr.port())
    }
}

/// The request heads received by a [`Server::raw`], in order.
#[derive(Clone, Default)]
pub struct RawRequests(Arc<Mutex<Vec<Vec<u8>>>>);

impl RawRequests {
    pub fn get(&self) -> Vec<Vec<u8>> {
        self.0.lock().unwrap().clone()
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

#[cfg(test)]
mod tests {
    use super::Server;
    use std::io::{Read as _, Write as _};
    use std::net::TcpStream;

    #[test]
    fn raw_server_records_and_responds() {
        const RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
            content-length: 5\r\n\
            content-length: 100\r\n\
            transfer-encoding: chunked\r\n\
            \r\n\
            0\r\n\r\n";
        let (server, requests) = Server::raw(2, |head| {
            assert!(head.starts_with(b"GET /dup "));
            RESPONSE.to_vec()
        })
        .unwrap();

        // Send a body in the same write as the head; only the head is recorded.
        let request = b"GET /dup HTTP/1.1\r\nhost: example.com\r\ncontent-length: 4\r\n\r\n";
        let mut stream = TcpStream::connect(server.addr()).unwrap();
        stream.write_all(&[&request[..], b"body"].concat()).unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        assert_eq!(response, RESPONSE);

        // Dropping the server connects once more for the unused connection;
        // that empty connection must not be recorded.
        drop(server);
        assert_eq!(requests.get(), [request.to_vec()]);
    }
}

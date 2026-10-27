use anyhow::{Context, Result};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::{Request, Response, body::Bytes, service::service_fn};
use std::{
    future::Future,
    net::{SocketAddr, TcpStream},
    sync::{Arc, Mutex},
    thread::JoinHandle,
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
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
    conns: usize,
    addr: SocketAddr,
    worker: Option<JoinHandle<Result<()>>>,
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
            tracing::debug!("dedicated thread to start listening");
            rt.block_on(async move {
                for _ in 0..conns {
                    tracing::debug!("preparing to accept connection");
                    let (stream, _) = listener.accept().await.map_err(anyhow::Error::from)?;
                    run(stream).await?;
                }
                Ok(())
            })
        });
        Ok(Self {
            conns,
            worker: Some(worker),
            addr,
        })
    }

    pub fn http1() -> Result<Self> {
        tracing::debug!("initializing http1 server");
        Self::new(1, |stream| async move {
            let io = TokioIo::new(stream);
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
        Self::new(1, |stream| async move {
            let io = TokioIo::new(stream);
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

    /// Test-only: a raw HTTP/1.1 server which doesn't go through `hyper`, for
    /// sending responses `hyper`'s server would refuse to produce.
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
        tracing::debug!("initializing raw http1 server");
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
                tracing::trace!(head = %String::from_utf8_lossy(&head), "raw request");
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

impl Drop for Server {
    fn drop(&mut self) {
        tracing::debug!("shutting down http1 server");
        for _ in 0..self.conns {
            // Force a connection to happen in case one hasn't happened already.
            let _ = TcpStream::connect(&self.addr);
        }

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

/// The request heads received by a [`Server::raw`], in order.
#[derive(Clone, Default)]
pub struct RawRequests(Arc<Mutex<Vec<Vec<u8>>>>);

impl RawRequests {
    pub fn get(&self) -> Vec<Vec<u8>> {
        self.0.lock().unwrap().clone()
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

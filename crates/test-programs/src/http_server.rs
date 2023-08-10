use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, service::service_fn, Request, Response};
use std::net::SocketAddr;
use std::sync::OnceLock;

async fn test(
    mut req: Request<hyper::body::Incoming>,
) -> http::Result<Response<BoxBody<Bytes, std::convert::Infallible>>> {
    let method = req.method().to_string();
    let mut buf: Vec<u8> = Vec::new();
    while let Some(next) = req.body_mut().frame().await {
        let frame = next.unwrap();
        if let Some(chunk) = frame.data_ref() {
            buf.extend_from_slice(chunk);
        }
    }
    Response::builder()
        .status(http::StatusCode::OK)
        .header("x-wasmtime-test-method", method)
        .header("x-wasmtime-test-uri", req.uri().to_string())
        .body(Full::<Bytes>::from(buf).boxed())
}

async fn serve_http1_connection(stream: std::net::TcpStream) -> Result<(), hyper::Error> {
    let mut builder = hyper::server::conn::http1::Builder::new();
    let http = builder.keep_alive(false).pipeline_flush(true);
    let io = tokio::net::TcpStream::from_std(stream).unwrap();
    http.serve_connection(io, service_fn(test)).await
}

#[derive(Clone)]
/// An Executor that uses the tokio runtime.
pub struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}

async fn serve_http2_connection(stream: std::net::TcpStream) -> Result<(), hyper::Error> {
    let mut builder = hyper::server::conn::http2::Builder::new(TokioExecutor);
    let http = builder.max_concurrent_streams(20);
    let io = tokio::net::TcpStream::from_std(stream).unwrap();
    http.serve_connection(io, service_fn(test)).await
}

pub async fn setup_http1(
    future: impl std::future::Future<Output = anyhow::Result<()>>,
) -> Result<(), anyhow::Error> {
    static CELL_HTTP1: OnceLock<std::net::TcpListener> = OnceLock::new();
    let listener = CELL_HTTP1.get_or_init(|| {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        std::net::TcpListener::bind(addr).unwrap()
    });

    let thread = tokio::task::spawn(async move {
        let (stream, _) = listener.accept().unwrap();
        let conn = serve_http1_connection(stream).await;
        if let Err(err) = conn {
            println!("Error serving connection: {:?}", err);
        }
    });

    future.await?;
    thread.await.unwrap();
    Ok(())
}

pub async fn setup_http2(
    future: impl std::future::Future<Output = anyhow::Result<()>>,
) -> anyhow::Result<()> {
    static CELL_HTTP2: OnceLock<std::net::TcpListener> = OnceLock::new();
    let listener = CELL_HTTP2.get_or_init(|| {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
        std::net::TcpListener::bind(addr).unwrap()
    });
    let thread = tokio::task::spawn(async move {
        let (stream, _) = listener.accept().unwrap();
        let conn = serve_http2_connection(stream).await;
        if let Err(err) = conn {
            println!("Error serving connection: {:?}", err);
        }
    });

    future.await?;
    thread.await.unwrap();

    Ok(())
}

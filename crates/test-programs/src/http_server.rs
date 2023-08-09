use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::{body::Bytes, service::service_fn, Request, Response};
use std::{error::Error, net::SocketAddr};
use tokio::{net::TcpListener, runtime::Handle};

async fn test(
    req: Request<hyper::body::Incoming>,
) -> http::Result<Response<BoxBody<Bytes, hyper::Error>>> {
    let method = req.method().to_string();
    Response::builder()
        .status(http::StatusCode::OK)
        .header("x-wasmtime-test-method", method)
        .header("x-wasmtime-test-uri", req.uri().to_string())
        .body(req.into_body().boxed())
}

async fn async_run_serve() -> Result<(), Box<dyn Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, service_fn(test))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

pub async fn run_server() {
    let _thread = Handle::current().spawn(async move {
        async_run_serve()
            .await
            .map_err(|err| format!("Error while running test server: {:?}", err))
            .unwrap();
    });
}

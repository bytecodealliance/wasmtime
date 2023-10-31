use anyhow::Context;
use wasi_http_tests::bindings::wasi::http::types::{Method, Scheme};

fn main() {
    wasi_http_tests::in_tokio(async { run().await })
}

async fn run() {
    let res = wasi_http_tests::request(
        Method::Get,
        Scheme::Http,
        "localhost:3000",
        "/get?some=arg&goes=here",
        None,
        None,
    )
    .await
    .context("localhost:3000 /get")
    .unwrap();

    println!("localhost:3000 /get: {res:?}");
    assert_eq!(res.status, 200);
    let method = res.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "GET");
    let uri = res.header("x-wasmtime-test-uri").unwrap();
    assert_eq!(
        std::str::from_utf8(uri).unwrap(),
        "http://localhost:3000/get?some=arg&goes=here"
    );
    assert_eq!(res.body, b"");
}

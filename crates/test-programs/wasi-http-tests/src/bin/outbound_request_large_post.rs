use anyhow::Context;
use std::io::{self, Read};
use wasi_http_tests::bindings::wasi::http::types::{Method, Scheme};

fn main() {
    wasi_http_tests::in_tokio(async { run().await })
}

async fn run() {
    // TODO: ensure more than 700 bytes is allowed without error
    const LEN: usize = 700;
    let mut buffer = [0; LEN];
    io::repeat(0b001).read_exact(&mut buffer).unwrap();
    let res = wasi_http_tests::request(
        Method::Post,
        Scheme::Http,
        "localhost:3000",
        "/post",
        Some(&buffer),
        None,
    )
    .await
    .context("localhost:3000 /post large")
    .unwrap();

    println!("localhost:3000 /post large: {res:?}");
    assert_eq!(res.status, 200);
    let method = res.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "POST");
    assert_eq!(res.body.len(), LEN);
}

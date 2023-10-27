use anyhow::Context;
use std::io::{self, Read};
use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    // TODO: ensure more than 700 bytes is allowed without error
    const LEN: usize = 700;
    let mut buffer = [0; LEN];
    let addr = std::env::var("HTTP_SERVER").unwrap();
    io::repeat(0b001).read_exact(&mut buffer).unwrap();
    let res = test_programs::http::request(
        Method::Post,
        Scheme::Http,
        &addr,
        "/post",
        Some(&buffer),
        None,
    )
    .context("/post large")
    .unwrap();

    println!("/post large: {}", res.status);
    assert_eq!(res.status, 200);
    let method = res.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "POST");
    assert_eq!(res.body.len(), LEN);
}

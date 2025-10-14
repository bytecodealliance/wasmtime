use anyhow::Context;
use std::io::{self, Read};
use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    // Make sure the final body is larger than 1024*1024, but we cannot allocate
    // so much memory directly in the wasm program, so we use the `repeat`
    // method to increase the body size.
    const LEN: usize = 1024;
    const REPEAT: usize = 1025;
    let mut buffer = [0; LEN];
    let addr = std::env::var("HTTP_SERVER").unwrap();
    io::repeat(0b001).read_exact(&mut buffer).unwrap();
    let res = test_programs::http::request(
        Method::Post,
        Scheme::Http,
        &addr,
        "/post",
        Some(&buffer.repeat(REPEAT)),
        None,
        None,
        None,
        None,
    )
    .context("/post large")
    .unwrap();

    println!("/post large: {}", res.status);
    assert_eq!(res.status, 200);
    let method = res.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "POST");
    assert_eq!(res.body.len(), LEN * REPEAT);
}

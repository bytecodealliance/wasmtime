use anyhow::Context;
use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    let addr = std::env::var("HTTP_SERVER").unwrap();
    let res = test_programs::http::request(
        Method::Post,
        Scheme::Http,
        &addr,
        "/post",
        Some(b"{\"foo\": \"bar\"}"),
        None,
    )
    .context("/post")
    .unwrap();

    println!("/post: {res:?}");
    assert_eq!(res.status, 200);
    let method = res.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "POST");
    let uri = res.header("x-wasmtime-test-uri").unwrap();
    assert_eq!(std::str::from_utf8(uri).unwrap(), format!("/post"));
    assert_eq!(res.body, b"{\"foo\": \"bar\"}", "invalid body returned");
}

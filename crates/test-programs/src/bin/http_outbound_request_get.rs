use anyhow::Context;
use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    let addr = std::env::var("HTTP_SERVER").unwrap();
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        &addr,
        "/get?some=arg&goes=here",
        None,
        None,
    )
    .context("/get")
    .unwrap();

    println!("{addr} /get: {res:?}");
    assert_eq!(res.status, 200);
    let method = res.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "GET");
    let uri = res.header("x-wasmtime-test-uri").unwrap();
    assert_eq!(
        std::str::from_utf8(uri).unwrap(),
        format!("/get?some=arg&goes=here")
    );
    assert_eq!(res.body, b"");
}

use anyhow::{Context, Result};
use wasi_http_tests::bindings::wasi::http::types::{Method, Scheme};

struct Component;

fn main() -> Result<(), ()> {
    let res = wasi_http_tests::request(
        Method::Put,
        Scheme::Http,
        "localhost:3000",
        "/put",
        Some(&[]),
        None,
    )
    .context("localhost:3000 /put")
    .unwrap();

    println!("localhost:3000 /put: {res:?}");
    assert_eq!(res.status, 200);
    let method = res.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "PUT");
    assert_eq!(res.body, b"");

    Ok(())
}

impl wasi_http_tests::bindings::CommandExtended for Component {
    fn run() -> Result<(), ()> {
        main()
    }
}

wasi_http_tests::export_command_extended!(Component);

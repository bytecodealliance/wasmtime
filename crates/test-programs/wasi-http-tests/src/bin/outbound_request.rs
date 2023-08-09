use anyhow::{Context, Result};
use wasi_http_tests::{
    bindings::{
        wasi::http::types::{Method, Scheme},
        CommandExtended,
    },
    request,
};

struct Component;

fn main() -> Result<()> {
    let r1 = request(
        Method::Get,
        Scheme::Http,
        "localhost:3000",
        "/get?some=arg&goes=here",
        None,
    )
    .context("localhost:3000 /get")?;

    println!("localhost:3000 /get: {r1:?}");
    assert_eq!(r1.status, 200);
    let method = r1.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "GET");
    let uri = r1.header("x-wasmtime-test-uri").unwrap();
    assert_eq!(
        std::str::from_utf8(uri).unwrap(),
        "http://localhost:3000/get?some=arg&goes=here"
    );
    assert_eq!(r1.body, b"");

    let r2 = request(
        Method::Post,
        Scheme::Http,
        "localhost:3000",
        "/post",
        Some(b"{\"foo\": \"bar\"}"),
    )
    .context("localhost:3000 /post")?;

    println!("localhost:3000 /post: {r2:?}");
    assert_eq!(r2.status, 200);
    let method = r2.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "POST");
    assert_eq!(r2.body, b"{\"foo\": \"bar\"}");

    let r3 = request(
        Method::Put,
        Scheme::Http,
        "localhost:3000",
        "/put",
        Some(&[]),
    )
    .context("localhost:3000 /put")?;

    println!("localhost:3000 /put: {r3:?}");
    assert_eq!(r3.status, 200);
    let method = r3.header("x-wasmtime-test-method").unwrap();
    assert_eq!(std::str::from_utf8(method).unwrap(), "PUT");
    assert_eq!(r3.body, b"");

    let r4 = request(
        Method::Other("OTHER".to_owned()),
        Scheme::Http,
        "localhost:3000",
        "/",
        None,
    );

    let error = r4.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Error::UnexpectedError(\"unknown method OTHER\")"
    );

    let r5 = request(
        Method::Get,
        Scheme::Other("WS".to_owned()),
        "localhost:3000",
        "/",
        None,
    );

    let error = r5.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Error::UnexpectedError(\"unsupported scheme WS\")"
    );

    Ok(())
}

impl CommandExtended for Component {
    fn run() -> Result<(), ()> {
        main().map_err(|e| eprintln!("{e:?}"))
    }
}

wasi_http_tests::export_command_extended!(Component);

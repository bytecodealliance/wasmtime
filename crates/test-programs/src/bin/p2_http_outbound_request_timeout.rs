use anyhow::Context;
use std::net::SocketAddr;
use std::time::Duration;
use test_programs::wasi::http::types::{ErrorCode, Method, Scheme};

fn main() {
    // This address inside the TEST-NET-3 address block is expected to time out.
    let addr = SocketAddr::from(([203, 0, 113, 12], 80)).to_string();
    let timeout = Duration::from_millis(200);
    let connect_timeout: Option<u64> = Some(timeout.as_nanos() as u64);
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        &addr,
        "/get?some=arg&goes=here",
        None,
        None,
        connect_timeout,
        None,
        None,
    )
    .context("/get");

    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        matches!(
            err.downcast_ref::<ErrorCode>(),
            Some(ErrorCode::ConnectionTimeout | ErrorCode::ConnectionRefused)
        ),
        "expected connection timeout: {err:?}"
    );

    let addr = std::env::var("HTTP_SERVER").unwrap();

    // zero timeouts are accepted and shouldn't panic anything on the host, but
    // they're a bit finnicky so don't assert the result
    let _ = test_programs::http::request(
        Method::Post,
        Scheme::Http,
        &addr,
        "/post",
        Some(b"{\"foo\": \"bar\"}"),
        None,
        Some(0),
        None,
        None,
    );
    let _ = test_programs::http::request(
        Method::Post,
        Scheme::Http,
        &addr,
        "/post",
        Some(b"{\"foo\": \"bar\"}"),
        None,
        None,
        Some(0),
        None,
    );
    let _ = test_programs::http::request(
        Method::Post,
        Scheme::Http,
        &addr,
        "/post",
        Some(b"{\"foo\": \"bar\"}"),
        None,
        None,
        None,
        Some(0),
    );
}

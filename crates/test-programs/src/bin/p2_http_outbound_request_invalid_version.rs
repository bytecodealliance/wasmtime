use test_programs::wasi::http::types::{ErrorCode, Method, Scheme};

fn main() {
    let addr = std::env::var("HTTP_SERVER").unwrap();
    let res = test_programs::http::request(
        Method::Connect,
        Scheme::Http,
        &addr,
        "/",
        None,
        Some(&[]),
        None,
        Some(1_000_000_000),
        None,
    );

    // The error seen during this test is mostly an `HttpProtocolError`, but
    // depending on scheduling it's possible to get stuck in hyper right now
    // where the server is indefinitely waiting on the client and the client
    // times out. Accept both kinds of errors here, and note the explicit 1s
    // timeout above to avoid this taking too long. in the timeout case.
    let err = res.unwrap_err();
    assert!(
        matches!(
            err.downcast_ref::<ErrorCode>()
                .expect("expected a wasi-http ErrorCode"),
            ErrorCode::HttpProtocolError | ErrorCode::ConnectionReadTimeout,
        ),
        "unexpected error: {err:?}"
    );
}

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
        None,
        None,
    );

    assert!(matches!(
        res.unwrap_err()
            .downcast::<ErrorCode>()
            .expect("expected a wasi-http ErrorCode"),
        ErrorCode::HttpProtocolError,
    ));
}

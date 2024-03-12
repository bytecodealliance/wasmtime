use test_programs::wasi::http::types::{ErrorCode, Method, Scheme};

fn main() {
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Other("WS".to_owned()),
        "localhost:3000",
        "/",
        None,
        None,
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

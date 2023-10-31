use test_programs::wasi::http::types::{ErrorCode, HttpRequestErrorPayload, Method, Scheme};

fn main() {
    let res = test_programs::http::request(
        Method::Other("OTHER".to_owned()),
        Scheme::Http,
        "localhost:3000",
        "/",
        None,
        None,
    );

    assert!(matches!(
        res.unwrap_err()
            .downcast::<ErrorCode>()
            .expect("expected a wasi-http ErrorCode"),
        ErrorCode::HttpRequestError(HttpRequestErrorPayload {
            status_code: 405,
            ..
        }),
    ));
}

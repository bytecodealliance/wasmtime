use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Other("WS".to_owned()),
        "localhost:3000",
        "/",
        None,
        None,
    );

    let error = res.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Error::InvalidUrl(\"unsupported scheme WS\")"
    );
}

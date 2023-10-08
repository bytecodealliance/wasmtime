use wasi_http_tests::bindings::wasi::http::types::{Method, Scheme};

fn main() {
    let res = wasi_http_tests::request(
        Method::Get,
        Scheme::Http,
        "localhost:99999",
        "/",
        None,
        None,
    );

    let error = res.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Error::InvalidUrl(\"invalid port value\")"
    );
}

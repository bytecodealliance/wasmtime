use wasi_http_tests::bindings::wasi::http::types::{Method, Scheme};

fn main() {
    let res = wasi_http_tests::request(
        Method::Get,
        Scheme::Http,
        "some.invalid.dnsname:3000",
        "/",
        None,
        None,
    );

    let error = res.unwrap_err().to_string();
    assert!(
        error.starts_with("Error::InvalidUrl(\""),
        "bad error: {error}"
    );
}

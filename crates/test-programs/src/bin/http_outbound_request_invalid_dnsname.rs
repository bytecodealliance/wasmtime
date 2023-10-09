use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    let res = test_programs::http::request(
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

use wasi_http_tests::bindings::wasi::http::types::{Method, Scheme};

fn main() {
    wasi_http_tests::in_tokio(async { run().await })
}

async fn run() {
    let res = wasi_http_tests::request(
        Method::Get,
        Scheme::Http,
        "some.invalid.dnsname:3000",
        "/",
        None,
        None,
    )
    .await;

    let error = res.unwrap_err();
    assert_eq!(error.to_string(), "Error::InvalidUrl(\"invalid dnsname\")");
}

use test_programs::wasi::http::types::{ErrorCode, Method, Scheme};

fn main() {
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        "some.invalid.dnsname:3000",
        "/",
        None,
        None,
    );

    assert!(matches!(
        res.unwrap_err()
            .downcast::<ErrorCode>()
            .expect("expected a wasi-http ErrorCode"),
        ErrorCode::DnsError(_)
    ));
}

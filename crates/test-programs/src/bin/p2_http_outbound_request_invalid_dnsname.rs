use test_programs::wasi::http::types::{ErrorCode, Method, Scheme};

fn main() {
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        "some.invalid.dnsname:3000",
        "/",
        None,
        None,
        None,
        None,
        None,
    );

    let e = res.unwrap_err();
    assert!(
        matches!(
            e.downcast_ref::<ErrorCode>()
                .expect("expected a wasi-http ErrorCode"),
            ErrorCode::DnsError(_) | ErrorCode::ConnectionRefused,
        ),
        "Unexpected error: {e:#?}"
    );
}

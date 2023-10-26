use test_programs::wasi::http::types::{Headers, Method, Scheme};

fn main() {
    let hdrs = Headers::new(&[]);
    assert!(hdrs
        .append(&"malformed header name".to_owned(), &b"bad".to_vec())
        .is_err());

    let addr = std::env::var("HTTP_SERVER").unwrap();
    let err = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        &addr,
        "/get?some=arg&goes=here",
        None,
        Some(&[("transfer-encoding".to_owned(), b"bad".to_vec())]),
    )
    .expect_err("invalid request");

    assert_eq!(
        err.to_string(),
        "Error::HeaderNameError(\"forbidden header\")"
    );
}

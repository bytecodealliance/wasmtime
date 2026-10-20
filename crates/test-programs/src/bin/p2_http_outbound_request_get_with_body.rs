use anyhow::Context;
use test_programs::wasi::http::types::{Method, Scheme};

fn main() {
    let addr = std::env::var("HTTP_SERVER").unwrap();

    // Same shape with `POST`, which the encoder already frames as chunked.
    let res = test_programs::http::request(
        Method::Post,
        Scheme::Http,
        &addr,
        "/post-with-body",
        Some(b"0123456789"),
        None,
        None,
        None,
        None,
    )
    .context("/post-with-body")
    .unwrap();
    println!("/post-with-body: {res:?}");
    assert_eq!(res.status, 200);
    assert_eq!(res.body, b"0123456789", "invalid body returned");

    // The request has a body but no framing header, which the HTTP/1.1 encoder
    // requires for `GET` to send the body at all.
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        &addr,
        "/get-with-body",
        Some(b"0123456789"),
        None,
        None,
        None,
        None,
    )
    .context("/get-with-body")
    .unwrap();
    println!("/get-with-body: {res:?}");
    assert_eq!(res.status, 200);
    assert_eq!(res.body, b"0123456789", "invalid body returned");

    // A GET whose body is created but never written to: rejecting would fail
    // this one too.
    let res = test_programs::http::request(
        Method::Get,
        Scheme::Http,
        &addr,
        "/get-with-empty-body",
        Some(b""),
        None,
        None,
        None,
        None,
    )
    .context("/get-with-empty-body")
    .unwrap();
    println!("/get-with-empty-body: {res:?}");
    assert_eq!(res.status, 200);
    assert_eq!(res.body, b"", "invalid body returned");
}

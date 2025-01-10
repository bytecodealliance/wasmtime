use test_programs::wasi::http::types::{HeaderError, Headers, OutgoingRequest};

fn main() {
    let hdrs = Headers::new();
    assert!(matches!(
        hdrs.append("malformed header name", b"ok value"),
        Err(HeaderError::InvalidSyntax)
    ));

    assert!(matches!(hdrs.append("ok-header-name", b"ok value"), Ok(())));

    assert!(matches!(
        hdrs.append("ok-header-name", b"bad\nvalue"),
        Err(HeaderError::InvalidSyntax)
    ));

    assert!(matches!(
        hdrs.append("Connection", b"keep-alive"),
        Err(HeaderError::Forbidden)
    ));

    assert!(matches!(
        hdrs.append("Keep-Alive", b"stuff"),
        Err(HeaderError::Forbidden)
    ));

    assert!(matches!(
        hdrs.append("Host", b"example.com"),
        Err(HeaderError::Forbidden)
    ));

    assert!(matches!(
        hdrs.append("curbidden-header", b"keep-alive"),
        Err(HeaderError::Forbidden)
    ));

    assert!(matches!(
        hdrs.append("Curbidden-Header", b"keep-alive"),
        Err(HeaderError::Forbidden)
    ));

    assert!(matches!(
        Headers::from_list(&[("bad header".to_owned(), b"value".to_vec())]),
        Err(HeaderError::InvalidSyntax)
    ));

    assert!(matches!(
        Headers::from_list(&[("custom-forbidden-header".to_owned(), b"value".to_vec())]),
        Err(HeaderError::Forbidden)
    ));

    assert!(matches!(
        Headers::from_list(&[("ok-header-name".to_owned(), b"bad\nvalue".to_vec())]),
        Err(HeaderError::InvalidSyntax)
    ));

    let req = OutgoingRequest::new(hdrs);
    let hdrs = req.headers();

    assert!(matches!(
        hdrs.set("Content-Length", &[b"10".to_vec()]),
        Err(HeaderError::Immutable),
    ));

    assert!(matches!(
        hdrs.append("Content-Length", b"10"),
        Err(HeaderError::Immutable),
    ));

    assert!(matches!(
        hdrs.delete("Content-Length"),
        Err(HeaderError::Immutable),
    ));
}

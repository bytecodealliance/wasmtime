use test_programs::wasi::http::types::{HeaderError, Headers, OutgoingRequest};

fn main() {
    let hdrs = Headers::new();
    assert!(matches!(
        hdrs.append(&"malformed header name".to_owned(), &b"ok value".to_vec()),
        Err(HeaderError::InvalidSyntax)
    ));

    assert!(matches!(
        hdrs.append(&"ok-header-name".to_owned(), &b"ok value".to_vec()),
        Ok(())
    ));

    assert!(matches!(
        hdrs.append(&"ok-header-name".to_owned(), &b"bad\nvalue".to_vec()),
        Err(HeaderError::InvalidSyntax)
    ));

    assert!(matches!(
        hdrs.append(&"Connection".to_owned(), &b"keep-alive".to_vec()),
        Err(HeaderError::Forbidden)
    ));

    assert!(matches!(
        hdrs.append(&"Keep-Alive".to_owned(), &b"stuff".to_vec()),
        Err(HeaderError::Forbidden)
    ));

    assert!(matches!(
        hdrs.append(&"Host".to_owned(), &b"example.com".to_vec()),
        Err(HeaderError::Forbidden)
    ));

    assert!(matches!(
        hdrs.append(
            &"custom-forbidden-header".to_owned(),
            &b"keep-alive".to_vec()
        ),
        Err(HeaderError::Forbidden)
    ));

    assert!(matches!(
        hdrs.append(
            &"Custom-Forbidden-Header".to_owned(),
            &b"keep-alive".to_vec()
        ),
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
        hdrs.set(&"Content-Length".to_owned(), &[b"10".to_vec()]),
        Err(HeaderError::Immutable),
    ));

    assert!(matches!(
        hdrs.append(&"Content-Length".to_owned(), &b"10".to_vec()),
        Err(HeaderError::Immutable),
    ));

    assert!(matches!(
        hdrs.delete(&"Content-Length".to_owned()),
        Err(HeaderError::Immutable),
    ));
}

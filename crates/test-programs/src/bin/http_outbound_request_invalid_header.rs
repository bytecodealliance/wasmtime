use test_programs::wasi::http::types::{HeaderError, Headers};

fn main() {
    let hdrs = Headers::new(&[]);
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
}

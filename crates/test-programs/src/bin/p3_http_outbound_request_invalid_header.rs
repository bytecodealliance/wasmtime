use test_programs::p3::wasi::http::types::{HeaderError, Headers, Request};
use test_programs::p3::wit_future;

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let hdrs = Headers::new();
        assert!(matches!(
            hdrs.append("malformed header name", b"ok value".as_ref()),
            Err(HeaderError::InvalidSyntax)
        ));

        assert!(matches!(
            hdrs.append("ok-header-name", b"ok value".as_ref()),
            Ok(())
        ));

        assert!(matches!(
            hdrs.append("ok-header-name", b"bad\nvalue".as_ref()),
            Err(HeaderError::InvalidSyntax)
        ));

        assert!(matches!(
            hdrs.append("Connection", b"keep-alive".as_ref()),
            Err(HeaderError::Forbidden)
        ));

        assert!(matches!(
            hdrs.append("Keep-Alive", b"stuff".as_ref()),
            Err(HeaderError::Forbidden)
        ));

        assert!(matches!(
            hdrs.append("Host", b"example.com".as_ref()),
            Err(HeaderError::Forbidden)
        ));

        assert!(matches!(
            hdrs.append("custom-forbidden-header", b"keep-alive".as_ref()),
            Err(HeaderError::Forbidden)
        ));

        assert!(matches!(
            hdrs.append("Custom-Forbidden-Header", b"keep-alive".as_ref()),
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

        let (_, rx) = wit_future::new(|| Ok(None));
        let (req, _) = Request::new(hdrs, None, rx, None);
        let hdrs = req.get_headers();

        assert!(matches!(
            hdrs.set("Content-Length", &[b"10".to_vec()]),
            Err(HeaderError::Immutable),
        ));

        assert!(matches!(
            hdrs.append("Content-Length", b"10".as_ref()),
            Err(HeaderError::Immutable),
        ));

        assert!(matches!(
            hdrs.delete("Content-Length"),
            Err(HeaderError::Immutable),
        ));
        Ok(())
    }
}

fn main() {}

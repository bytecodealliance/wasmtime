//! Wasmtime's implementation of `wasi:http`
//!
//! This crate is organized similarly to [`wasmtime_wasi`] where there is a
//! top-level [`p2`] and [`p3`] module corresponding to the implementation for
//! WASIp2 and WASIp3.

#![deny(missing_docs)]
#![doc(test(attr(deny(warnings))))]
#![doc(test(attr(allow(dead_code, unused_variables, unused_mut))))]
#![cfg_attr(docsrs, feature(doc_cfg))]

use http::{HeaderName, header};

#[cfg(any(feature = "p2", feature = "p3"))]
mod authority;
#[cfg(any(feature = "p2", feature = "p3"))]
pub(crate) use authority::parse_authority;

mod ctx;
#[cfg(feature = "default-send-request")]
mod default_send_request;
mod error;
mod field_map;
#[cfg(feature = "component-model-async")]
pub mod handler;
pub mod io;
#[cfg(feature = "p2")]
pub mod p2;
#[cfg(feature = "p3")]
pub mod p3;
mod request_options;

pub use ctx::*;
#[cfg(feature = "default-send-request")]
pub use default_send_request::*;
pub use error::*;
pub use field_map::*;
pub use request_options::*;

/// Extract the `Content-Length` header value from a [`http::HeaderMap`], returning `None` if it's not
/// present. This function will return `Err` if it's not possible to parse the `Content-Length`
/// header.
#[cfg(any(feature = "p2", feature = "p3"))]
fn get_content_length(headers: &http::HeaderMap) -> wasmtime::Result<Option<u64>> {
    let mut values = headers.get_all(header::CONTENT_LENGTH).iter();
    let Some(v) = values.next() else {
        return Ok(None);
    };
    // `FieldMap` rejects multiple `Content-Length` values, but double-check
    // here since the framing of the body depends on there being exactly one.
    if values.next().is_some() {
        wasmtime::bail!("multiple `content-length` header values");
    }
    let v = v.to_str()?;
    // RFC 9110 defines `Content-Length` as `1*DIGIT`. `u64`'s `FromStr` is more
    // lenient and also accepts a leading `+`, so reject anything that isn't a
    // non-empty run of decimal digits before parsing.
    if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
        wasmtime::bail!("invalid `content-length` header value: {v:?}");
    }
    let v = v.parse()?;
    Ok(Some(v))
}

/// Parse the specified string as an `http::uri::PathAndQuery`, if possible.
///
/// https://datatracker.ietf.org/doc/html/rfc3986#section-3.3 says:
///
/// > If a URI contains an authority component, then the path component must
/// > either be empty or begin with a slash ("/") character.
///
/// `PathAndQuery` doesn't check this: it accepts `http://example.com/`, `*`,
/// and `x`. Those are rejected here, as they would otherwise end up verbatim in
/// the request line of outgoing requests, and an absolute URI there addresses
/// the request to a different host than the one it's sent to.
#[cfg(any(feature = "p2", feature = "p3"))]
fn parse_path_with_query(path_with_query: &str) -> Option<http::uri::PathAndQuery> {
    if !(path_with_query.is_empty()
        || path_with_query.starts_with('/')
        || path_with_query.starts_with('?'))
    {
        return None;
    }
    http::uri::Builder::new()
        .path_and_query(path_with_query)
        .build()
        .ok()
        .and_then(|uri| uri.path_and_query().cloned())
}

#[cfg(all(test, any(feature = "p2", feature = "p3")))]
mod content_length_tests {
    use super::get_content_length;
    use http::{HeaderMap, HeaderValue, header};

    fn headers(value: &str) -> HeaderMap {
        let mut map = HeaderMap::new();
        map.insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(value).unwrap(),
        );
        map
    }

    #[test]
    fn content_length_must_be_decimal_digits() {
        assert_eq!(get_content_length(&HeaderMap::new()).unwrap(), None);
        assert_eq!(get_content_length(&headers("0")).unwrap(), Some(0));
        assert_eq!(get_content_length(&headers("1234")).unwrap(), Some(1234));

        // `u64::from_str` accepts these but they are not `1*DIGIT` per RFC 9110.
        assert!(get_content_length(&headers("+5")).is_err());
        assert!(get_content_length(&headers("-5")).is_err());
        assert!(get_content_length(&headers(" 5")).is_err());
        assert!(get_content_length(&headers("")).is_err());

        // Duplicate content-length definition is forbidden
        let mut map = headers("5");
        map.append(header::CONTENT_LENGTH, HeaderValue::from_static("5"));
        assert!(get_content_length(&map).is_err());
    }
}

/// Set of [http::header::HeaderName], that are forbidden by default
/// for requests and responses originating in the guest.
pub const DEFAULT_FORBIDDEN_HEADERS: [HeaderName; 9] = [
    header::CONNECTION,
    HeaderName::from_static("keep-alive"),
    header::PROXY_AUTHENTICATE,
    header::PROXY_AUTHORIZATION,
    HeaderName::from_static("proxy-connection"),
    header::TRANSFER_ENCODING,
    header::UPGRADE,
    header::HOST,
    HeaderName::from_static("http2-settings"),
];

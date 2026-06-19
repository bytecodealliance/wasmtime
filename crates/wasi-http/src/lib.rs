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

mod ctx;
mod field_map;
#[cfg(feature = "component-model-async")]
pub mod handler;
pub mod io;
#[cfg(feature = "p2")]
pub mod p2;
#[cfg(feature = "p3")]
pub mod p3;

pub use ctx::*;
pub use field_map::*;

/// Extract the `Content-Length` header value from a [`http::HeaderMap`], returning `None` if it's not
/// present. This function will return `Err` if it's not possible to parse the `Content-Length`
/// header.
#[cfg(any(feature = "p2", feature = "p3"))]
fn get_content_length(headers: &http::HeaderMap) -> wasmtime::Result<Option<u64>> {
    let Some(v) = headers.get(header::CONTENT_LENGTH) else {
        return Ok(None);
    };
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
    }
}

/// Resolve the rustls [`ServerName`] used for TLS certificate verification from
/// an outbound request `authority`.
///
/// `authority` is in `host:port` form, where an IPv6 `host` is wrapped in
/// brackets (for example `[::1]:443`). An IP literal is recognized by parsing
/// the whole authority as a [`SocketAddr`]; this handles the bracketed IPv6
/// form, which splitting on the first `:` would truncate. Anything else is
/// treated as a host name, with the port stripped off before it is handed to
/// rustls.
///
/// [`ServerName`]: rustls::pki_types::ServerName
/// [`SocketAddr`]: std::net::SocketAddr
#[cfg(all(feature = "default-send-request", any(feature = "p2", feature = "p3")))]
fn tls_server_name(
    authority: &str,
) -> Result<rustls::pki_types::ServerName<'static>, rustls::pki_types::InvalidDnsNameError> {
    use rustls::pki_types::ServerName;

    if let Ok(addr) = authority.parse::<std::net::SocketAddr>() {
        return Ok(ServerName::from(addr.ip()));
    }
    let host = match authority.split_once(':') {
        Some((host, _port)) => host,
        None => authority,
    };
    Ok(ServerName::try_from(host)?.to_owned())
}

#[cfg(all(
    test,
    feature = "default-send-request",
    any(feature = "p2", feature = "p3")
))]
mod tls_server_name_tests {
    use super::tls_server_name;
    use rustls::pki_types::ServerName;

    #[test]
    fn resolves_server_name_from_authority() {
        // Host names keep their host and drop the port.
        assert_eq!(
            tls_server_name("example.com:443").unwrap(),
            ServerName::try_from("example.com").unwrap()
        );
        assert_eq!(
            tls_server_name("example.com").unwrap(),
            ServerName::try_from("example.com").unwrap()
        );

        // IP literals resolve to an `IpAddress` server name. The bracketed IPv6
        // form must not be truncated at the first `:`.
        assert_eq!(
            tls_server_name("127.0.0.1:80").unwrap(),
            ServerName::from(std::net::Ipv4Addr::LOCALHOST)
        );
        assert_eq!(
            tls_server_name("[::1]:443").unwrap(),
            ServerName::from(std::net::Ipv6Addr::LOCALHOST)
        );
        assert_eq!(
            tls_server_name("[2001:db8::1]:8443").unwrap(),
            ServerName::from("2001:db8::1".parse::<std::net::Ipv6Addr>().unwrap())
        );
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

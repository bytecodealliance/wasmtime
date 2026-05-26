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
    let v = v.parse()?;
    Ok(Some(v))
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

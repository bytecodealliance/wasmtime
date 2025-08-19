use crate::p3::bindings::http::types::ErrorCode;
use crate::p3::body::Body;
use bytes::Bytes;
use core::time::Duration;
use http::uri::{Authority, PathAndQuery, Scheme};
use http::{HeaderMap, Method};
use http_body_util::BodyExt as _;
use http_body_util::combinators::BoxBody;
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct RequestOptions {
    /// How long to wait for a connection to be established.
    pub connect_timeout: Option<Duration>,
    /// How long to wait for the first byte of the response body.
    pub first_byte_timeout: Option<Duration>,
    /// How long to wait between frames of the response body.
    pub between_bytes_timeout: Option<Duration>,
}

/// The concrete type behind a `wasi:http/types/request` resource.
pub struct Request {
    /// The method of the request.
    pub method: Method,
    /// The scheme of the request.
    pub scheme: Option<Scheme>,
    /// The authority of the request.
    pub authority: Option<Authority>,
    /// The path and query of the request.
    pub path_with_query: Option<PathAndQuery>,
    /// The request headers.
    pub headers: Arc<HeaderMap>,
    /// Request options.
    pub options: Option<Arc<RequestOptions>>,
    /// Request body.
    pub(crate) body: Body,
}

impl<T> From<http::Request<T>> for Request
where
    T: http_body::Body<Data = Bytes> + Send + Sync + 'static,
    T::Error: Into<ErrorCode>,
{
    fn from(req: http::Request<T>) -> Self {
        let (
            http::request::Parts {
                method,
                uri,
                headers,
                ..
            },
            body,
        ) = req.into_parts();
        let http::uri::Parts {
            scheme,
            authority,
            path_and_query,
            ..
        } = uri.into_parts();
        Self::new(
            method,
            scheme,
            authority,
            path_and_query,
            headers,
            None,
            body.map_err(Into::into).boxed(),
        )
    }
}

impl Request {
    /// Construct a new [Request]
    pub fn new(
        method: Method,
        scheme: Option<Scheme>,
        authority: Option<Authority>,
        path_with_query: Option<PathAndQuery>,
        headers: impl Into<Arc<HeaderMap>>,
        options: Option<Arc<RequestOptions>>,
        body: impl Into<BoxBody<Bytes, ErrorCode>>,
    ) -> Self {
        Self {
            method,
            scheme,
            authority,
            path_with_query,
            headers: headers.into(),
            options,
            body: Body::Host(body.into()),
        }
    }
}

//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::p2::{
    WasiHttpCtxView, WasiHttpHooks,
    bindings::http::types::{self, ErrorCode, Method, Scheme},
    body::{HostIncomingBody, HyperIncomingBody, HyperOutgoingBody},
};
use bytes::Bytes;
use http::header::{HeaderMap, HeaderName, HeaderValue};
use http_body_util::BodyExt;
use hyper::body::Body;
use std::any::Any;
use std::fmt;
use std::time::Duration;
use wasmtime::component::Resource;
use wasmtime::{Result, bail};
use wasmtime_wasi::p2::Pollable;
use wasmtime_wasi::runtime::AbortOnDropJoinHandle;

/// Removes forbidden headers from a [`FieldMap`].
pub(crate) fn remove_forbidden_headers(hooks: &mut dyn WasiHttpHooks, headers: &mut FieldMap) {
    let forbidden_keys = Vec::from_iter(headers.as_ref().keys().filter_map(|name| {
        if hooks.is_forbidden_header(name) {
            Some(name.clone())
        } else {
            None
        }
    }));

    for name in forbidden_keys {
        headers.remove_all(&name);
    }
}

/// Configuration for an outgoing request.
pub struct OutgoingRequestConfig {
    /// Whether to use TLS for the request.
    pub use_tls: bool,
    /// The timeout for connecting.
    pub connect_timeout: Duration,
    /// The timeout until the first byte.
    pub first_byte_timeout: Duration,
    /// The timeout between chunks of a streaming body
    pub between_bytes_timeout: Duration,
}

impl From<http::Method> for types::Method {
    fn from(method: http::Method) -> Self {
        if method == http::Method::GET {
            types::Method::Get
        } else if method == hyper::Method::HEAD {
            types::Method::Head
        } else if method == hyper::Method::POST {
            types::Method::Post
        } else if method == hyper::Method::PUT {
            types::Method::Put
        } else if method == hyper::Method::DELETE {
            types::Method::Delete
        } else if method == hyper::Method::CONNECT {
            types::Method::Connect
        } else if method == hyper::Method::OPTIONS {
            types::Method::Options
        } else if method == hyper::Method::TRACE {
            types::Method::Trace
        } else if method == hyper::Method::PATCH {
            types::Method::Patch
        } else {
            types::Method::Other(method.to_string())
        }
    }
}

impl TryInto<http::Method> for types::Method {
    type Error = http::method::InvalidMethod;

    fn try_into(self) -> Result<http::Method, Self::Error> {
        match self {
            Method::Get => Ok(http::Method::GET),
            Method::Head => Ok(http::Method::HEAD),
            Method::Post => Ok(http::Method::POST),
            Method::Put => Ok(http::Method::PUT),
            Method::Delete => Ok(http::Method::DELETE),
            Method::Connect => Ok(http::Method::CONNECT),
            Method::Options => Ok(http::Method::OPTIONS),
            Method::Trace => Ok(http::Method::TRACE),
            Method::Patch => Ok(http::Method::PATCH),
            Method::Other(s) => http::Method::from_bytes(s.as_bytes()),
        }
    }
}

/// The concrete type behind a `wasi:http/types.incoming-request` resource.
#[derive(Debug)]
pub struct HostIncomingRequest {
    pub(crate) method: http::method::Method,
    pub(crate) uri: http::uri::Uri,
    pub(crate) headers: FieldMap,
    pub(crate) scheme: Scheme,
    pub(crate) authority: String,
    /// The body of the incoming request.
    pub body: Option<HostIncomingBody>,
}

impl WasiHttpCtxView<'_> {
    /// Create a new incoming request resource.
    pub fn new_incoming_request<B>(
        &mut self,
        scheme: Scheme,
        req: hyper::Request<B>,
    ) -> wasmtime::Result<Resource<HostIncomingRequest>>
    where
        B: Body<Data = Bytes> + Send + 'static,
        B::Error: Into<ErrorCode>,
    {
        let field_size_limit = self.ctx.field_size_limit;
        let (parts, body) = req.into_parts();
        let body = body.map_err(Into::into).boxed_unsync();
        let body = HostIncomingBody::new(
            body,
            // TODO: this needs to be plumbed through
            std::time::Duration::from_millis(600 * 1000),
            field_size_limit,
        );
        let authority = match parts.uri.authority() {
            Some(authority) => authority.to_string(),
            None => match parts.headers.get(http::header::HOST) {
                Some(host) => host.to_str()?.to_string(),
                None => bail!("invalid HTTP request missing authority in URI and host header"),
            },
        };

        let mut headers = FieldMap::new(parts.headers, field_size_limit);
        remove_forbidden_headers(self.hooks, &mut headers);

        let req = HostIncomingRequest {
            method: parts.method,
            uri: parts.uri,
            headers,
            authority,
            scheme,
            body: Some(body),
        };
        Ok(self.table.push(req)?)
    }
}

/// The concrete type behind a `wasi:http/types.response-outparam` resource.
pub struct HostResponseOutparam {
    /// The sender for sending a response.
    pub result:
        tokio::sync::oneshot::Sender<Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>>,
}

impl WasiHttpCtxView<'_> {
    /// Create a new outgoing response resource.
    pub fn new_response_outparam(
        &mut self,
        result: tokio::sync::oneshot::Sender<
            Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>,
        >,
    ) -> wasmtime::Result<Resource<HostResponseOutparam>> {
        let id = self.table.push(HostResponseOutparam { result })?;
        Ok(id)
    }
}

/// The concrete type behind a `wasi:http/types.outgoing-response` resource.
pub struct HostOutgoingResponse {
    /// The status of the response.
    pub status: http::StatusCode,
    /// The headers of the response.
    pub headers: FieldMap,
    /// The body of the response.
    pub body: Option<HyperOutgoingBody>,
}

impl TryFrom<HostOutgoingResponse> for hyper::Response<HyperOutgoingBody> {
    type Error = http::Error;

    fn try_from(
        resp: HostOutgoingResponse,
    ) -> Result<hyper::Response<HyperOutgoingBody>, Self::Error> {
        use http_body_util::Empty;

        let mut builder = hyper::Response::builder().status(resp.status);

        *builder.headers_mut().unwrap() = resp.headers.map;

        match resp.body {
            Some(body) => builder.body(body),
            None => builder.body(
                Empty::<bytes::Bytes>::new()
                    .map_err(|_| unreachable!("Infallible error"))
                    .boxed_unsync(),
            ),
        }
    }
}

/// The concrete type behind a `wasi:http/types.outgoing-request` resource.
#[derive(Debug)]
pub struct HostOutgoingRequest {
    /// The method of the request.
    pub method: Method,
    /// The scheme of the request.
    pub scheme: Option<Scheme>,
    /// The authority of the request.
    pub authority: Option<String>,
    /// The path and query of the request.
    pub path_with_query: Option<String>,
    /// The request headers.
    pub headers: FieldMap,
    /// The request body.
    pub body: Option<HyperOutgoingBody>,
}

/// The concrete type behind a `wasi:http/types.request-options` resource.
#[derive(Debug, Default)]
pub struct HostRequestOptions {
    /// How long to wait for a connection to be established.
    pub connect_timeout: Option<std::time::Duration>,
    /// How long to wait for the first byte of the response body.
    pub first_byte_timeout: Option<std::time::Duration>,
    /// How long to wait between frames of the response body.
    pub between_bytes_timeout: Option<std::time::Duration>,
}

/// The concrete type behind a `wasi:http/types.incoming-response` resource.
#[derive(Debug)]
pub struct HostIncomingResponse {
    /// The response status
    pub status: u16,
    /// The response headers
    pub headers: FieldMap,
    /// The response body
    pub body: Option<HostIncomingBody>,
}

/// The concrete type behind a `wasi:http/types.fields` resource.
#[derive(Debug)]
pub enum HostFields {
    /// A reference to the fields of a parent entry.
    Ref {
        /// The parent resource rep.
        parent: u32,

        /// The function to get the fields from the parent.
        // NOTE: there's not failure in the result here because we assume that HostFields will
        // always be registered as a child of the entry with the `parent` id. This ensures that the
        // entry will always exist while this `HostFields::Ref` entry exists in the table, thus we
        // don't need to account for failure when fetching the fields ref from the parent.
        get_fields: for<'a> fn(elem: &'a mut (dyn Any + 'static)) -> &'a mut FieldMap,
    },
    /// An owned version of the fields.
    Owned {
        /// The fields themselves.
        fields: FieldMap,
    },
}

/// An owned version of `HostFields`. A wrapper on http `HeaderMap` that
/// keeps a running tally of memory consumed by header names and values.
#[derive(Debug, Clone)]
pub struct FieldMap {
    map: HeaderMap,
    limit: usize,
    size: usize,
}

/// Error given when a `FieldMap` has exceeded the size limit.
#[derive(Debug)]
pub struct FieldSizeLimitError {
    /// The erroring `FieldMap` operation would require this content size
    pub(crate) size: usize,
    /// The limit set on `FieldMap` content size
    pub(crate) limit: usize,
}
impl fmt::Display for FieldSizeLimitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Field size limit {} exceeded: {}", self.limit, self.size)
    }
}
impl std::error::Error for FieldSizeLimitError {}

impl FieldMap {
    /// Construct a `FieldMap` from a `HeaderMap` and a size limit.
    ///
    /// Construction with a `HeaderMap` which exceeds the size limit is
    /// allowed, but subsequent operations to expand the resource use will
    /// fail.
    pub fn new(map: HeaderMap, limit: usize) -> Self {
        let size = Self::content_size(&map);
        Self { map, size, limit }
    }
    /// Construct an empty `FieldMap`
    pub fn empty(limit: usize) -> Self {
        Self {
            map: HeaderMap::new(),
            size: 0,
            limit,
        }
    }
    /// Get the `HeaderMap` out of the `FieldMap`
    pub fn into_inner(self) -> HeaderMap {
        self.map
    }
    /// Calculate the content size of a `HeaderMap`. This is a sum of the size
    /// of all of the keys and all of the values.
    pub(crate) fn content_size(map: &HeaderMap) -> usize {
        let mut sum = 0;
        for key in map.keys() {
            sum += header_name_size(key);
        }
        for value in map.values() {
            sum += header_value_size(value);
        }
        sum
    }
    /// Remove all values associated with a key in a map.
    ///
    /// Returns an empty list if the key is not already present within the map.
    pub fn remove_all(&mut self, key: &HeaderName) -> Vec<HeaderValue> {
        use http::header::Entry;
        match self.map.try_entry(key) {
            Ok(Entry::Vacant { .. }) | Err(_) => Vec::new(),
            Ok(Entry::Occupied(e)) => {
                let (name, value_drain) = e.remove_entry_mult();
                let mut removed = header_name_size(&name);
                let values = value_drain.collect::<Vec<_>>();
                for v in values.iter() {
                    removed += header_value_size(v);
                }
                self.size -= removed;
                values
            }
        }
    }
    /// Add a value associated with a key to the map.
    ///
    /// If `key` is already present within the map then `value` is appended to
    /// the list of values it already has.
    pub fn append(&mut self, key: &HeaderName, value: HeaderValue) -> Result<bool> {
        let key_size = header_name_size(key);
        let val_size = header_value_size(&value);
        let new_size = if !self.map.contains_key(key) {
            self.size + key_size + val_size
        } else {
            self.size + val_size
        };
        if new_size > self.limit {
            bail!(FieldSizeLimitError {
                limit: self.limit,
                size: new_size
            })
        }
        self.size = new_size;
        Ok(self.map.try_append(key, value)?)
    }
}

/// Returns the size, in accounting cost, to consider for `name`.
///
/// This includes both the byte length of the `name` itself as well as the size
/// of the data structure itself as it'll reside within a `HeaderMap`.
fn header_name_size(name: &HeaderName) -> usize {
    name.as_str().len() + size_of::<HeaderName>()
}

/// Same as `header_name_size`, but for values.
///
/// This notably includes the size of `HeaderValue` itself to ensure that all
/// headers have a nonzero size as otherwise this would never limit addition of
/// an empty header value.
fn header_value_size(value: &HeaderValue) -> usize {
    value.len() + size_of::<HeaderValue>()
}

// We impl AsRef, but not AsMut, because any modifications of the
// underlying HeaderMap must account for changes in size
impl AsRef<HeaderMap> for FieldMap {
    fn as_ref(&self) -> &HeaderMap {
        &self.map
    }
}

/// A handle to a future incoming response.
pub type FutureIncomingResponseHandle =
    AbortOnDropJoinHandle<wasmtime::Result<Result<IncomingResponse, types::ErrorCode>>>;

/// A response that is in the process of being received.
#[derive(Debug)]
pub struct IncomingResponse {
    /// The response itself.
    pub resp: hyper::Response<HyperIncomingBody>,
    /// Optional worker task that continues to process the response.
    pub worker: Option<AbortOnDropJoinHandle<()>>,
    /// The timeout between chunks of the response.
    pub between_bytes_timeout: std::time::Duration,
}

/// The concrete type behind a `wasi:http/types.future-incoming-response` resource.
#[derive(Debug)]
pub enum HostFutureIncomingResponse {
    /// A pending response
    Pending(FutureIncomingResponseHandle),
    /// The response is ready.
    ///
    /// An outer error will trap while the inner error gets returned to the guest.
    Ready(wasmtime::Result<Result<IncomingResponse, types::ErrorCode>>),
    /// The response has been consumed.
    Consumed,
}

impl HostFutureIncomingResponse {
    /// Create a new `HostFutureIncomingResponse` that is pending on the provided task handle.
    pub fn pending(handle: FutureIncomingResponseHandle) -> Self {
        Self::Pending(handle)
    }

    /// Create a new `HostFutureIncomingResponse` that is ready.
    pub fn ready(result: wasmtime::Result<Result<IncomingResponse, types::ErrorCode>>) -> Self {
        Self::Ready(result)
    }

    /// Returns `true` if the response is ready.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// Unwrap the response, panicking if it is not ready.
    pub fn unwrap_ready(self) -> wasmtime::Result<Result<IncomingResponse, types::ErrorCode>> {
        match self {
            Self::Ready(res) => res,
            Self::Pending(_) | Self::Consumed => {
                panic!("unwrap_ready called on a pending HostFutureIncomingResponse")
            }
        }
    }
}

#[async_trait::async_trait]
impl Pollable for HostFutureIncomingResponse {
    async fn ready(&mut self) {
        if let Self::Pending(handle) = self {
            *self = Self::Ready(handle.await);
        }
    }
}

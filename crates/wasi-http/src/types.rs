//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::{
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
use wasmtime::component::{Resource, ResourceTable};
use wasmtime::{Result, bail};
use wasmtime_wasi::p2::Pollable;
use wasmtime_wasi::runtime::AbortOnDropJoinHandle;

#[cfg(feature = "default-send-request")]
use {
    crate::io::TokioIo,
    crate::{error::dns_error, hyper_request_error},
    tokio::net::TcpStream,
    tokio::time::timeout,
};

/// Default maximum size for the contents of a fields resource.
///
/// Typically, HTTP proxies limit headers to 8k. This number is higher than that
/// because it not only includes the wire-size of headers but it additionally
/// includes factors for the in-memory representation of `HeaderMap`. This is in
/// theory high enough that no one runs into it but low enough such that a
/// completely full `HeaderMap` doesn't break the bank in terms of memory
/// consumption.
const DEFAULT_FIELD_SIZE_LIMIT: usize = 128 * 1024;

/// Capture the state necessary for use in the wasi-http API implementation.
#[derive(Debug)]
pub struct WasiHttpCtx {
    pub(crate) field_size_limit: usize,
}

impl WasiHttpCtx {
    /// Create a new context.
    pub fn new() -> Self {
        Self {
            field_size_limit: DEFAULT_FIELD_SIZE_LIMIT,
        }
    }

    /// Set the maximum size for any fields resources created by this context.
    ///
    /// The limit specified here is roughly a byte limit for the size of the
    /// in-memory representation of headers. This means that the limit needs to
    /// be larger than the literal representation of headers on the wire to
    /// account for in-memory Rust-side data structures representing the header
    /// names/values/etc.
    pub fn set_field_size_limit(&mut self, limit: usize) {
        self.field_size_limit = limit;
    }
}

/// A trait which provides internal WASI HTTP state.
///
/// # Example
///
/// ```
/// use wasmtime::component::ResourceTable;
/// use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
/// use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};
///
/// struct MyState {
///     ctx: WasiCtx,
///     http_ctx: WasiHttpCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiHttpView for MyState {
///     fn ctx(&mut self) -> &mut WasiHttpCtx { &mut self.http_ctx }
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
///
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> WasiCtxView<'_> {
///         WasiCtxView { ctx: &mut self.ctx, table: &mut self.table }
///     }
/// }
///
/// impl MyState {
///     fn new() -> MyState {
///         let mut wasi = WasiCtx::builder();
///         wasi.arg("./foo.wasm");
///         wasi.arg("--help");
///         wasi.env("FOO", "bar");
///
///         MyState {
///             ctx: wasi.build(),
///             table: ResourceTable::new(),
///             http_ctx: WasiHttpCtx::new(),
///         }
///     }
/// }
/// ```
pub trait WasiHttpView {
    /// Returns a mutable reference to the WASI HTTP context.
    fn ctx(&mut self) -> &mut WasiHttpCtx;

    /// Returns the table used to manage resources.
    fn table(&mut self) -> &mut ResourceTable;

    /// Create a new incoming request resource.
    fn new_incoming_request<B>(
        &mut self,
        scheme: Scheme,
        req: hyper::Request<B>,
    ) -> wasmtime::Result<Resource<HostIncomingRequest>>
    where
        B: Body<Data = Bytes> + Send + 'static,
        B::Error: Into<ErrorCode>,
        Self: Sized,
    {
        let field_size_limit = self.ctx().field_size_limit;
        let (parts, body) = req.into_parts();
        let body = body.map_err(Into::into).boxed_unsync();
        let body = HostIncomingBody::new(
            body,
            // TODO: this needs to be plumbed through
            std::time::Duration::from_millis(600 * 1000),
            field_size_limit,
        );
        let incoming_req =
            HostIncomingRequest::new(self, parts, scheme, Some(body), field_size_limit)?;
        Ok(self.table().push(incoming_req)?)
    }

    /// Create a new outgoing response resource.
    fn new_response_outparam(
        &mut self,
        result: tokio::sync::oneshot::Sender<
            Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>,
        >,
    ) -> wasmtime::Result<Resource<HostResponseOutparam>> {
        let id = self.table().push(HostResponseOutparam { result })?;
        Ok(id)
    }

    /// Send an outgoing request.
    #[cfg(feature = "default-send-request")]
    fn send_request(
        &mut self,
        request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> crate::HttpResult<HostFutureIncomingResponse> {
        Ok(default_send_request(request, config))
    }

    /// Send an outgoing request.
    #[cfg(not(feature = "default-send-request"))]
    fn send_request(
        &mut self,
        request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> crate::HttpResult<HostFutureIncomingResponse>;

    /// Whether a given header should be considered forbidden and not allowed.
    fn is_forbidden_header(&mut self, name: &HeaderName) -> bool {
        DEFAULT_FORBIDDEN_HEADERS.contains(name)
    }

    /// Number of distinct write calls to the outgoing body's output-stream
    /// that the implementation will buffer.
    /// Default: 1.
    fn outgoing_body_buffer_chunks(&mut self) -> usize {
        DEFAULT_OUTGOING_BODY_BUFFER_CHUNKS
    }

    /// Maximum size allowed in a write call to the outgoing body's output-stream.
    /// Default: 1024 * 1024.
    fn outgoing_body_chunk_size(&mut self) -> usize {
        DEFAULT_OUTGOING_BODY_CHUNK_SIZE
    }
}

/// The default value configured for [`WasiHttpView::outgoing_body_buffer_chunks`] in [`WasiHttpView`].
pub const DEFAULT_OUTGOING_BODY_BUFFER_CHUNKS: usize = 1;
/// The default value configured for [`WasiHttpView::outgoing_body_chunk_size`] in [`WasiHttpView`].
pub const DEFAULT_OUTGOING_BODY_CHUNK_SIZE: usize = 1024 * 1024;

impl<T: ?Sized + WasiHttpView> WasiHttpView for &mut T {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        T::ctx(self)
    }

    fn table(&mut self) -> &mut ResourceTable {
        T::table(self)
    }

    fn new_response_outparam(
        &mut self,
        result: tokio::sync::oneshot::Sender<
            Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>,
        >,
    ) -> wasmtime::Result<Resource<HostResponseOutparam>> {
        T::new_response_outparam(self, result)
    }

    fn send_request(
        &mut self,
        request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> crate::HttpResult<HostFutureIncomingResponse> {
        T::send_request(self, request, config)
    }

    fn is_forbidden_header(&mut self, name: &HeaderName) -> bool {
        T::is_forbidden_header(self, name)
    }

    fn outgoing_body_buffer_chunks(&mut self) -> usize {
        T::outgoing_body_buffer_chunks(self)
    }

    fn outgoing_body_chunk_size(&mut self) -> usize {
        T::outgoing_body_chunk_size(self)
    }
}

impl<T: ?Sized + WasiHttpView> WasiHttpView for Box<T> {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        T::ctx(self)
    }

    fn table(&mut self) -> &mut ResourceTable {
        T::table(self)
    }

    fn new_response_outparam(
        &mut self,
        result: tokio::sync::oneshot::Sender<
            Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>,
        >,
    ) -> wasmtime::Result<Resource<HostResponseOutparam>> {
        T::new_response_outparam(self, result)
    }

    fn send_request(
        &mut self,
        request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> crate::HttpResult<HostFutureIncomingResponse> {
        T::send_request(self, request, config)
    }

    fn is_forbidden_header(&mut self, name: &HeaderName) -> bool {
        T::is_forbidden_header(self, name)
    }

    fn outgoing_body_buffer_chunks(&mut self) -> usize {
        T::outgoing_body_buffer_chunks(self)
    }

    fn outgoing_body_chunk_size(&mut self) -> usize {
        T::outgoing_body_chunk_size(self)
    }
}

/// A concrete structure that all generated `Host` traits are implemented for.
///
/// This type serves as a small newtype wrapper to implement all of the `Host`
/// traits for `wasi:http`. This type is internally used and is only needed if
/// you're interacting with `add_to_linker` functions generated by bindings
/// themselves (or `add_to_linker_get_host`).
///
/// This type is automatically used when using
/// [`add_to_linker_async`](crate::add_to_linker_async)
/// or
/// [`add_to_linker_sync`](crate::add_to_linker_sync)
/// and doesn't need to be manually configured.
#[repr(transparent)]
pub struct WasiHttpImpl<T>(pub T);

impl<T: WasiHttpView> WasiHttpView for WasiHttpImpl<T> {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        self.0.ctx()
    }

    fn table(&mut self) -> &mut ResourceTable {
        self.0.table()
    }

    fn new_response_outparam(
        &mut self,
        result: tokio::sync::oneshot::Sender<
            Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>,
        >,
    ) -> wasmtime::Result<Resource<HostResponseOutparam>> {
        self.0.new_response_outparam(result)
    }

    fn send_request(
        &mut self,
        request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> crate::HttpResult<HostFutureIncomingResponse> {
        self.0.send_request(request, config)
    }

    fn is_forbidden_header(&mut self, name: &HeaderName) -> bool {
        self.0.is_forbidden_header(name)
    }

    fn outgoing_body_buffer_chunks(&mut self) -> usize {
        self.0.outgoing_body_buffer_chunks()
    }

    fn outgoing_body_chunk_size(&mut self) -> usize {
        self.0.outgoing_body_chunk_size()
    }
}

/// Set of [http::header::HeaderName], that are forbidden by default
/// for requests and responses originating in the guest.
pub const DEFAULT_FORBIDDEN_HEADERS: [http::header::HeaderName; 9] = [
    hyper::header::CONNECTION,
    HeaderName::from_static("keep-alive"),
    hyper::header::PROXY_AUTHENTICATE,
    hyper::header::PROXY_AUTHORIZATION,
    HeaderName::from_static("proxy-connection"),
    hyper::header::TRANSFER_ENCODING,
    hyper::header::UPGRADE,
    hyper::header::HOST,
    HeaderName::from_static("http2-settings"),
];

/// Removes forbidden headers from a [`FieldMap`].
pub(crate) fn remove_forbidden_headers(view: &mut dyn WasiHttpView, headers: &mut FieldMap) {
    let forbidden_keys = Vec::from_iter(headers.as_ref().keys().filter_map(|name| {
        if view.is_forbidden_header(name) {
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

/// The default implementation of how an outgoing request is sent.
///
/// This implementation is used by the `wasi:http/outgoing-handler` interface
/// default implementation.
#[cfg(feature = "default-send-request")]
pub fn default_send_request(
    request: hyper::Request<HyperOutgoingBody>,
    config: OutgoingRequestConfig,
) -> HostFutureIncomingResponse {
    let handle = wasmtime_wasi::runtime::spawn(async move {
        Ok(default_send_request_handler(request, config).await)
    });
    HostFutureIncomingResponse::pending(handle)
}

/// The underlying implementation of how an outgoing request is sent. This should likely be spawned
/// in a task.
///
/// This is called from [default_send_request] to actually send the request.
#[cfg(feature = "default-send-request")]
pub async fn default_send_request_handler(
    mut request: hyper::Request<HyperOutgoingBody>,
    OutgoingRequestConfig {
        use_tls,
        connect_timeout,
        first_byte_timeout,
        between_bytes_timeout,
    }: OutgoingRequestConfig,
) -> Result<IncomingResponse, types::ErrorCode> {
    let authority = if let Some(authority) = request.uri().authority() {
        if authority.port().is_some() {
            authority.to_string()
        } else {
            let port = if use_tls { 443 } else { 80 };
            format!("{}:{port}", authority.to_string())
        }
    } else {
        return Err(types::ErrorCode::HttpRequestUriInvalid);
    };
    let tcp_stream = timeout(connect_timeout, TcpStream::connect(&authority))
        .await
        .map_err(|_| types::ErrorCode::ConnectionTimeout)?
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AddrNotAvailable => {
                dns_error("address not available".to_string(), 0)
            }

            _ => {
                if e.to_string()
                    .starts_with("failed to lookup address information")
                {
                    dns_error("address not available".to_string(), 0)
                } else {
                    types::ErrorCode::ConnectionRefused
                }
            }
        })?;

    let (mut sender, worker) = if use_tls {
        use rustls::pki_types::ServerName;

        // derived from https://github.com/rustls/rustls/blob/main/examples/src/bin/simpleclient.rs
        let root_cert_store = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.into(),
        };
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
        let mut parts = authority.split(":");
        let host = parts.next().unwrap_or(&authority);
        let domain = ServerName::try_from(host)
            .map_err(|e| {
                tracing::warn!("dns lookup error: {e:?}");
                dns_error("invalid dns name".to_string(), 0)
            })?
            .to_owned();
        let stream = connector.connect(domain, tcp_stream).await.map_err(|e| {
            tracing::warn!("tls protocol error: {e:?}");
            types::ErrorCode::TlsProtocolError
        })?;
        let stream = TokioIo::new(stream);

        let (sender, conn) = timeout(
            connect_timeout,
            hyper::client::conn::http1::handshake(stream),
        )
        .await
        .map_err(|_| types::ErrorCode::ConnectionTimeout)?
        .map_err(hyper_request_error)?;

        let worker = wasmtime_wasi::runtime::spawn(async move {
            match conn.await {
                Ok(()) => {}
                // TODO: shouldn't throw away this error and ideally should
                // surface somewhere.
                Err(e) => tracing::warn!("dropping error {e}"),
            }
        });

        (sender, worker)
    } else {
        let tcp_stream = TokioIo::new(tcp_stream);
        let (sender, conn) = timeout(
            connect_timeout,
            // TODO: we should plumb the builder through the http context, and use it here
            hyper::client::conn::http1::handshake(tcp_stream),
        )
        .await
        .map_err(|_| types::ErrorCode::ConnectionTimeout)?
        .map_err(hyper_request_error)?;

        let worker = wasmtime_wasi::runtime::spawn(async move {
            match conn.await {
                Ok(()) => {}
                // TODO: same as above, shouldn't throw this error away.
                Err(e) => tracing::warn!("dropping error {e}"),
            }
        });

        (sender, worker)
    };

    // at this point, the request contains the scheme and the authority, but
    // the http packet should only include those if addressing a proxy, so
    // remove them here, since SendRequest::send_request does not do it for us
    *request.uri_mut() = http::Uri::builder()
        .path_and_query(
            request
                .uri()
                .path_and_query()
                .map(|p| p.as_str())
                .unwrap_or("/"),
        )
        .build()
        .expect("comes from valid request");

    let resp = timeout(first_byte_timeout, sender.send_request(request))
        .await
        .map_err(|_| types::ErrorCode::ConnectionReadTimeout)?
        .map_err(hyper_request_error)?
        .map(|body| body.map_err(hyper_request_error).boxed_unsync());

    Ok(IncomingResponse {
        resp,
        worker: Some(worker),
        between_bytes_timeout,
    })
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

impl HostIncomingRequest {
    /// Create a new `HostIncomingRequest`.
    pub fn new(
        view: &mut dyn WasiHttpView,
        parts: http::request::Parts,
        scheme: Scheme,
        body: Option<HostIncomingBody>,
        field_size_limit: usize,
    ) -> wasmtime::Result<Self> {
        let authority = match parts.uri.authority() {
            Some(authority) => authority.to_string(),
            None => match parts.headers.get(http::header::HOST) {
                Some(host) => host.to_str()?.to_string(),
                None => bail!("invalid HTTP request missing authority in URI and host header"),
            },
        };

        let mut headers = FieldMap::new(parts.headers, field_size_limit);
        remove_forbidden_headers(view, &mut headers);

        Ok(Self {
            method: parts.method,
            uri: parts.uri,
            headers,
            authority,
            scheme,
            body,
        })
    }
}

/// The concrete type behind a `wasi:http/types.response-outparam` resource.
pub struct HostResponseOutparam {
    /// The sender for sending a response.
    pub result:
        tokio::sync::oneshot::Sender<Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>>,
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

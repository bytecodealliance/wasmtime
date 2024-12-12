//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::io::TokioIo;
use crate::{
    bindings::http::types::{self, Method, Scheme},
    body::{HostIncomingBody, HyperIncomingBody, HyperOutgoingBody},
    error::dns_error,
    hyper_request_error,
};
use anyhow::bail;
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Body;
use hyper::header::HeaderName;
use std::any::Any;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi::{runtime::AbortOnDropJoinHandle, Subscribe};

/// Capture the state necessary for use in the wasi-http API implementation.
#[derive(Debug)]
pub struct WasiHttpCtx {
    _priv: (),
}

impl WasiHttpCtx {
    /// Create a new context.
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

/// A trait which provides internal WASI HTTP state.
///
/// # Example
///
/// ```
/// use wasmtime::component::ResourceTable;
/// use wasmtime_wasi::{WasiCtx, WasiView, WasiCtxBuilder};
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
///     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
///
/// impl MyState {
///     fn new() -> MyState {
///         let mut wasi = WasiCtxBuilder::new();
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
pub trait WasiHttpView: Send {
    /// Returns a mutable reference to the WASI HTTP context.
    fn ctx(&mut self) -> &mut WasiHttpCtx;

    /// Returns a mutable reference to the WASI HTTP resource table.
    fn table(&mut self) -> &mut ResourceTable;

    /// Create a new incoming request resource.
    fn new_incoming_request<B>(
        &mut self,
        scheme: Scheme,
        req: hyper::Request<B>,
    ) -> wasmtime::Result<Resource<HostIncomingRequest>>
    where
        B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static,
        Self: Sized,
    {
        let (parts, body) = req.into_parts();
        let body = body.map_err(crate::hyper_response_error).boxed();
        let body = HostIncomingBody::new(
            body,
            // TODO: this needs to be plumbed through
            std::time::Duration::from_millis(600 * 1000),
        );
        let incoming_req = HostIncomingRequest::new(self, parts, scheme, Some(body))?;
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
    fn send_request(
        &mut self,
        request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> crate::HttpResult<HostFutureIncomingResponse> {
        Ok(default_send_request(request, config))
    }

    /// Whether a given header should be considered forbidden and not allowed.
    fn is_forbidden_header(&mut self, _name: &HeaderName) -> bool {
        false
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

/// Returns `true` when the header is forbidden according to this [`WasiHttpView`] implementation.
pub(crate) fn is_forbidden_header(view: &mut dyn WasiHttpView, name: &HeaderName) -> bool {
    static FORBIDDEN_HEADERS: [HeaderName; 10] = [
        hyper::header::CONNECTION,
        HeaderName::from_static("keep-alive"),
        hyper::header::PROXY_AUTHENTICATE,
        hyper::header::PROXY_AUTHORIZATION,
        HeaderName::from_static("proxy-connection"),
        hyper::header::TE,
        hyper::header::TRANSFER_ENCODING,
        hyper::header::UPGRADE,
        hyper::header::HOST,
        HeaderName::from_static("http2-settings"),
    ];

    FORBIDDEN_HEADERS.contains(name) || view.is_forbidden_header(name)
}

/// Removes forbidden headers from a [`hyper::HeaderMap`].
pub(crate) fn remove_forbidden_headers(
    view: &mut dyn WasiHttpView,
    headers: &mut hyper::HeaderMap,
) {
    let forbidden_keys = Vec::from_iter(headers.keys().filter_map(|name| {
        if is_forbidden_header(view, name) {
            Some(name.clone())
        } else {
            None
        }
    }));

    for name in forbidden_keys {
        headers.remove(name);
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
        #[cfg(any(target_arch = "riscv64", target_arch = "s390x"))]
        {
            return Err(crate::bindings::http::types::ErrorCode::InternalError(
                Some("unsupported architecture for SSL".to_string()),
            ));
        }

        #[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
        {
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
        }
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
        .map(|body| body.map_err(hyper_request_error).boxed());

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

/// The concrete type behind a `wasi:http/types/incoming-request` resource.
#[derive(Debug)]
pub struct HostIncomingRequest {
    pub(crate) parts: http::request::Parts,
    pub(crate) scheme: Scheme,
    pub(crate) authority: String,
    /// The body of the incoming request.
    pub body: Option<HostIncomingBody>,
}

impl HostIncomingRequest {
    /// Create a new `HostIncomingRequest`.
    pub fn new(
        view: &mut dyn WasiHttpView,
        mut parts: http::request::Parts,
        scheme: Scheme,
        body: Option<HostIncomingBody>,
    ) -> anyhow::Result<Self> {
        let authority = match parts.uri.authority() {
            Some(authority) => authority.to_string(),
            None => match parts.headers.get(http::header::HOST) {
                Some(host) => host.to_str()?.to_string(),
                None => bail!("invalid HTTP request missing authority in URI and host header"),
            },
        };

        remove_forbidden_headers(view, &mut parts.headers);
        Ok(Self {
            parts,
            authority,
            scheme,
            body,
        })
    }
}

/// The concrete type behind a `wasi:http/types/response-outparam` resource.
pub struct HostResponseOutparam {
    /// The sender for sending a response.
    pub result:
        tokio::sync::oneshot::Sender<Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>>,
}

/// The concrete type behind a `wasi:http/types/outgoing-response` resource.
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

        *builder.headers_mut().unwrap() = resp.headers;

        match resp.body {
            Some(body) => builder.body(body),
            None => builder.body(
                Empty::<bytes::Bytes>::new()
                    .map_err(|_| unreachable!("Infallible error"))
                    .boxed(),
            ),
        }
    }
}

/// The concrete type behind a `wasi:http/types/outgoing-request` resource.
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

/// The concrete type behind a `wasi:http/types/request-options` resource.
#[derive(Debug, Default)]
pub struct HostRequestOptions {
    /// How long to wait for a connection to be established.
    pub connect_timeout: Option<std::time::Duration>,
    /// How long to wait for the first byte of the response body.
    pub first_byte_timeout: Option<std::time::Duration>,
    /// How long to wait between frames of the response body.
    pub between_bytes_timeout: Option<std::time::Duration>,
}

/// The concrete type behind a `wasi:http/types/incoming-response` resource.
#[derive(Debug)]
pub struct HostIncomingResponse {
    /// The response status
    pub status: u16,
    /// The response headers
    pub headers: FieldMap,
    /// The response body
    pub body: Option<HostIncomingBody>,
}

/// The concrete type behind a `wasi:http/types/fields` resource.
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

/// An owned version of `HostFields`
pub type FieldMap = hyper::HeaderMap;

/// A handle to a future incoming response.
pub type FutureIncomingResponseHandle =
    AbortOnDropJoinHandle<anyhow::Result<Result<IncomingResponse, types::ErrorCode>>>;

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

/// The concrete type behind a `wasi:http/types/future-incoming-response` resource.
#[derive(Debug)]
pub enum HostFutureIncomingResponse {
    /// A pending response
    Pending(FutureIncomingResponseHandle),
    /// The response is ready.
    ///
    /// An outer error will trap while the inner error gets returned to the guest.
    Ready(anyhow::Result<Result<IncomingResponse, types::ErrorCode>>),
    /// The response has been consumed.
    Consumed,
}

impl HostFutureIncomingResponse {
    /// Create a new `HostFutureIncomingResponse` that is pending on the provided task handle.
    pub fn pending(handle: FutureIncomingResponseHandle) -> Self {
        Self::Pending(handle)
    }

    /// Create a new `HostFutureIncomingResponse` that is ready.
    pub fn ready(result: anyhow::Result<Result<IncomingResponse, types::ErrorCode>>) -> Self {
        Self::Ready(result)
    }

    /// Returns `true` if the response is ready.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// Unwrap the response, panicking if it is not ready.
    pub fn unwrap_ready(self) -> anyhow::Result<Result<IncomingResponse, types::ErrorCode>> {
        match self {
            Self::Ready(res) => res,
            Self::Pending(_) | Self::Consumed => {
                panic!("unwrap_ready called on a pending HostFutureIncomingResponse")
            }
        }
    }
}

#[async_trait::async_trait]
impl Subscribe for HostFutureIncomingResponse {
    async fn ready(&mut self) {
        if let Self::Pending(handle) = self {
            *self = Self::Ready(handle.await);
        }
    }
}

//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::io::TokioIo;
use crate::{
    bindings::http::types::{self, Method, Scheme},
    body::{HostIncomingBody, HyperIncomingBody, HyperOutgoingBody},
    error::dns_error,
    hyper_request_error,
};
use http_body_util::BodyExt;
use hyper::header::HeaderName;
use std::any::Any;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi::{runtime::AbortOnDropJoinHandle, Subscribe};

/// Capture the state necessary for use in the wasi-http API implementation.
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
pub trait WasiHttpView {
    /// Returns a mutable reference to the WASI HTTP context.
    fn ctx(&mut self) -> &mut WasiHttpCtx;

    /// Returns a mutable reference to the WASI HTTP resource table.
    fn table(&mut self) -> &mut ResourceTable;

    /// Create a new incoming request resource.
    fn new_incoming_request(
        &mut self,
        req: hyper::Request<HyperIncomingBody>,
    ) -> wasmtime::Result<Resource<HostIncomingRequest>>
    where
        Self: Sized,
    {
        let (parts, body) = req.into_parts();
        let body = HostIncomingBody::new(
            body,
            // TODO: this needs to be plumbed through
            std::time::Duration::from_millis(600 * 1000),
        );
        let incoming_req = HostIncomingRequest::new(self, parts, Some(body));
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
}

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
    let Some(authority) = request.uri().authority().map(ToString::to_string) else {
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
            use rustls::pki_types::{ServerName, TrustAnchor};

            // derived from https://github.com/tokio-rs/tls/blob/master/tokio-rustls/examples/client/src/main.rs
            let mut root_cert_store = rustls::RootCertStore::empty();
            root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| TrustAnchor {
                name_constraints: ta.name_constraints.to_owned(),
                subject: ta.subject.to_owned(),
                subject_public_key_info: ta.subject_public_key_info.to_owned(),
            }));
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
pub struct HostIncomingRequest {
    pub(crate) parts: http::request::Parts,
    /// The body of the incoming request.
    pub body: Option<HostIncomingBody>,
}

impl HostIncomingRequest {
    /// Create a new `HostIncomingRequest`.
    pub fn new(
        view: &mut dyn WasiHttpView,
        mut parts: http::request::Parts,
        body: Option<HostIncomingBody>,
    ) -> Self {
        remove_forbidden_headers(view, &mut parts.headers);
        Self { parts, body }
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
#[derive(Default)]
pub struct HostRequestOptions {
    /// How long to wait for a connection to be established.
    pub connect_timeout: Option<std::time::Duration>,
    /// How long to wait for the first byte of the response body.
    pub first_byte_timeout: Option<std::time::Duration>,
    /// How long to wait between frames of the response body.
    pub between_bytes_timeout: Option<std::time::Duration>,
}

/// The concrete type behind a `wasi:http/types/incoming-response` resource.
pub struct HostIncomingResponse {
    /// The response status
    pub status: u16,
    /// The response headers
    pub headers: FieldMap,
    /// The response body
    pub body: Option<HostIncomingBody>,
}

/// The concrete type behind a `wasi:http/types/fields` resource.
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
pub struct IncomingResponse {
    /// The response itself.
    pub resp: hyper::Response<HyperIncomingBody>,
    /// Optional worker task that continues to process the response.
    pub worker: Option<AbortOnDropJoinHandle<()>>,
    /// The timeout between chunks of the response.
    pub between_bytes_timeout: std::time::Duration,
}

/// The concrete type behind a `wasi:http/types/future-incoming-response` resource.
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

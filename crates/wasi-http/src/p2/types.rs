//! Implements the base structure that will provide the implementation of the
//! wasi-http API.

use crate::p2::{
    bindings::http::types::{self, Method, Scheme},
    body::{HostIncomingBody, HyperIncomingBody, HyperOutgoingBody},
};
use crate::{Error, ErrorResponse, FieldMap, WasiHttpCtxView};
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Body;
use wasmtime::component::Resource;
use wasmtime::error::{Context, Result};
use wasmtime_wasi::p2::Pollable;
use wasmtime_wasi::runtime::AbortOnDropJoinHandle;

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
        B::Error: Into<Error>,
    {
        let (parts, body) = req.into_parts();
        let parts = normalize_authority(parts, &scheme)
            .with_context(|| ErrorResponse::new(http::StatusCode::BAD_REQUEST))?;
        let body = body.map_err(Into::into).boxed_unsync();
        let body = HostIncomingBody::new(body);

        let mut validator = crate::request_strings::RequestStringsValidator::new(self.ctx);
        validator.host_parts(
            &parts.method,
            parts.uri.scheme(),
            parts.uri.authority(),
            parts.uri.path_and_query(),
        )?;
        let headers = FieldMap::new_immutable(self.hooks, parts.headers);

        let req = HostIncomingRequest {
            method: parts.method,
            uri: parts.uri,
            headers,
            scheme,
            body: Some(body),
        };
        Ok(self.table.push(req)?)
    }
}

/// Ensure `parts.uri` has an authority, taking it from the `Host` header if
/// necessary. A URI with an authority must also have a scheme, so `scheme`
/// fills it in when the URI lacks one.
///
/// All failures in this function get propogated as a BAD_REQUEST error from the
/// call site.
fn normalize_authority(
    parts: http::request::Parts,
    scheme: &Scheme,
) -> Result<http::request::Parts> {
    if parts.uri.authority().is_some() {
        return Ok(parts);
    }
    if parts.headers.get(http::header::HOST).is_none() {
        return Err(wasmtime::Error::msg(
            "invalid HTTP request missing authority in URI and host header",
        ));
    }
    let host: http::uri::Authority = parts
        .headers
        .get(http::header::HOST)
        .unwrap()
        .to_str()?
        .parse()?;
    let mut parts = parts;
    let mut uri = parts.uri.into_parts();
    uri.authority = Some(host);
    if uri.scheme.is_none() {
        uri.scheme = Some(match scheme {
            Scheme::Http => http::uri::Scheme::HTTP,
            Scheme::Https => http::uri::Scheme::HTTPS,
            Scheme::Other(s) => s.parse()?,
        });
    }
    parts.uri = http::uri::Uri::from_parts(uri)?;
    Ok(parts)
}

/// The concrete type behind a `wasi:http/types.response-outparam` resource.
pub struct HostResponseOutparam {
    /// The callback sending a response.
    pub send:
        Box<dyn FnOnce(Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>) + Send + Sync>,
}

impl WasiHttpCtxView<'_> {
    /// Create a new outgoing response resource.
    pub fn new_response_outparam(
        &mut self,
        result: tokio::sync::oneshot::Sender<
            Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>,
        >,
    ) -> wasmtime::Result<Resource<HostResponseOutparam>> {
        let id = self.table.push(HostResponseOutparam {
            send: Box::new(move |value| {
                // Giving the API doesn't return any error, it's probably
                // better to ignore the error than trap the guest, in case of
                // host timeout and dropped the receiver side of the channel.
                // See also: #10784
                _ = result.send(value)
            }),
        })?;
        Ok(id)
    }

    /// Create a new outgoing response from an `FnOnce`.
    pub fn new_response_outparam_from_callback(
        &mut self,
        callback: impl FnOnce(Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>)
        + Send
        + Sync
        + 'static,
    ) -> wasmtime::Result<Resource<HostResponseOutparam>> {
        let id = self.table.push(HostResponseOutparam {
            send: Box::new(callback),
        })?;
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

        *builder.headers_mut().unwrap() = resp.headers.into();

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
    /// Accounting for the size of the method, scheme, authority, and path.
    pub(crate) strings: crate::request_strings::RequestStringsValidator,
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

/// A handle to a future incoming response.
pub type FutureIncomingResponseHandle = AbortOnDropJoinHandle<SendRequestResult>;

/// A response that is in the process of being received.
#[derive(Debug)]
pub struct IncomingResponse {
    /// The response itself.
    pub resp: hyper::Response<HyperIncomingBody>,
    /// Optional worker task that continues to process the response.
    pub worker: Option<AbortOnDropJoinHandle<()>>,
}

type SendRequestResult =
    crate::Result<(http::Response<HyperIncomingBody>, AbortOnDropJoinHandle<()>)>;

/// The concrete type behind a `wasi:http/types.future-incoming-response` resource.
pub enum HostFutureIncomingResponse {
    /// A pending response
    Pending(FutureIncomingResponseHandle),
    /// The response is ready.
    ///
    /// An outer error will trap while the inner error gets returned to the guest.
    Ready(SendRequestResult),
    /// The response has been consumed.
    Consumed,
}

impl HostFutureIncomingResponse {
    /// Unwrap the response, panicking if it is not ready.
    pub(crate) fn unwrap_ready(self) -> SendRequestResult {
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

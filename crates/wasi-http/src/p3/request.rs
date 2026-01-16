use crate::get_content_length;
use crate::p3::bindings::http::types::ErrorCode;
use crate::p3::body::{Body, BodyExt as _, GuestBody};
use crate::p3::{WasiHttpCtxView, WasiHttpView};
use bytes::Bytes;
use core::time::Duration;
use http::header::HOST;
use http::uri::{Authority, PathAndQuery, Scheme};
use http::{HeaderMap, HeaderValue, Method, Uri};
use http_body_util::BodyExt as _;
use http_body_util::combinators::UnsyncBoxBody;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::debug;
use wasmtime::AsContextMut;

/// The concrete type behind a `wasi:http/types.request-options` resource.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct RequestOptions {
    /// How long to wait for a connection to be established.
    pub connect_timeout: Option<Duration>,
    /// How long to wait for the first byte of the response body.
    pub first_byte_timeout: Option<Duration>,
    /// How long to wait between frames of the response body.
    pub between_bytes_timeout: Option<Duration>,
}

/// The concrete type behind a `wasi:http/types.request` resource.
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

impl Request {
    /// Construct a new [Request]
    ///
    /// This returns a [Future] that the will be used to communicate
    /// a request processing error, if any.
    ///
    /// Requests constructed this way will not perform any `Content-Length` validation.
    pub fn new(
        method: Method,
        scheme: Option<Scheme>,
        authority: Option<Authority>,
        path_with_query: Option<PathAndQuery>,
        headers: impl Into<Arc<HeaderMap>>,
        options: Option<Arc<RequestOptions>>,
        body: impl Into<UnsyncBoxBody<Bytes, ErrorCode>>,
    ) -> (
        Self,
        impl Future<Output = Result<(), ErrorCode>> + Send + 'static,
    ) {
        let (tx, rx) = oneshot::channel();
        (
            Self {
                method,
                scheme,
                authority,
                path_with_query,
                headers: headers.into(),
                options,
                body: Body::Host {
                    body: body.into(),
                    result_tx: tx,
                },
            },
            async {
                let Ok(fut) = rx.await else { return Ok(()) };
                Box::into_pin(fut).await
            },
        )
    }

    /// Construct a new [Request] from [http::Request].
    ///
    /// This returns a [Future] that will be used to communicate
    /// a request processing error, if any.
    ///
    /// Requests constructed this way will not perform any `Content-Length` validation.
    pub fn from_http<T>(
        req: http::Request<T>,
    ) -> (
        Self,
        impl Future<Output = Result<(), ErrorCode>> + Send + 'static,
    )
    where
        T: http_body::Body<Data = Bytes> + Send + 'static,
        T::Error: Into<ErrorCode>,
    {
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
            body.map_err(Into::into).boxed_unsync(),
        )
    }

    /// Convert this [`Request`] into an [`http::Request<UnsyncBoxBody<Bytes, ErrorCode>>`].
    ///
    /// The specified future `fut` can be used to communicate a request processing
    /// error, if any, back to the caller (e.g., if this request was constructed
    /// through `wasi:http/types.request#new`).
    pub fn into_http<T: WasiHttpView + 'static>(
        self,
        store: impl AsContextMut<Data = T>,
        fut: impl Future<Output = Result<(), ErrorCode>> + Send + 'static,
    ) -> Result<
        (
            http::Request<UnsyncBoxBody<Bytes, ErrorCode>>,
            Option<Arc<RequestOptions>>,
        ),
        ErrorCode,
    > {
        self.into_http_with_getter(store, fut, T::http)
    }

    /// Like [`Self::into_http`], but uses a custom getter for obtaining the [`WasiHttpCtxView`].
    pub fn into_http_with_getter<T: 'static>(
        self,
        mut store: impl AsContextMut<Data = T>,
        fut: impl Future<Output = Result<(), ErrorCode>> + Send + 'static,
        getter: fn(&mut T) -> WasiHttpCtxView<'_>,
    ) -> Result<
        (
            http::Request<UnsyncBoxBody<Bytes, ErrorCode>>,
            Option<Arc<RequestOptions>>,
        ),
        ErrorCode,
    > {
        let Request {
            method,
            scheme,
            authority,
            path_with_query,
            headers,
            options,
            body,
        } = self;
        // `Content-Length` header value is validated in `fields` implementation
        let content_length = match get_content_length(&headers) {
            Ok(content_length) => content_length,
            Err(err) => {
                body.drop(&mut store);
                return Err(ErrorCode::InternalError(Some(format!("{err:#}"))));
            }
        };
        // This match must appear before any potential errors handled with '?'
        // (or errors have to explicitly be addressed and drop the body, as above),
        // as otherwise the Body::Guest resources will not be cleaned up when dropped.
        // see: https://github.com/bytecodealliance/wasmtime/pull/11440#discussion_r2326139381
        // for additional context.
        let body = match body {
            Body::Guest {
                contents_rx,
                trailers_rx,
                result_tx,
            } => GuestBody::new(
                &mut store,
                contents_rx,
                trailers_rx,
                result_tx,
                fut,
                content_length,
                ErrorCode::HttpRequestBodySize,
                getter,
            )
            .boxed_unsync(),
            Body::Host { body, result_tx } => {
                if let Some(limit) = content_length {
                    let (http_result_tx, http_result_rx) = oneshot::channel();
                    _ = result_tx.send(Box::new(async move {
                        if let Ok(err) = http_result_rx.await {
                            return Err(err);
                        };
                        fut.await
                    }));
                    body.with_content_length(limit, http_result_tx, ErrorCode::HttpRequestBodySize)
                        .boxed_unsync()
                } else {
                    _ = result_tx.send(Box::new(fut));
                    body
                }
            }
        };
        let mut headers = Arc::unwrap_or_clone(headers);
        let mut store = store.as_context_mut();
        let WasiHttpCtxView { ctx, .. } = getter(store.data_mut());
        if ctx.set_host_header() {
            let host = if let Some(authority) = authority.as_ref() {
                HeaderValue::try_from(authority.as_str())
                    .map_err(|err| ErrorCode::InternalError(Some(err.to_string())))?
            } else {
                HeaderValue::from_static("")
            };
            headers.insert(HOST, host);
        }
        let scheme = match scheme {
            None => ctx.default_scheme().ok_or(ErrorCode::HttpProtocolError)?,
            Some(scheme) if ctx.is_supported_scheme(&scheme) => scheme,
            Some(..) => return Err(ErrorCode::HttpProtocolError),
        };
        let mut uri = Uri::builder().scheme(scheme);
        if let Some(authority) = authority {
            uri = uri.authority(authority)
        };
        if let Some(path_with_query) = path_with_query {
            uri = uri.path_and_query(path_with_query)
        };
        let uri = uri.build().map_err(|err| {
            debug!(?err, "failed to build request URI");
            ErrorCode::HttpRequestUriInvalid
        })?;
        let mut req = http::Request::builder();
        *req.headers_mut().unwrap() = headers;
        let req = req
            .method(method)
            .uri(uri)
            .body(body)
            .map_err(|err| ErrorCode::InternalError(Some(err.to_string())))?;
        let (req, body) = req.into_parts();
        Ok((http::Request::from_parts(req, body), options))
    }
}

/// The default implementation of how an outgoing request is sent.
///
/// This implementation is used by the `wasi:http/handler` interface
/// default implementation.
///
/// The returned [Future] can be used to communicate
/// a request processing error, if any, to the constructor of the request.
/// For example, if the request was constructed via `wasi:http/types.request#new`,
/// a result resolved from it will be forwarded to the guest on the future handle returned.
///
/// This function performs no `Content-Length` validation.
#[cfg(feature = "default-send-request")]
pub async fn default_send_request(
    mut req: http::Request<impl http_body::Body<Data = Bytes, Error = ErrorCode> + Send + 'static>,
    options: Option<RequestOptions>,
) -> Result<
    (
        http::Response<impl http_body::Body<Data = Bytes, Error = ErrorCode>>,
        impl Future<Output = Result<(), ErrorCode>> + Send,
    ),
    ErrorCode,
> {
    use core::future::poll_fn;
    use core::pin::{Pin, pin};
    use core::task::{Poll, ready};
    use tokio::io::{AsyncRead, AsyncWrite};
    use tokio::net::TcpStream;

    trait TokioStream: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static {
        fn boxed(self) -> Box<dyn TokioStream>
        where
            Self: Sized,
        {
            Box::new(self)
        }
    }
    impl<T> TokioStream for T where T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static {}

    fn dns_error(rcode: String, info_code: u16) -> ErrorCode {
        ErrorCode::DnsError(crate::p3::bindings::http::types::DnsErrorPayload {
            rcode: Some(rcode),
            info_code: Some(info_code),
        })
    }

    let uri = req.uri();
    let authority = uri.authority().ok_or(ErrorCode::HttpRequestUriInvalid)?;
    let use_tls = uri.scheme() == Some(&Scheme::HTTPS);
    let authority = if authority.port().is_some() {
        authority.to_string()
    } else {
        let port = if use_tls { 443 } else { 80 };
        format!("{authority}:{port}")
    };

    let connect_timeout = options
        .and_then(
            |RequestOptions {
                 connect_timeout, ..
             }| connect_timeout,
        )
        .unwrap_or(Duration::from_secs(600));

    let first_byte_timeout = options
        .and_then(
            |RequestOptions {
                 first_byte_timeout, ..
             }| first_byte_timeout,
        )
        .unwrap_or(Duration::from_secs(600));

    let between_bytes_timeout = options
        .and_then(
            |RequestOptions {
                 between_bytes_timeout,
                 ..
             }| between_bytes_timeout,
        )
        .unwrap_or(Duration::from_secs(600));

    let stream = match tokio::time::timeout(connect_timeout, TcpStream::connect(&authority)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(err)) if err.kind() == std::io::ErrorKind::AddrNotAvailable => {
            return Err(dns_error("address not available".to_string(), 0));
        }
        Ok(Err(err))
            if err
                .to_string()
                .starts_with("failed to lookup address information") =>
        {
            return Err(dns_error("address not available".to_string(), 0));
        }
        Ok(Err(err)) => {
            tracing::warn!(?err, "connection refused");
            return Err(ErrorCode::ConnectionRefused);
        }
        Err(..) => return Err(ErrorCode::ConnectionTimeout),
    };
    let stream = if use_tls {
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
        let stream = connector.connect(domain, stream).await.map_err(|e| {
            tracing::warn!("tls protocol error: {e:?}");
            ErrorCode::TlsProtocolError
        })?;
        stream.boxed()
    } else {
        stream.boxed()
    };
    let (mut sender, conn) = tokio::time::timeout(
        connect_timeout,
        // TODO: we should plumb the builder through the http context, and use it here
        hyper::client::conn::http1::Builder::new().handshake(crate::io::TokioIo::new(stream)),
    )
    .await
    .map_err(|_| ErrorCode::ConnectionTimeout)?
    .map_err(ErrorCode::from_hyper_request_error)?;

    // at this point, the request contains the scheme and the authority, but
    // the http packet should only include those if addressing a proxy, so
    // remove them here, since SendRequest::send_request does not do it for us
    *req.uri_mut() = http::Uri::builder()
        .path_and_query(
            req.uri()
                .path_and_query()
                .map(|p| p.as_str())
                .unwrap_or("/"),
        )
        .build()
        .expect("comes from valid request");

    let send = async move {
        use core::task::Context;

        /// Wrapper around [hyper::body::Incoming] used to
        /// account for request option timeout configuration
        struct IncomingResponseBody {
            incoming: hyper::body::Incoming,
            timeout: tokio::time::Interval,
        }
        impl http_body::Body for IncomingResponseBody {
            type Data = <hyper::body::Incoming as http_body::Body>::Data;
            type Error = ErrorCode;

            fn poll_frame(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
                match Pin::new(&mut self.as_mut().incoming).poll_frame(cx) {
                    Poll::Ready(None) => Poll::Ready(None),
                    Poll::Ready(Some(Err(err))) => {
                        Poll::Ready(Some(Err(ErrorCode::from_hyper_response_error(err))))
                    }
                    Poll::Ready(Some(Ok(frame))) => {
                        self.timeout.reset();
                        Poll::Ready(Some(Ok(frame)))
                    }
                    Poll::Pending => {
                        ready!(self.timeout.poll_tick(cx));
                        Poll::Ready(Some(Err(ErrorCode::ConnectionReadTimeout)))
                    }
                }
            }
            fn is_end_stream(&self) -> bool {
                self.incoming.is_end_stream()
            }
            fn size_hint(&self) -> http_body::SizeHint {
                self.incoming.size_hint()
            }
        }

        let res = tokio::time::timeout(first_byte_timeout, sender.send_request(req))
            .await
            .map_err(|_| ErrorCode::ConnectionReadTimeout)?
            .map_err(ErrorCode::from_hyper_request_error)?;
        let mut timeout = tokio::time::interval(between_bytes_timeout);
        timeout.reset();
        Ok(res.map(|incoming| IncomingResponseBody { incoming, timeout }))
    };
    let mut send = pin!(send);
    let mut conn = Some(conn);
    // Wait for response while driving connection I/O
    let res = poll_fn(|cx| match send.as_mut().poll(cx) {
        Poll::Ready(Ok(res)) => Poll::Ready(Ok(res)),
        Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
        Poll::Pending => {
            // Response is not ready, poll `hyper` connection to drive I/O if it has not completed yet
            let Some(fut) = conn.as_mut() else {
                // `hyper` connection already completed
                return Poll::Pending;
            };
            let res = ready!(Pin::new(fut).poll(cx));
            // `hyper` connection completed, record that to prevent repeated poll
            conn = None;
            match res {
                // `hyper` connection has successfully completed, optimistically poll for response
                Ok(()) => send.as_mut().poll(cx),
                // `hyper` connection has failed, return the error
                Err(err) => Poll::Ready(Err(ErrorCode::from_hyper_request_error(err))),
            }
        }
    })
    .await?;
    Ok((res, async move {
        let Some(conn) = conn.take() else {
            // `hyper` connection has already completed
            return Ok(());
        };
        conn.await.map_err(ErrorCode::from_hyper_response_error)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p3::DefaultWasiHttpCtx;
    use core::future::Future;
    use core::pin::pin;
    use core::str::FromStr;
    use core::task::{Context, Poll, Waker};
    use http_body_util::{BodyExt, Empty, Full};
    use wasmtime::Result;
    use wasmtime::{Engine, Store};
    use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

    struct TestCtx {
        table: ResourceTable,
        wasi: WasiCtx,
        http: DefaultWasiHttpCtx,
    }

    impl TestCtx {
        fn new() -> Self {
            Self {
                table: ResourceTable::default(),
                wasi: WasiCtxBuilder::new().build(),
                http: DefaultWasiHttpCtx,
            }
        }
    }

    impl WasiView for TestCtx {
        fn ctx(&mut self) -> WasiCtxView<'_> {
            WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.table,
            }
        }
    }

    impl WasiHttpView for TestCtx {
        fn http(&mut self) -> WasiHttpCtxView<'_> {
            WasiHttpCtxView {
                ctx: &mut self.http,
                table: &mut self.table,
            }
        }
    }

    #[tokio::test]
    async fn test_request_into_http_schemes() -> Result<()> {
        let schemes = vec![Some(Scheme::HTTP), Some(Scheme::HTTPS), None];
        let engine = Engine::default();

        for scheme in schemes {
            let (req, fut) = Request::new(
                Method::POST,
                scheme.clone(),
                Some(Authority::from_static("example.com")),
                Some(PathAndQuery::from_static("/path?query=1")),
                HeaderMap::new(),
                None,
                Full::new(Bytes::from_static(b"body"))
                    .map_err(|x| match x {})
                    .boxed_unsync(),
            );
            let mut store = Store::new(&engine, TestCtx::new());
            let (http_req, options) = req.into_http(&mut store, async { Ok(()) }).unwrap();
            assert_eq!(options, None);
            assert_eq!(http_req.method(), Method::POST);
            let expected_scheme = scheme.unwrap_or(Scheme::HTTPS); // default scheme
            assert_eq!(
                http_req.uri(),
                &http::Uri::from_str(&format!(
                    "{}://example.com/path?query=1",
                    expected_scheme.as_str()
                ))
                .unwrap()
            );
            let body_bytes = http_req.into_body().collect().await?;
            assert_eq!(body_bytes.to_bytes(), b"body".as_slice());
            let mut cx = Context::from_waker(Waker::noop());
            let result = pin!(fut).poll(&mut cx);
            assert!(matches!(result, Poll::Ready(Ok(()))));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_request_into_http_uri_error() -> Result<()> {
        let (req, fut) = Request::new(
            Method::GET,
            Some(Scheme::HTTP),
            Some(Authority::from_static("example.com")),
            None, // <-- should fail, must be Some(_) when authority is set
            HeaderMap::new(),
            None,
            Empty::new().map_err(|x| match x {}).boxed_unsync(),
        );
        let mut store = Store::new(&Engine::default(), TestCtx::new());
        let result = req.into_http(&mut store, async {
            Err(ErrorCode::InternalError(Some("uh oh".to_string())))
        });
        assert!(matches!(result, Err(ErrorCode::HttpRequestUriInvalid)));
        let mut cx = Context::from_waker(Waker::noop());
        let result = pin!(fut).poll(&mut cx);
        assert!(matches!(
            result,
            Poll::Ready(Err(ErrorCode::InternalError(Some(_))))
        ));

        Ok(())
    }
}

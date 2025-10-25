use crate::p3::bindings::http::types::ErrorCode;
use crate::p3::body::Body;
use bytes::Bytes;
use core::time::Duration;
use http::uri::{Authority, PathAndQuery, Scheme};
use http::{HeaderMap, Method};
use http_body_util::BodyExt as _;
use http_body_util::combinators::UnsyncBoxBody;
use std::sync::Arc;
use tokio::sync::oneshot;

/// The concrete type behind a `wasi:http/types/request-options` resource.
#[derive(Copy, Clone, Debug, Default)]
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

use crate::{Error, RequestOptions};
use bytes::Bytes;
use core::future::poll_fn;
use core::pin::{Pin, pin};
use core::task::{Context, Poll, ready};
use http::uri::Scheme;
use http::{Request, Response};
use http_body::Body;
use std::time::Duration;
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
pub async fn default_send_request(
    req: Request<impl Body<Data = Bytes, Error = Error> + Send + 'static>,
    options: Option<RequestOptions>,
) -> Result<
    (
        Response<impl Body<Data = Bytes, Error = Error>>,
        impl Future<Output = Result<(), Error>> + Send,
    ),
    Error,
> {
    default_send_request_with_http2(req, options, crate::Http2Config::default()).await
}

fn between_bytes_timeout_or_1ns(t: Duration) -> Duration {
    if t == Duration::ZERO {
        Duration::new(0, 1)
    } else {
        t
    }
}

/// Use an executor to drive HTTP/2 client connections.
#[derive(Clone, Copy, Debug)]
struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: Future<Output = ()> + Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::spawn(fut);
    }
}

/// Like [`default_send_request`], but with explicit HTTP/2 negotiation behavior.
pub async fn default_send_request_with_http2(
    mut req: Request<impl Body<Data = Bytes, Error = Error> + Send + 'static>,
    options: Option<RequestOptions>,
    http2: crate::Http2Config,
) -> Result<
    (
        Response<impl Body<Data = Bytes, Error = Error>>,
        impl Future<Output = Result<(), Error>> + Send,
    ),
    Error,
> {
    let uri = req.uri();
    let authority = uri.authority().ok_or(Error::HttpRequestUriInvalid)?;
    let use_tls = uri.scheme() == Some(&Scheme::HTTPS);
    let authority = if authority.port().is_some() {
        authority.to_string()
    } else {
        let port = if use_tls { 443 } else { 80 };
        format!("{authority}:{port}")
    };

    let connect_timeout = options
        .and_then(|r| r.connect_timeout)
        .unwrap_or(Duration::from_secs(600));
    let first_byte_timeout = options
        .and_then(|r| r.first_byte_timeout)
        .unwrap_or(Duration::from_secs(600));
    let between_bytes_timeout = options
        .and_then(|r| r.between_bytes_timeout)
        .unwrap_or(Duration::from_secs(600));

    // `connect_timeout` covers everything up to a usable connection: the TCP
    // connect, the TLS handshake, and the HTTP handshake.
    let establish = async {
        let stream = TcpStream::connect(&authority)
            .await
            .map_err(Error::Connect)?;
        let (stream, negotiated_h2): (Box<dyn TokioStream>, bool) = if use_tls {
            // derived from https://github.com/rustls/rustls/blob/main/examples/src/bin/simpleclient.rs
            let root_cert_store = rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.into(),
            };
            let mut config = rustls::ClientConfig::builder()
                .with_root_certificates(root_cert_store)
                .with_no_client_auth();
            if http2.alpn {
                config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
            }
            let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
            let domain = tls_server_name(&authority)?;
            let stream = connector
                .connect(domain, stream)
                .await
                .map_err(Error::Tls)?;
            let negotiated_h2 = stream.get_ref().1.alpn_protocol() == Some(b"h2");
            (stream.boxed(), negotiated_h2)
        } else {
            (stream.boxed(), false)
        };
        let use_http2 = negotiated_h2 || (!use_tls && http2.prior_knowledge);
        Ok::<_, Error>((stream, use_http2))
    };
    let (stream, use_http2) = tokio::time::timeout(connect_timeout, establish)
        .await
        .map_err(|_| Error::ConnectionTimeout)??;

    if use_http2 {
        return send_http2(
            req,
            stream,
            connect_timeout,
            first_byte_timeout,
            between_bytes_timeout,
        )
        .await;
    }

    let (mut sender, conn) = tokio::time::timeout(
        connect_timeout,
        hyper::client::conn::http1::Builder::new().handshake(crate::io::TokioIo::new(stream)),
    )
    .await
    .map_err(|_| Error::ConnectionTimeout)??;

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

    let (response, conn): (
        Pin<Box<dyn Future<Output = Result<Response<hyper::body::Incoming>, hyper::Error>> + Send>>,
        Pin<Box<dyn Future<Output = Result<(), hyper::Error>> + Send>>,
    ) = (
        Box::pin(async move { sender.send_request(req).await }),
        Box::pin(async move { conn.await }),
    );

    let send = async move {
        let res = tokio::time::timeout(first_byte_timeout, response)
            .await
            .map_err(|_| Error::ConnectionReadTimeout)?
            .map_err(Error::from)?;
        // Tokio panics on an `interval` with a zero duration, so clamp it to a
        // 1ns timeout which is sort of effectively zero anyway.
        let between_bytes_timeout = between_bytes_timeout_or_1ns(between_bytes_timeout);
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
                Err(err) => Poll::Ready(Err(Error::from(err))),
            }
        }
    })
    .await?;
    let io = Box::pin(async move {
        let Some(conn) = conn.take() else {
            // `hyper` connection has already completed
            return Ok(());
        };
        if let Err(err) = conn.await {
            if err.is_timeout() {
                return Err(Error::HttpResponseTimeout);
            }
            return Err(err.into());
        }
        Ok(())
    }) as Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>;
    Ok((res, io))
}

/// Wrapper around [hyper::body::Incoming] used to account for request option
/// timeout configuration.
struct IncomingResponseBody {
    incoming: hyper::body::Incoming,
    timeout: tokio::time::Interval,
}

impl http_body::Body for IncomingResponseBody {
    type Data = <hyper::body::Incoming as http_body::Body>::Data;
    type Error = Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        match Pin::new(&mut self.as_mut().incoming).poll_frame(cx) {
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Err(err))) => {
                let err = if err.is_timeout() {
                    Error::HttpResponseTimeout
                } else {
                    Error::from(err)
                };
                Poll::Ready(Some(Err(err)))
            }
            Poll::Ready(Some(Ok(frame))) => {
                self.timeout.reset();
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Pending => {
                ready!(self.timeout.poll_tick(cx));
                Poll::Ready(Some(Err(Error::ConnectionReadTimeout)))
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

/// Sends `req` over an HTTP/2 connection and keeps driving it until the caller
/// drops the returned future.
#[cfg_attr(not(feature = "default-send-request"), allow(dead_code))]
async fn send_http2(
    req: Request<impl Body<Data = Bytes, Error = Error> + Send + 'static>,
    stream: Box<dyn TokioStream>,
    connect_timeout: Duration,
    first_byte_timeout: Duration,
    between_bytes_timeout: Duration,
) -> Result<
    (
        Response<IncomingResponseBody>,
        Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>,
    ),
    Error,
> {
    let req = req.map(|body| http_body_util::BodyExt::boxed_unsync(body));
    let (mut sender, conn) = tokio::time::timeout(
        connect_timeout,
        hyper::client::conn::http2::Builder::new(TokioExecutor)
            .handshake(crate::io::TokioIo::new(stream)),
    )
    .await
    .map_err(|_| Error::ConnectionTimeout)??;
    let mut conn = Box::pin(conn);
    let res = tokio::time::timeout(first_byte_timeout, async {
        use futures::FutureExt as _;
        use futures::future::{Either, select};
        match select(sender.send_request(req).boxed(), &mut conn).await {
            Either::Left((res, _)) => res.map_err(Error::from),
            Either::Right((done, _)) => {
                done.map_err(Error::from)?;
                Err(Error::ConnectionReadTimeout)
            }
        }
    })
    .await
    .map_err(|_| Error::ConnectionReadTimeout)??;
    let mut timeout = tokio::time::interval(between_bytes_timeout_or_1ns(between_bytes_timeout));
    timeout.reset();
    let io = Box::pin(async move {
        let _ = conn.await;
        Ok(())
    }) as Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>;
    Ok((
        res.map(|incoming| IncomingResponseBody { incoming, timeout }),
        io,
    ))
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
pub(crate) fn tls_server_name(
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

#[cfg(test)]
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

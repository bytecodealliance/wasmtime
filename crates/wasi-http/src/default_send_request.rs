use crate::{Error, RequestOptions};
use bytes::Bytes;
use core::future::poll_fn;
use core::pin::{Pin, pin};
use core::task::{Poll, ready};
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
    mut req: Request<impl Body<Data = Bytes, Error = Error> + Send + 'static>,
    options: Option<RequestOptions>,
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

    let stream = match tokio::time::timeout(connect_timeout, TcpStream::connect(&authority)).await {
        Ok(stream) => stream.map_err(Error::Connect)?,
        Err(..) => return Err(Error::ConnectionTimeout),
    };
    let stream = if use_tls {
        // derived from https://github.com/rustls/rustls/blob/main/examples/src/bin/simpleclient.rs
        let root_cert_store = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.into(),
        };
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
        let domain = tls_server_name(&authority)?;
        let stream = connector
            .connect(domain, stream)
            .await
            .map_err(Error::Tls)?;
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

        let res = tokio::time::timeout(first_byte_timeout, sender.send_request(req))
            .await
            .map_err(|_| Error::ConnectionReadTimeout)?
            .map_err(Error::from)?;
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
    Ok((res, async move {
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
    }))
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

use crate::bindings::http::{
    outgoing_handler,
    types::{FutureIncomingResponse, OutgoingRequest, RequestOptions, Scheme},
};
use crate::types::{self, HostFutureIncomingResponse, IncomingResponseInternal, TableHttpExt};
use crate::WasiHttpView;
use anyhow::Context;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Method;
use hyper_util::rt::TokioIo;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use wasmtime_wasi::preview2;

impl<T: WasiHttpView> outgoing_handler::Host for T {
    fn handle(
        &mut self,
        request_id: OutgoingRequest,
        options: Option<RequestOptions>,
    ) -> wasmtime::Result<Result<FutureIncomingResponse, outgoing_handler::Error>> {
        let connect_timeout = Duration::from_millis(
            options
                .and_then(|opts| opts.connect_timeout_ms)
                .unwrap_or(600 * 1000) as u64,
        );

        let first_byte_timeout = Duration::from_millis(
            options
                .and_then(|opts| opts.first_byte_timeout_ms)
                .unwrap_or(600 * 1000) as u64,
        );

        let between_bytes_timeout = Duration::from_millis(
            options
                .and_then(|opts| opts.between_bytes_timeout_ms)
                .unwrap_or(600 * 1000) as u64,
        );

        let req = types::OutgoingRequestLens::from(request_id).delete(self.table())?;

        let method = match req.method {
            crate::bindings::http::types::Method::Get => Method::GET,
            crate::bindings::http::types::Method::Head => Method::HEAD,
            crate::bindings::http::types::Method::Post => Method::POST,
            crate::bindings::http::types::Method::Put => Method::PUT,
            crate::bindings::http::types::Method::Delete => Method::DELETE,
            crate::bindings::http::types::Method::Connect => Method::CONNECT,
            crate::bindings::http::types::Method::Options => Method::OPTIONS,
            crate::bindings::http::types::Method::Trace => Method::TRACE,
            crate::bindings::http::types::Method::Patch => Method::PATCH,
            crate::bindings::http::types::Method::Other(method) => {
                return Ok(Err(outgoing_handler::Error::InvalidUrl(format!(
                    "unknown method {method}"
                ))));
            }
        };

        let (use_tls, scheme, port) = match req.scheme.unwrap_or(Scheme::Https) {
            Scheme::Http => (false, "http://", 80),
            Scheme::Https => (true, "https://", 443),
            Scheme::Other(scheme) => {
                return Ok(Err(outgoing_handler::Error::InvalidUrl(format!(
                    "unsupported scheme {scheme}"
                ))))
            }
        };

        let authority = if req.authority.find(':').is_some() {
            req.authority.clone()
        } else {
            format!("{}:{port}", req.authority)
        };

        let mut builder = hyper::Request::builder()
            .method(method)
            .uri(format!("{scheme}{authority}{}", req.path_with_query))
            .header(hyper::header::HOST, &authority);

        for (k, v) in req.headers.iter() {
            builder = builder.header(k, v);
        }

        let body = req.body.unwrap_or_else(|| {
            Empty::<Bytes>::new()
                .map_err(|_| anyhow::anyhow!("empty error"))
                .boxed()
        });

        let request = builder.body(body).map_err(http_protocol_error)?;

        let handle = preview2::spawn(async move {
            let tcp_stream = TcpStream::connect(authority.clone())
                .await
                .map_err(invalid_url)?;

            let (mut sender, worker) = if use_tls {
                #[cfg(any(target_arch = "riscv64", target_arch = "s390x"))]
                {
                    anyhow::bail!(crate::bindings::http::types::Error::UnexpectedError(
                        "unsupported architecture for SSL".to_string(),
                    ));
                }

                #[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
                {
                    use tokio_rustls::rustls::OwnedTrustAnchor;

                    // derived from https://github.com/tokio-rs/tls/blob/master/tokio-rustls/examples/client/src/main.rs
                    let mut root_cert_store = rustls::RootCertStore::empty();
                    root_cert_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(
                        |ta| {
                            OwnedTrustAnchor::from_subject_spki_name_constraints(
                                ta.subject,
                                ta.spki,
                                ta.name_constraints,
                            )
                        },
                    ));
                    let config = rustls::ClientConfig::builder()
                        .with_safe_defaults()
                        .with_root_certificates(root_cert_store)
                        .with_no_client_auth();
                    let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
                    let mut parts = authority.split(":");
                    let host = parts.next().unwrap_or(&authority);
                    let domain = rustls::ServerName::try_from(host)?;
                    let stream = connector.connect(domain, tcp_stream).await.map_err(|e| {
                        crate::bindings::http::types::Error::ProtocolError(e.to_string())
                    })?;

                    let (sender, conn) = timeout(
                        connect_timeout,
                        hyper::client::conn::http1::handshake(TokioIo::new(stream)),
                    )
                    .await
                    .map_err(|_| timeout_error("connection"))??;

                    let worker = preview2::spawn(async move {
                        conn.await.context("hyper connection failed")?;
                        Ok::<_, anyhow::Error>(())
                    });

                    (sender, worker)
                }
            } else {
                let (sender, conn) = timeout(
                    connect_timeout,
                    // TODO: we should plumb the builder through the http context, and use it here
                    hyper::client::conn::http1::handshake(TokioIo::new(tcp_stream)),
                )
                .await
                .map_err(|_| timeout_error("connection"))??;

                let worker = preview2::spawn(async move {
                    conn.await.context("hyper connection failed")?;
                    Ok::<_, anyhow::Error>(())
                });

                (sender, worker)
            };

            let resp = timeout(first_byte_timeout, sender.send_request(request))
                .await
                .map_err(|_| timeout_error("first byte"))?
                .map_err(hyper_protocol_error)?
                .map(|body| body.map_err(|e| anyhow::anyhow!(e)).boxed());

            Ok(IncomingResponseInternal {
                resp,
                worker,
                between_bytes_timeout,
            })
        });

        let fut = self
            .table()
            .push_future_incoming_response(HostFutureIncomingResponse::new(handle))?;

        Ok(Ok(fut))
    }
}

fn timeout_error(kind: &str) -> anyhow::Error {
    anyhow::anyhow!(crate::bindings::http::types::Error::TimeoutError(format!(
        "{kind} timed out"
    )))
}

fn http_protocol_error(e: http::Error) -> anyhow::Error {
    anyhow::anyhow!(crate::bindings::http::types::Error::ProtocolError(
        e.to_string()
    ))
}

fn hyper_protocol_error(e: hyper::Error) -> anyhow::Error {
    anyhow::anyhow!(crate::bindings::http::types::Error::ProtocolError(
        e.to_string()
    ))
}

fn invalid_url(e: std::io::Error) -> anyhow::Error {
    // TODO: DNS errors show up as a Custom io error, what subset of errors should we consider for
    // InvalidUrl here?
    anyhow::anyhow!(crate::bindings::http::types::Error::InvalidUrl(
        e.to_string()
    ))
}

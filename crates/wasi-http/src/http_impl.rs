use crate::r#struct::{ActiveFields, ActiveFuture, ActiveResponse, HttpResponse, TableHttpExt};
use crate::wasi::http::types::{FutureIncomingResponse, OutgoingRequest, RequestOptions, Scheme};
pub use crate::{WasiHttpCtx, WasiHttpView};
#[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
use anyhow::Context;
use bytes::{Bytes, BytesMut};
use http_body_util::{BodyExt, Empty, Full};
use hyper::{Method, Request};
#[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
#[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
use tokio_rustls::rustls::{self, OwnedTrustAnchor};
use wasmtime_wasi::preview2::{StreamState, TableStreamExt};

#[async_trait::async_trait]
impl<T: WasiHttpView> crate::wasi::http::outgoing_handler::Host for T {
    async fn handle(
        &mut self,
        request_id: OutgoingRequest,
        options: Option<RequestOptions>,
    ) -> wasmtime::Result<FutureIncomingResponse> {
        let future = ActiveFuture::new(request_id, options);
        let future_id = self
            .table_mut()
            .push_future(Box::new(future))
            .context("[handle] pushing future")?;
        Ok(future_id)
    }
}

fn port_for_scheme(scheme: &Option<Scheme>) -> &str {
    match scheme {
        Some(s) => match s {
            Scheme::Http => ":80",
            Scheme::Https => ":443",
            // This should never happen.
            _ => panic!("unsupported scheme!"),
        },
        None => ":443",
    }
}

#[async_trait::async_trait]
pub trait WasiHttpViewExt {
    async fn handle_async(
        &mut self,
        request_id: OutgoingRequest,
        options: Option<RequestOptions>,
    ) -> wasmtime::Result<FutureIncomingResponse, crate::wasi::http::types::Error>;
}

#[async_trait::async_trait]
impl<T: WasiHttpView> WasiHttpViewExt for T {
    async fn handle_async(
        &mut self,
        request_id: OutgoingRequest,
        options: Option<RequestOptions>,
    ) -> wasmtime::Result<FutureIncomingResponse, crate::wasi::http::types::Error> {
        let opts = options.unwrap_or(
            // TODO: Configurable defaults here?
            RequestOptions {
                connect_timeout_ms: Some(600 * 1000),
                first_byte_timeout_ms: Some(600 * 1000),
                between_bytes_timeout_ms: Some(600 * 1000),
            },
        );
        let connect_timeout =
            Duration::from_millis(opts.connect_timeout_ms.unwrap_or(600 * 1000).into());
        let first_bytes_timeout =
            Duration::from_millis(opts.first_byte_timeout_ms.unwrap_or(600 * 1000).into());
        let between_bytes_timeout =
            Duration::from_millis(opts.between_bytes_timeout_ms.unwrap_or(600 * 1000).into());

        let request = self
            .table()
            .get_request(request_id)
            .context("[handle_async] getting request")?
            .clone();

        let method = match request.method() {
            crate::wasi::http::types::Method::Get => Method::GET,
            crate::wasi::http::types::Method::Head => Method::HEAD,
            crate::wasi::http::types::Method::Post => Method::POST,
            crate::wasi::http::types::Method::Put => Method::PUT,
            crate::wasi::http::types::Method::Delete => Method::DELETE,
            crate::wasi::http::types::Method::Connect => Method::CONNECT,
            crate::wasi::http::types::Method::Options => Method::OPTIONS,
            crate::wasi::http::types::Method::Trace => Method::TRACE,
            crate::wasi::http::types::Method::Patch => Method::PATCH,
            crate::wasi::http::types::Method::Other(s) => {
                return Err(crate::wasi::http::types::Error::InvalidUrl(format!(
                    "unknown method {}",
                    s
                ))
                .into());
            }
        };

        let scheme = match request.scheme().as_ref().unwrap_or(&Scheme::Https) {
            Scheme::Http => "http://",
            Scheme::Https => "https://",
            Scheme::Other(s) => {
                return Err(crate::wasi::http::types::Error::InvalidUrl(format!(
                    "unsupported scheme {}",
                    s
                ))
                .into());
            }
        };

        // Largely adapted from https://hyper.rs/guides/1/client/basic/
        let authority = match request.authority().find(":") {
            Some(_) => request.authority().to_owned(),
            None => request.authority().to_owned() + port_for_scheme(request.scheme()),
        };
        let tcp_stream = TcpStream::connect(authority.clone()).await?;
        let mut sender = if scheme == "https://" {
            #[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
            {
                //TODO: uncomment this code and make the tls implementation a feature decision.
                //let connector = tokio_native_tls::native_tls::TlsConnector::builder().build()?;
                //let connector = tokio_native_tls::TlsConnector::from(connector);
                //let host = authority.split(":").next().unwrap_or(&authority);
                //let stream = connector.connect(&host, stream).await?;

                // derived from https://github.com/tokio-rs/tls/blob/master/tokio-rustls/examples/client/src/main.rs
                let mut root_cert_store = rustls::RootCertStore::empty();
                root_cert_store.add_server_trust_anchors(
                    webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
                        OwnedTrustAnchor::from_subject_spki_name_constraints(
                            ta.subject,
                            ta.spki,
                            ta.name_constraints,
                        )
                    }),
                );
                let config = rustls::ClientConfig::builder()
                    .with_safe_defaults()
                    .with_root_certificates(root_cert_store)
                    .with_no_client_auth();
                let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
                let mut parts = authority.split(":");
                let host = parts.next().unwrap_or(&authority);
                let domain = rustls::ServerName::try_from(host)?;
                let stream = connector
                    .connect(domain, tcp_stream)
                    .await
                    .map_err(|e| crate::wasi::http::types::Error::ProtocolError(e.to_string()))?;

                let t = timeout(
                    connect_timeout,
                    hyper::client::conn::http1::handshake(stream),
                )
                .await?;
                let (s, conn) = t?;
                tokio::task::spawn(async move {
                    if let Err(err) = conn.await {
                        println!("Connection failed: {:?}", err);
                    }
                });
                s
            }
            #[cfg(any(target_arch = "riscv64", target_arch = "s390x"))]
            return Err(crate::wasi::http::types::Error::UnexpectedError(
                "unsupported architecture for SSL",
            ));
        } else {
            let t = timeout(
                connect_timeout,
                hyper::client::conn::http1::handshake(tcp_stream),
            )
            .await?;
            let (s, conn) = t?;
            tokio::task::spawn(async move {
                if let Err(err) = conn.await {
                    println!("Connection failed: {:?}", err);
                }
            });
            s
        };

        let url = scheme.to_owned() + &request.authority() + &request.path_with_query();

        let mut call = Request::builder()
            .method(method)
            .uri(url)
            .header(hyper::header::HOST, request.authority());

        if let Some(headers) = request.headers() {
            for (key, val) in self
                .table()
                .get_fields(headers)
                .context("[handle_async] getting request headers")?
                .iter()
            {
                for item in val {
                    call = call.header(key, item.clone());
                }
            }
        }

        let mut response = ActiveResponse::new();
        let body = match request.body() {
            Some(id) => {
                let table = self.table_mut();
                let stream = table
                    .get_stream(id)
                    .context("[handle_async] getting stream")?;
                let input_stream = table
                    .get_input_stream_mut(stream.incoming())
                    .context("[handle_async] getting mutable input stream")?;
                let mut bytes = BytesMut::new();
                let mut eof = StreamState::Open;
                while eof != StreamState::Closed {
                    let (chunk, state) = input_stream.read(4096)?;
                    eof = if chunk.is_empty() {
                        StreamState::Closed
                    } else {
                        state
                    };
                    bytes.extend_from_slice(&chunk[..]);
                }
                Full::<Bytes>::new(bytes.freeze()).boxed()
            }
            None => Empty::<Bytes>::new().boxed(),
        };
        let t = timeout(first_bytes_timeout, sender.send_request(call.body(body)?)).await?;
        let mut res = t?;
        response.status = res.status().as_u16();

        let mut map = ActiveFields::new();
        for (key, value) in res.headers().iter() {
            let mut vec = Vec::new();
            vec.push(value.as_bytes().to_vec());
            map.insert(key.as_str().to_string(), vec);
        }
        let headers = self
            .table_mut()
            .push_fields(Box::new(map))
            .context("[handle_async] pushing response headers")?;
        response.set_headers(headers);

        let mut buf: Vec<u8> = Vec::new();
        while let Some(next) = timeout(between_bytes_timeout, res.frame()).await? {
            let frame = next?;
            if let Some(chunk) = frame.data_ref() {
                buf.extend_from_slice(chunk);
            }
            if let Some(trailers) = frame.trailers_ref() {
                let mut map = ActiveFields::new();
                for (name, value) in trailers.iter() {
                    let key = name.to_string();
                    match map.get_mut(&key) {
                        Some(vec) => vec.push(value.as_bytes().to_vec()),
                        None => {
                            let mut vec = Vec::new();
                            vec.push(value.as_bytes().to_vec());
                            map.insert(key, vec);
                        }
                    };
                }
                let trailers = self
                    .table_mut()
                    .push_fields(Box::new(map))
                    .context("[handle_async] pushing response trailers")?;
                response.set_trailers(trailers);
            }
        }

        let response_id = self
            .table_mut()
            .push_response(Box::new(response))
            .context("[handle_async] pushing response")?;
        let (stream_id, stream) = self
            .table_mut()
            .push_stream(Bytes::from(buf), response_id)
            .context("[handle_async] pushing stream")?;
        let response = self
            .table_mut()
            .get_response_mut(response_id)
            .context("[handle_async] getting mutable response")?;
        response.set_body(stream_id);

        self.http_ctx_mut().streams.insert(stream_id, stream);

        Ok(response_id)
    }
}

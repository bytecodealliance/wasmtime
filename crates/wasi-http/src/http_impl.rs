use crate::r#struct::{ActiveFuture, ActiveResponse};
use crate::r#struct::{Stream, WasiHttp};
use crate::wasi::http::types::{FutureIncomingResponse, OutgoingRequest, RequestOptions, Scheme};
#[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
use anyhow::anyhow;
use anyhow::bail;
use bytes::{BufMut, Bytes, BytesMut};
use http_body_util::{BodyExt, Full};
use hyper::Method;
use hyper::Request;
use std::collections::HashMap;
use std::net::SocketAddr;
#[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
#[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
use tokio_rustls::rustls::{self, OwnedTrustAnchor};

impl crate::wasi::http::outgoing_handler::Host for WasiHttp {
    fn handle(
        &mut self,
        request_id: OutgoingRequest,
        options: Option<RequestOptions>,
    ) -> wasmtime::Result<FutureIncomingResponse> {
        let future_id = self.future_id_base;
        self.future_id_base = self.future_id_base + 1;
        let future = ActiveFuture::new(future_id, request_id, options);
        self.futures.insert(future_id, future);
        Ok(future_id)
    }
}

fn port_for_scheme(scheme: &Option<Scheme>) -> u16 {
    match scheme {
        Some(s) => match s {
            Scheme::Http => 80,
            Scheme::Https => 443,
            // This should never happen.
            _ => panic!("unsupported scheme!"),
        },
        None => 443,
    }
}

impl WasiHttp {
    pub(crate) async fn handle_async(
        &mut self,
        request_id: OutgoingRequest,
        options: Option<RequestOptions>,
    ) -> wasmtime::Result<FutureIncomingResponse> {
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

        let request = match self.requests.get(&request_id) {
            Some(r) => r,
            None => bail!("not found!"),
        };

        let method = match &request.method {
            crate::wasi::http::types::Method::Get => Method::GET,
            crate::wasi::http::types::Method::Head => Method::HEAD,
            crate::wasi::http::types::Method::Post => Method::POST,
            crate::wasi::http::types::Method::Put => Method::PUT,
            crate::wasi::http::types::Method::Delete => Method::DELETE,
            crate::wasi::http::types::Method::Connect => Method::CONNECT,
            crate::wasi::http::types::Method::Options => Method::OPTIONS,
            crate::wasi::http::types::Method::Trace => Method::TRACE,
            crate::wasi::http::types::Method::Patch => Method::PATCH,
            crate::wasi::http::types::Method::Other(s) => bail!("unknown method {}", s),
        };

        let acl_method_match = self.acl.is_method_allowed(method.as_str());
        if acl_method_match.is_denied() {
            bail!(
                "Method {} is not allowed - {}",
                method.as_str(),
                acl_method_match
            );
        }

        let scheme = match request.scheme.as_ref().unwrap_or(&Scheme::Https) {
            Scheme::Http => "http",
            Scheme::Https => "https",
            Scheme::Other(s) => bail!("unsupported scheme {}", s),
        };

        let acl_scheme_match = self.acl.is_scheme_allowed(scheme);
        if acl_scheme_match.is_denied() {
            bail!("Scheme {} is not allowed - {}", scheme, acl_scheme_match);
        }

        let authority = match http_acl::utils::authority::Authority::parse(&request.authority) {
            Ok(a) => a,
            Err(e) => bail!("invalid authority: {}", e),
        };

        let port = if authority.port == 0 {
            port_for_scheme(&request.scheme)
        } else {
            authority.port
        };
        let acl_port_match = self.acl.is_port_allowed(port);
        if acl_port_match.is_denied() {
            bail!("Port {} is not allowed - {}", port, acl_port_match);
        }

        let tcp_addresses = match &authority.host {
            http_acl::utils::authority::Host::Domain(domain) => {
                let acl_host_match = self.acl.is_host_allowed(domain);
                if acl_host_match.is_denied() {
                    bail!("Host {} is not allowed - {}", domain, acl_host_match);
                }

                tokio::net::lookup_host(&(domain.clone() + ":" + &port.to_string()))
                    .await?
                    .collect::<Vec<_>>()
            }
            http_acl::utils::authority::Host::Ip(ip) => {
                let acl_ip_match = self.acl.is_ip_allowed(ip);
                if acl_ip_match.is_denied() {
                    bail!("IP {} is not allowed - {}", ip, acl_ip_match);
                }

                vec![SocketAddr::new(*ip, port)]
            }
        };

        for tcp_address in &tcp_addresses {
            let acl_ip_match = self.acl.is_ip_allowed(&tcp_address.ip());
            if acl_ip_match.is_denied() {
                bail!("IP {} is not allowed - {}", tcp_address.ip(), acl_ip_match);
            }
        }

        // Largely adapted from https://hyper.rs/guides/1/client/basic/
        let mut sender = if scheme == "https" {
            #[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
            {
                let stream = TcpStream::connect(tcp_addresses.as_slice()).await?;
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
                let authority_string = authority.host.to_string();
                let server_name = rustls::ServerName::try_from(authority_string.as_str())
                    .map_err(|_| anyhow!("invalid dnsname"))?;
                let stream = connector.connect(server_name, stream).await?;

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
            bail!("unsupported architecture for SSL")
        } else {
            let tcp = TcpStream::connect(tcp_addresses.as_slice()).await?;
            let t = timeout(connect_timeout, hyper::client::conn::http1::handshake(tcp)).await?;
            let (s, conn) = t?;
            tokio::task::spawn(async move {
                if let Err(err) = conn.await {
                    println!("Connection failed: {:?}", err);
                }
            });
            s
        };

        let url = scheme.to_owned() + "://" + &request.authority + &request.path_with_query;

        let mut call = Request::builder()
            .method(method)
            .uri(url)
            .header(hyper::header::HOST, request.authority.as_str());

        for (key, val) in request.headers.iter() {
            for item in val {
                call = call.header(key, item.clone());
            }
        }

        let response_id = self.response_id_base;
        self.response_id_base = self.response_id_base + 1;
        let mut response = ActiveResponse::new(response_id);
        let body = Full::<Bytes>::new(
            self.streams
                .get(&request.body)
                .unwrap_or(&Stream::default())
                .data
                .clone()
                .freeze(),
        );
        let t = timeout(first_bytes_timeout, sender.send_request(call.body(body)?)).await?;
        let mut res = t?;
        response.status = res.status().try_into()?;
        for (key, value) in res.headers().iter() {
            let mut vec = std::vec::Vec::new();
            vec.push(value.as_bytes().to_vec());
            response
                .response_headers
                .insert(key.as_str().to_string(), vec);
        }
        let mut buf = BytesMut::new();
        while let Some(next) = timeout(between_bytes_timeout, res.frame()).await? {
            let frame = next?;
            if let Some(chunk) = frame.data_ref() {
                buf.put(chunk.clone());
            }
            if let Some(trailers) = frame.trailers_ref() {
                response.trailers = self.fields_id_base;
                self.fields_id_base += 1;
                let mut map: HashMap<String, Vec<Vec<u8>>> = HashMap::new();
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
                self.fields.insert(response.trailers, map);
            }
        }
        response.body = self.streams_id_base;
        self.streams_id_base = self.streams_id_base + 1;
        self.streams.insert(response.body, buf.freeze().into());
        self.responses.insert(response_id, response);
        Ok(response_id)
    }
}

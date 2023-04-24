use crate::types::{Method, Scheme};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context};
use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use http_body_util::Full;
use tokio::net::TcpStream;
use tokio::time;

#[derive(Clone, Default)]
pub struct Stream {
    pub closed: bool,
    pub data: BytesMut,
}

impl From<Stream> for Bytes {
    fn from(Stream { data, .. }: Stream) -> Self {
        data.freeze()
    }
}

#[derive(Clone)]
pub struct WasiHttp<Response = hyper::body::Incoming> {
    pub outgoing_handler: Arc<Box<dyn OutgoingHandler<Body = Response>>>,
    pub request_id_base: u32,
    pub response_id_base: u32,
    pub fields_id_base: u32,
    pub streams_id_base: u32,
    pub requests: HashMap<u32, ActiveRequest>,
    pub responses: HashMap<u32, ActiveResponse>,
    pub fields: HashMap<u32, HashMap<String, Vec<String>>>,
    pub streams: HashMap<u32, Stream>,
}

#[derive(Clone)]
pub struct ActiveRequest {
    pub id: u32,
    pub active_request: bool,
    pub method: Method,
    pub scheme: Option<Scheme>,
    pub path: String,
    pub query: String,
    pub authority: String,
    pub headers: HashMap<String, Vec<String>>,
    pub body: u32,
}

#[derive(Clone)]
pub struct ActiveResponse {
    pub id: u32,
    pub active_response: bool,
    pub status: u16,
    pub body: u32,
    pub response_headers: HashMap<String, Vec<String>>,
    pub trailers: u32,
}

impl ActiveRequest {
    pub fn new(id: u32) -> Self {
        Self {
            id: id,
            active_request: false,
            method: Method::Get,
            scheme: Some(Scheme::Http),
            path: "".to_string(),
            query: "".to_string(),
            authority: "".to_string(),
            headers: HashMap::new(),
            body: 0,
        }
    }
}

impl ActiveResponse {
    pub fn new(id: u32) -> Self {
        Self {
            id: id,
            active_response: false,
            status: 0,
            body: 0,
            response_headers: HashMap::new(),
            trailers: 0,
        }
    }
}

impl Stream {
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<Bytes> for Stream {
    fn from(bytes: Bytes) -> Self {
        let mut buf = BytesMut::with_capacity(bytes.len());
        buf.put(bytes);
        Self {
            closed: false,
            data: buf,
        }
    }
}

#[async_trait]
pub trait OutgoingHandler {
    type Body;

    async fn handle(
        &self,
        request: http::Request<Full<Bytes>>,
        connect_timeout: Duration,
        first_byte_timeout: Duration,
    ) -> anyhow::Result<http::Response<Self::Body>>;
}

/// Default [OutgoingHandler], which relies on Tokio and Hyper to handle both HTTP and HTTPS
/// requests.
pub struct DefaultOutgoingHandler;

#[async_trait]
impl OutgoingHandler for DefaultOutgoingHandler {
    type Body = hyper::body::Incoming;

    async fn handle(
        &self,
        request: http::Request<Full<Bytes>>,
        connect_timeout: Duration,
        first_byte_timeout: Duration,
    ) -> anyhow::Result<http::Response<Self::Body>> {
        let uri = request.uri();
        let authority = uri.authority().context("unknown authority")?;
        let stream = TcpStream::connect(authority.as_str())
            .await
            .with_context(|| format!("failed to connect to `{authority}`"))?;
        let mut sender = match uri.scheme_str() {
            Some("http") => {
                let (sender, conn) = time::timeout(
                    connect_timeout,
                    hyper::client::conn::http1::handshake(stream),
                )
                .await
                .context("connection timed out")?
                .context("handshake failed")?;
                tokio::task::spawn(async move {
                    if let Err(err) = conn.await {
                        println!("Connection failed: {err:?}");
                    }
                });
                sender
            }
            #[cfg(any(target_arch = "riscv64", target_arch = "s390x"))]
            None | Some("https") => bail!("unsupported architecture for SSL"),
            #[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
            None | Some("https") => {
                use tokio_rustls::rustls::{self, OwnedTrustAnchor};

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
                let domain =
                    rustls::ServerName::try_from(authority.host()).context("invalid dnsname")?;
                let stream = connector.connect(domain, stream).await?;
                let (sender, conn) = time::timeout(
                    connect_timeout,
                    hyper::client::conn::http1::handshake(stream),
                )
                .await
                .context("connection timed out")?
                .context("handshake failed")?;
                tokio::task::spawn(async move {
                    if let Err(err) = conn.await {
                        println!("Connection failed: {err:?}");
                    }
                });
                sender
            }
            Some(scheme) => bail!("unsupported scheme `{scheme}`"),
        };
        time::timeout(first_byte_timeout, sender.send_request(request))
            .await
            .context("request timed out")?
            .context("failed to send request")
    }
}

impl Default for WasiHttp {
    fn default() -> Self {
        Self {
            outgoing_handler: Arc::new(Box::new(DefaultOutgoingHandler)),
            request_id_base: 1,
            response_id_base: 1,
            fields_id_base: 1,
            streams_id_base: 1,
            requests: HashMap::default(),
            responses: HashMap::default(),
            fields: HashMap::default(),
            streams: HashMap::default(),
        }
    }
}

impl WasiHttp {
    pub fn new() -> Self {
        Self::default()
    }
}

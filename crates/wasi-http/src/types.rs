//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::{
    bindings::http::types::{self, Method, Scheme},
    body::{HostIncomingBodyBuilder, HyperIncomingBody, HyperOutgoingBody},
};
use anyhow::Context;
use http_body_util::BodyExt;
use std::any::Any;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use wasmtime::component::Resource;
use wasmtime_wasi::preview2::{self, AbortOnDropJoinHandle, Subscribe, Table};

/// Capture the state necessary for use in the wasi-http API implementation.
pub struct WasiHttpCtx;

pub struct OutgoingRequest {
    pub use_tls: bool,
    pub authority: String,
    pub request: hyper::Request<HyperOutgoingBody>,
    pub connect_timeout: Duration,
    pub first_byte_timeout: Duration,
    pub between_bytes_timeout: Duration,
}

pub trait WasiHttpView: Send {
    fn ctx(&mut self) -> &mut WasiHttpCtx;
    fn table(&mut self) -> &mut Table;

    fn new_incoming_request(
        &mut self,
        req: hyper::Request<HyperIncomingBody>,
    ) -> wasmtime::Result<Resource<HostIncomingRequest>> {
        let (parts, body) = req.into_parts();
        let body = HostIncomingBodyBuilder {
            body,
            // TODO: this needs to be plumbed through
            between_bytes_timeout: std::time::Duration::from_millis(600 * 1000),
        };
        Ok(self.table().push(HostIncomingRequest {
            parts,
            body: Some(body),
        })?)
    }

    fn new_response_outparam(
        &mut self,
        result: tokio::sync::oneshot::Sender<
            Result<hyper::Response<HyperOutgoingBody>, types::Error>,
        >,
    ) -> wasmtime::Result<Resource<HostResponseOutparam>> {
        let id = self.table().push(HostResponseOutparam { result })?;
        Ok(id)
    }

    fn send_request(
        &mut self,
        request: OutgoingRequest,
    ) -> wasmtime::Result<Resource<HostFutureIncomingResponse>>
    where
        Self: Sized,
    {
        default_send_request(self, request)
    }
}

pub fn default_send_request(
    view: &mut dyn WasiHttpView,
    OutgoingRequest {
        use_tls,
        authority,
        request,
        connect_timeout,
        first_byte_timeout,
        between_bytes_timeout,
    }: OutgoingRequest,
) -> wasmtime::Result<Resource<HostFutureIncomingResponse>> {
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
                    hyper::client::conn::http1::handshake(stream),
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
                hyper::client::conn::http1::handshake(tcp_stream),
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

    let fut = view.table().push(HostFutureIncomingResponse::new(handle))?;

    Ok(fut)
}

pub fn timeout_error(kind: &str) -> anyhow::Error {
    anyhow::anyhow!(crate::bindings::http::types::Error::TimeoutError(format!(
        "{kind} timed out"
    )))
}

pub fn http_protocol_error(e: http::Error) -> anyhow::Error {
    anyhow::anyhow!(crate::bindings::http::types::Error::ProtocolError(
        e.to_string()
    ))
}

pub fn hyper_protocol_error(e: hyper::Error) -> anyhow::Error {
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

pub struct HostIncomingRequest {
    pub parts: http::request::Parts,
    pub body: Option<HostIncomingBodyBuilder>,
}

pub struct HostResponseOutparam {
    pub result:
        tokio::sync::oneshot::Sender<Result<hyper::Response<HyperOutgoingBody>, types::Error>>,
}

pub struct HostOutgoingRequest {
    pub method: Method,
    pub scheme: Option<Scheme>,
    pub path_with_query: String,
    pub authority: String,
    pub headers: FieldMap,
    pub body: Option<HyperOutgoingBody>,
}

pub struct HostIncomingResponse {
    pub status: u16,
    pub headers: FieldMap,
    pub body: Option<HostIncomingBodyBuilder>,
    pub worker: AbortOnDropJoinHandle<anyhow::Result<()>>,
}

pub struct HostOutgoingResponse {
    pub status: u16,
    pub headers: FieldMap,
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
                    .map_err(|_| anyhow::anyhow!("empty error"))
                    .boxed(),
            ),
        }
    }
}

pub type FieldMap = hyper::HeaderMap;

pub enum HostFields {
    Ref {
        parent: u32,

        // NOTE: there's not failure in the result here because we assume that HostFields will
        // always be registered as a child of the entry with the `parent` id. This ensures that the
        // entry will always exist while this `HostFields::Ref` entry exists in the table, thus we
        // don't need to account for failure when fetching the fields ref from the parent.
        get_fields: for<'a> fn(elem: &'a mut (dyn Any + 'static)) -> &'a mut FieldMap,
    },
    Owned {
        fields: FieldMap,
    },
}

pub struct IncomingResponseInternal {
    pub resp: hyper::Response<HyperIncomingBody>,
    pub worker: AbortOnDropJoinHandle<anyhow::Result<()>>,
    pub between_bytes_timeout: std::time::Duration,
}

type FutureIncomingResponseHandle = AbortOnDropJoinHandle<anyhow::Result<IncomingResponseInternal>>;

pub enum HostFutureIncomingResponse {
    Pending(FutureIncomingResponseHandle),
    Ready(anyhow::Result<IncomingResponseInternal>),
    Consumed,
}

impl HostFutureIncomingResponse {
    pub fn new(handle: FutureIncomingResponseHandle) -> Self {
        Self::Pending(handle)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    pub fn unwrap_ready(self) -> anyhow::Result<IncomingResponseInternal> {
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

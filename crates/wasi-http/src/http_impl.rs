use crate::bindings::http::{
    outgoing_handler,
    types::{FutureIncomingResponse, OutgoingRequest, RequestOptions, Scheme},
};
use crate::types::{HostFutureIncomingResponse, IncomingResponseInternal, TableHttpExt};
use crate::WasiHttpView;
use anyhow::Context;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Method;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use wasmtime_wasi::preview2;

#[async_trait::async_trait]
impl<T: WasiHttpView> outgoing_handler::Host for T {
    async fn handle(
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

        let _between_bytes_timeout = Duration::from_millis(
            options
                .and_then(|opts| opts.between_bytes_timeout_ms)
                .unwrap_or(600 * 1000) as u64,
        );

        let req = self.table().delete_outgoing_request(request_id)?;

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
                return Ok(Err(outgoing_handler::Error::Invalid(format!(
                    "unknown method {method}"
                ))));
            }
        };

        let (use_tls, scheme, port) = match req.scheme.unwrap_or(Scheme::Https) {
            Scheme::Http => (false, "http://", 80),
            Scheme::Https => (true, "https://", 443),
            Scheme::Other(scheme) => {
                return Ok(Err(outgoing_handler::Error::Invalid(format!(
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

        for (k, v) in req.headers.0 {
            for item in v {
                builder = builder.header(&k, item);
            }
        }

        let body = req.body.unwrap_or_else(|| Empty::<Bytes>::new().boxed());

        let request = builder.body(body).map_err(http_protocol_error)?;

        let handle = preview2::spawn(async move {
            let tcp_stream = TcpStream::connect(authority.clone()).await?;
            let (mut sender, conn) = if use_tls {
                if cfg!(any(target_arch = "riscv64", target_arch = "s390x")) {
                    anyhow::bail!(crate::bindings::http::types::Error::UnexpectedError(
                        "unsupported architecture for SSL".to_string(),
                    ));
                }

                todo!("tls")
            } else {
                timeout(
                    connect_timeout,
                    // TODO: we should plumb the builder through the http context, and use it here
                    hyper::client::conn::http1::handshake(tcp_stream),
                )
                .await
                .map_err(|_| timeout_error("connection"))??
            };

            let worker = preview2::spawn(async move {
                conn.await.context("hyper connection failed")?;
                Ok::<_, anyhow::Error>(())
            });

            let resp = timeout(first_byte_timeout, sender.send_request(request))
                .await
                .map_err(|_| timeout_error("first byte"))?
                .map_err(hyper_protocol_error)?;

            Ok(IncomingResponseInternal { resp, worker })
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

use crate::bindings::http::{
    outgoing_handler,
    types::{RequestOptions, Scheme},
};
use crate::types::{self, HostFutureIncomingResponse, OutgoingRequest};
use crate::WasiHttpView;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Method;
use std::time::Duration;
use types::HostOutgoingRequest;
use wasmtime::component::Resource;

impl<T: WasiHttpView> outgoing_handler::Host for T {
    fn handle(
        &mut self,
        request_id: Resource<HostOutgoingRequest>,
        options: Option<RequestOptions>,
    ) -> wasmtime::Result<Result<Resource<HostFutureIncomingResponse>, outgoing_handler::Error>>
    {
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

        let req = self.table().delete(request_id)?;

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

        let request = builder.body(body).map_err(types::http_protocol_error)?;

        Ok(Ok(self.send_request(OutgoingRequest {
            use_tls,
            authority,
            request,
            connect_timeout,
            first_byte_timeout,
            between_bytes_timeout,
        })?))
    }
}

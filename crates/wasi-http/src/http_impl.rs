use crate::bindings::http::types::{
    FutureIncomingResponse, OutgoingRequest, RequestOptions, Scheme,
};
use crate::types::{HostIncomingResponse, TableHttpExt, HostFutureIncomingResponse};
use crate::WasiHttpView;
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
use wasmtime_wasi::preview2::{self, StreamState, TableStreamExt};

#[async_trait::async_trait]
impl<T: WasiHttpView> crate::bindings::http::outgoing_handler::Host for T {
    async fn handle(
        &mut self,
        request_id: OutgoingRequest,
        options: Option<RequestOptions>,
    ) -> wasmtime::Result<FutureIncomingResponse> {
        let mut req = self.table().delete_outgoing_request(request_id)?;

        let handle = preview2::spawn(async move {
            todo!("put the old contents of handle_async in here")
        });

        let fut = self
            .table()
            .push_future_incoming_response(HostFutureIncomingResponse::new(handle))?;

        Ok(fut)
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

//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::{
    bindings::http::types::{self, Method, Scheme},
    body::{HostIncomingBodyBuilder, HyperIncomingBody, HyperOutgoingBody},
};
use std::any::Any;
use wasmtime::component::Resource;
use wasmtime_wasi::preview2::{AbortOnDropJoinHandle, Subscribe, Table};

/// Capture the state necessary for use in the wasi-http API implementation.
pub struct WasiHttpCtx;

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
        use http_body_util::{BodyExt, Empty};

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

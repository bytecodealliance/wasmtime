//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::{
    bindings::http::types::{
        self, FutureTrailers, IncomingBody, IncomingRequest, Method, OutgoingBody,
        OutgoingResponse, ResponseOutparam, Scheme,
    },
    body::{
        HostFutureTrailers, HostIncomingBody, HostIncomingBodyBuilder, HostOutgoingBody, HyperBody,
    },
};
use std::any::Any;
use std::pin::Pin;
use std::task;
use wasmtime_wasi::preview2::{AbortOnDropJoinHandle, Table, TableError};

/// Capture the state necessary for use in the wasi-http API implementation.
pub struct WasiHttpCtx;

pub trait WasiHttpView: Send {
    fn ctx(&mut self) -> &mut WasiHttpCtx;
    fn table(&mut self) -> &mut Table;

    fn new_incoming_request(
        &mut self,
        req: HostIncomingRequest,
    ) -> wasmtime::Result<IncomingRequest> {
        Ok(IncomingRequestLens::push(self.table(), req)?.id)
    }

    fn new_response_outparam(&mut self) -> wasmtime::Result<ResponseOutparam> {
        Ok(ResponseOutparamLens::push(self.table(), HostResponseOutparam { result: None })?.id)
    }

    fn take_response_outparam(
        &mut self,
        outparam: ResponseOutparam,
    ) -> wasmtime::Result<Option<Result<HostOutgoingResponse, types::Error>>> {
        Ok(ResponseOutparamLens::from(outparam)
            .delete(self.table())?
            .result)
    }
}

pub type IncomingRequestLens = TableLens<HostIncomingRequest>;

pub struct HostIncomingRequest {
    pub method: Method,
}

pub type ResponseOutparamLens = TableLens<HostResponseOutparam>;

pub struct HostResponseOutparam {
    pub result: Option<Result<HostOutgoingResponse, types::Error>>,
}

pub type OutgoingRequestLens = TableLens<HostOutgoingRequest>;

pub struct HostOutgoingRequest {
    pub method: Method,
    pub scheme: Option<Scheme>,
    pub path_with_query: String,
    pub authority: String,
    pub headers: FieldMap,
    pub body: Option<HyperBody>,
}

pub struct HostIncomingResponse {
    pub status: u16,
    pub headers: FieldMap,
    pub body: Option<HostIncomingBodyBuilder>,
    pub worker: AbortOnDropJoinHandle<anyhow::Result<()>>,
}

pub type OutgoingResponseLens = TableLens<HostOutgoingResponse>;

pub struct HostOutgoingResponse {}

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
    pub resp: hyper::Response<hyper::body::Incoming>,
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

impl std::future::Future for HostFutureIncomingResponse {
    type Output = anyhow::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let s = self.get_mut();
        match s {
            Self::Pending(ref mut handle) => match Pin::new(handle).poll(cx) {
                task::Poll::Pending => task::Poll::Pending,
                task::Poll::Ready(r) => {
                    *s = Self::Ready(r);
                    task::Poll::Ready(Ok(()))
                }
            },

            Self::Consumed | Self::Ready(_) => task::Poll::Ready(Ok(())),
        }
    }
}

pub struct TableLens<T> {
    id: u32,
    _unused: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static> TableLens<T> {
    pub fn from(id: u32) -> Self {
        Self {
            id,
            _unused: std::marker::PhantomData {},
        }
    }

    pub fn into(self) -> u32 {
        self.id
    }

    #[inline(always)]
    pub fn push(table: &mut Table, val: T) -> Result<Self, TableError> {
        let id = table.push(Box::new(val))?;
        Ok(Self::from(id))
    }

    #[inline(always)]
    pub fn get<'t>(&self, table: &'t Table) -> Result<&'t T, TableError> {
        table.get(self.id)
    }

    #[inline(always)]
    pub fn get_mut<'t>(&self, table: &'t mut Table) -> Result<&'t mut T, TableError> {
        table.get_mut(self.id)
    }

    #[inline(always)]
    pub fn delete(&self, table: &mut Table) -> Result<T, TableError> {
        table.delete(self.id)
    }
}

pub trait TableHttpExt {
    fn push_incoming_request(
        &mut self,
        request: HostIncomingRequest,
    ) -> Result<IncomingRequest, TableError>;
    fn get_incoming_request(
        &mut self,
        id: IncomingRequest,
    ) -> Result<&mut HostIncomingRequest, TableError>;
    fn delete_incoming_request(
        &mut self,
        id: IncomingRequest,
    ) -> Result<HostIncomingRequest, TableError>;

    fn push_outgoing_response(
        &mut self,
        resp: HostOutgoingResponse,
    ) -> Result<OutgoingResponse, TableError>;
    fn get_outgoing_response(&mut self, id: u32) -> Result<&mut HostOutgoingResponse, TableError>;
    fn delete_outgoing_response(&mut self, id: u32) -> Result<HostOutgoingResponse, TableError>;

    fn push_incoming_response(&mut self, response: HostIncomingResponse)
        -> Result<u32, TableError>;
    fn get_incoming_response(&self, id: u32) -> Result<&HostIncomingResponse, TableError>;
    fn get_incoming_response_mut(
        &mut self,
        id: u32,
    ) -> Result<&mut HostIncomingResponse, TableError>;
    fn delete_incoming_response(&mut self, id: u32) -> Result<HostIncomingResponse, TableError>;

    fn push_fields(&mut self, fields: HostFields) -> Result<u32, TableError>;
    fn get_fields(&mut self, id: u32) -> Result<&mut FieldMap, TableError>;
    fn delete_fields(&mut self, id: u32) -> Result<HostFields, TableError>;

    fn push_future_incoming_response(
        &mut self,
        response: HostFutureIncomingResponse,
    ) -> Result<u32, TableError>;
    fn get_future_incoming_response(
        &self,
        id: u32,
    ) -> Result<&HostFutureIncomingResponse, TableError>;
    fn get_future_incoming_response_mut(
        &mut self,
        id: u32,
    ) -> Result<&mut HostFutureIncomingResponse, TableError>;
    fn delete_future_incoming_response(
        &mut self,
        id: u32,
    ) -> Result<HostFutureIncomingResponse, TableError>;

    fn push_incoming_body(&mut self, body: HostIncomingBody) -> Result<IncomingBody, TableError>;
    fn get_incoming_body(&mut self, id: IncomingBody) -> Result<&mut HostIncomingBody, TableError>;
    fn delete_incoming_body(&mut self, id: IncomingBody) -> Result<HostIncomingBody, TableError>;

    fn push_outgoing_body(&mut self, body: HostOutgoingBody) -> Result<OutgoingBody, TableError>;
    fn get_outgoing_body(&mut self, id: OutgoingBody) -> Result<&mut HostOutgoingBody, TableError>;
    fn delete_outgoing_body(&mut self, id: OutgoingBody) -> Result<HostOutgoingBody, TableError>;

    fn push_future_trailers(
        &mut self,
        trailers: HostFutureTrailers,
    ) -> Result<FutureTrailers, TableError>;
    fn get_future_trailers(
        &mut self,
        id: FutureTrailers,
    ) -> Result<&mut HostFutureTrailers, TableError>;
    fn delete_future_trailers(
        &mut self,
        id: FutureTrailers,
    ) -> Result<HostFutureTrailers, TableError>;
}

impl TableHttpExt for Table {
    fn push_incoming_request(
        &mut self,
        request: HostIncomingRequest,
    ) -> Result<IncomingRequest, TableError> {
        self.push(Box::new(request))
    }

    fn get_incoming_request(
        &mut self,
        id: IncomingRequest,
    ) -> Result<&mut HostIncomingRequest, TableError> {
        self.get_mut(id)
    }

    fn delete_incoming_request(
        &mut self,
        id: IncomingRequest,
    ) -> Result<HostIncomingRequest, TableError> {
        self.delete(id)
    }

    fn push_outgoing_response(
        &mut self,
        response: HostOutgoingResponse,
    ) -> Result<OutgoingResponse, TableError> {
        self.push(Box::new(response))
    }

    fn get_outgoing_response(
        &mut self,
        id: OutgoingResponse,
    ) -> Result<&mut HostOutgoingResponse, TableError> {
        self.get_mut(id)
    }

    fn delete_outgoing_response(
        &mut self,
        id: OutgoingResponse,
    ) -> Result<HostOutgoingResponse, TableError> {
        self.delete(id)
    }

    fn push_incoming_response(
        &mut self,
        response: HostIncomingResponse,
    ) -> Result<u32, TableError> {
        self.push(Box::new(response))
    }
    fn get_incoming_response(&self, id: u32) -> Result<&HostIncomingResponse, TableError> {
        self.get::<HostIncomingResponse>(id)
    }
    fn get_incoming_response_mut(
        &mut self,
        id: u32,
    ) -> Result<&mut HostIncomingResponse, TableError> {
        self.get_mut::<HostIncomingResponse>(id)
    }
    fn delete_incoming_response(&mut self, id: u32) -> Result<HostIncomingResponse, TableError> {
        let resp = self.delete::<HostIncomingResponse>(id)?;
        Ok(resp)
    }

    fn push_fields(&mut self, fields: HostFields) -> Result<u32, TableError> {
        match fields {
            HostFields::Ref { parent, .. } => self.push_child(Box::new(fields), parent),
            HostFields::Owned { .. } => self.push(Box::new(fields)),
        }
    }
    fn get_fields(&mut self, id: u32) -> Result<&mut FieldMap, TableError> {
        let fields = self.get_mut::<HostFields>(id)?;
        if let HostFields::Ref { parent, get_fields } = *fields {
            let entry = self.get_any_mut(parent)?;
            return Ok(get_fields(entry));
        }

        match self.get_mut::<HostFields>(id)? {
            HostFields::Owned { fields } => Ok(fields),
            // NB: ideally the `if let` above would go here instead. That makes
            // the borrow-checker unhappy. Unclear why. If you, dear reader, can
            // refactor this to remove the `unreachable!` please do.
            HostFields::Ref { .. } => unreachable!(),
        }
    }
    fn delete_fields(&mut self, id: u32) -> Result<HostFields, TableError> {
        let fields = self.delete::<HostFields>(id)?;
        Ok(fields)
    }

    fn push_future_incoming_response(
        &mut self,
        response: HostFutureIncomingResponse,
    ) -> Result<u32, TableError> {
        self.push(Box::new(response))
    }
    fn get_future_incoming_response(
        &self,
        id: u32,
    ) -> Result<&HostFutureIncomingResponse, TableError> {
        self.get::<HostFutureIncomingResponse>(id)
    }
    fn get_future_incoming_response_mut(
        &mut self,
        id: u32,
    ) -> Result<&mut HostFutureIncomingResponse, TableError> {
        self.get_mut::<HostFutureIncomingResponse>(id)
    }
    fn delete_future_incoming_response(
        &mut self,
        id: u32,
    ) -> Result<HostFutureIncomingResponse, TableError> {
        self.delete(id)
    }

    fn push_incoming_body(&mut self, body: HostIncomingBody) -> Result<IncomingBody, TableError> {
        self.push(Box::new(body))
    }

    fn get_incoming_body(&mut self, id: IncomingBody) -> Result<&mut HostIncomingBody, TableError> {
        self.get_mut(id)
    }

    fn delete_incoming_body(&mut self, id: IncomingBody) -> Result<HostIncomingBody, TableError> {
        self.delete(id)
    }

    fn push_outgoing_body(&mut self, body: HostOutgoingBody) -> Result<OutgoingBody, TableError> {
        self.push(Box::new(body))
    }

    fn get_outgoing_body(&mut self, id: OutgoingBody) -> Result<&mut HostOutgoingBody, TableError> {
        self.get_mut(id)
    }

    fn delete_outgoing_body(&mut self, id: OutgoingBody) -> Result<HostOutgoingBody, TableError> {
        self.delete(id)
    }

    fn push_future_trailers(
        &mut self,
        trailers: HostFutureTrailers,
    ) -> Result<FutureTrailers, TableError> {
        self.push(Box::new(trailers))
    }

    fn get_future_trailers(
        &mut self,
        id: FutureTrailers,
    ) -> Result<&mut HostFutureTrailers, TableError> {
        self.get_mut(id)
    }

    fn delete_future_trailers(
        &mut self,
        id: FutureTrailers,
    ) -> Result<HostFutureTrailers, TableError> {
        self.delete(id)
    }
}

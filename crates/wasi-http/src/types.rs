//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::{
    bindings::http::types::{FutureTrailers, Headers, IncomingBody, Method, Scheme},
    body::{HostFutureTrailers, HostIncomingBody},
};
use std::pin::Pin;
use std::task;
use std::{any::Any, collections::HashMap};
use wasmtime_wasi::preview2::{
    pipe::AsyncReadStream, AbortOnDropJoinHandle, OccupiedEntry, Table, TableError,
};

const MAX_BUF_SIZE: usize = 65_536;

/// Capture the state necessary for use in the wasi-http API implementation.
pub struct WasiHttpCtx;

pub trait WasiHttpView: Send {
    fn ctx(&mut self) -> &mut WasiHttpCtx;
    fn table(&mut self) -> &mut Table;
}

pub struct HostOutgoingRequest {
    pub method: Method,
    pub scheme: Option<Scheme>,
    pub path_with_query: String,
    pub authority: String,
    pub headers: FieldMap,
    pub body: Option<AsyncReadStream>,
}

pub struct HostIncomingResponse {
    pub status: u16,
    pub headers: FieldMap,
    pub body: Option<hyper::body::Incoming>,
    pub worker: AbortOnDropJoinHandle<anyhow::Result<()>>,
}

#[derive(Clone)]
pub struct FieldMap(pub HashMap<String, Vec<Vec<u8>>>);

impl From<hyper::HeaderMap> for FieldMap {
    fn from(headers: hyper::HeaderMap) -> Self {
        use std::collections::hash_map::Entry;

        let mut res: HashMap<String, Vec<Vec<u8>>> = HashMap::new();

        for (k, v) in headers.iter() {
            let v = v.as_bytes().to_vec();
            match res.entry(k.as_str().to_string()) {
                Entry::Occupied(mut vs) => vs.get_mut().push(v),
                Entry::Vacant(e) => {
                    e.insert(vec![v]);
                }
            }
        }

        Self(res)
    }
}

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
}

type FutureIncomingResponseHandle = AbortOnDropJoinHandle<anyhow::Result<IncomingResponseInternal>>;

pub enum HostFutureIncomingResponse {
    Pending(FutureIncomingResponseHandle),
    Ready(anyhow::Result<IncomingResponseInternal>),
}

impl HostFutureIncomingResponse {
    pub fn new(handle: FutureIncomingResponseHandle) -> Self {
        Self::Pending(handle)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Pending(_))
    }

    pub fn unwrap_ready(self) -> anyhow::Result<IncomingResponseInternal> {
        match self {
            Self::Ready(res) => res,
            Self::Pending(_) => {
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

            Self::Ready(_) => task::Poll::Ready(Ok(())),
        }
    }
}

#[async_trait::async_trait]
pub trait TableHttpExt {
    fn push_outgoing_response(&mut self, request: HostOutgoingRequest) -> Result<u32, TableError>;
    fn get_outgoing_request(&self, id: u32) -> Result<&HostOutgoingRequest, TableError>;
    fn get_outgoing_request_mut(&mut self, id: u32)
        -> Result<&mut HostOutgoingRequest, TableError>;
    fn delete_outgoing_request(&mut self, id: u32) -> Result<HostOutgoingRequest, TableError>;

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

#[async_trait::async_trait]
impl TableHttpExt for Table {
    fn push_outgoing_response(&mut self, request: HostOutgoingRequest) -> Result<u32, TableError> {
        self.push(Box::new(request))
    }
    fn get_outgoing_request(&self, id: u32) -> Result<&HostOutgoingRequest, TableError> {
        self.get::<HostOutgoingRequest>(id)
    }
    fn get_outgoing_request_mut(
        &mut self,
        id: u32,
    ) -> Result<&mut HostOutgoingRequest, TableError> {
        self.get_mut::<HostOutgoingRequest>(id)
    }
    fn delete_outgoing_request(&mut self, id: u32) -> Result<HostOutgoingRequest, TableError> {
        let req = self.delete::<HostOutgoingRequest>(id)?;
        Ok(req)
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

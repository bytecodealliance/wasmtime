//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::bindings::http::types::{Method, Scheme};
use bytes::Bytes;
use std::any::Any;
use std::collections::HashMap;
use wasmtime_wasi::preview2::{
    bindings::io::streams::{InputStream, OutputStream},
    pipe::{AsyncReadStream, AsyncWriteStream},
    AbortOnDropJoinHandle, HostInputStream, HostOutputStream, Table, TableError, TableStreamExt,
};

const MAX_BUF_SIZE: usize = 65_536;

/// Capture the state necessary for use in the wasi-http API implementation.
pub struct WasiHttpCtx;

pub trait WasiHttpView: Send {
    fn ctx(&mut self) -> &mut WasiHttpCtx;
    fn table(&mut self) -> &mut Table;
}

pub type FieldsMap = HashMap<String, Vec<Vec<u8>>>;

pub struct HostOutgoingRequest {
    pub method: Method,
    pub scheme: Option<Scheme>,
    pub path_with_query: String,
    pub authority: String,
    pub headers: HostFields,
    pub body: Option<AsyncReadStream>,
}

#[derive(Clone, Debug)]
pub struct HostIncomingResponse {
    pub active: bool,
    pub status: u16,
    pub headers: Option<u32>,
    pub body: Option<u32>,
    pub trailers: Option<u32>,
}

impl HostIncomingResponse {
    pub fn new() -> Self {
        Self {
            active: false,
            status: 0,
            headers: None,
            body: None,
            trailers: None,
        }
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn headers(&self) -> Option<u32> {
        self.headers
    }

    pub fn set_headers(&mut self, headers: u32) {
        self.headers = Some(headers);
    }

    pub fn body(&self) -> Option<u32> {
        self.body
    }

    pub fn set_body(&mut self, body: u32) {
        self.body = Some(body);
    }

    pub fn trailers(&self) -> Option<u32> {
        self.trailers
    }

    pub fn set_trailers(&mut self, trailers: u32) {
        self.trailers = Some(trailers);
    }
}

#[derive(Clone, Debug)]
pub struct HostFields(pub HashMap<String, Vec<Vec<u8>>>);

impl HostFields {
    pub fn new() -> Self {
        Self(FieldsMap::new())
    }
}

pub struct HostFutureIncomingResponse {
    handle: futures::future::MaybeDone<AbortOnDropJoinHandle<HostIncomingResponse>>,
}

impl HostFutureIncomingResponse {
    pub fn new(handle: AbortOnDropJoinHandle<HostIncomingResponse>) -> Self {
        Self { handle: futures::future::maybe_done(handle) }
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
    fn get_fields(&self, id: u32) -> Result<&HostFields, TableError>;
    fn get_fields_mut(&mut self, id: u32) -> Result<&mut HostFields, TableError>;
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
    fn delete_future_incoming_response(&mut self, id: u32) -> Result<(), TableError>;
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
        self.push(Box::new(fields))
    }
    fn get_fields(&self, id: u32) -> Result<&HostFields, TableError> {
        self.get::<HostFields>(id)
    }
    fn get_fields_mut(&mut self, id: u32) -> Result<&mut HostFields, TableError> {
        self.get_mut::<HostFields>(id)
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
    fn delete_future_incoming_response(&mut self, id: u32) -> Result<(), TableError> {
        let _ = self.delete::<HostFutureIncomingResponse>(id)?;
        Ok(())
    }
}

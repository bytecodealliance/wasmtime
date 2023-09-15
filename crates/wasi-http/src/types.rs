//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::bindings::http::types::{
    IncomingStream, Method, OutgoingRequest, OutgoingStream, RequestOptions, Scheme,
};
use bytes::Bytes;
use std::any::Any;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use wasmtime_wasi::preview2::{
    pipe::{AsyncReadStream, AsyncWriteStream},
    HostInputStream, HostOutputStream, Table, TableError, TableStreamExt, WasiView,
};

const MAX_BUF_SIZE: usize = 65_536;

/// Capture the state necessary for use in the wasi-http API implementation.
pub struct WasiHttpCtx {
    pub streams: HashMap<u32, Stream>,
}

impl WasiHttpCtx {
    /// Make a new context from the default state.
    pub fn new() -> Self {
        Self {
            streams: HashMap::new(),
        }
    }
}

pub trait WasiHttpView: WasiView {
    fn http_ctx(&self) -> &WasiHttpCtx;
    fn http_ctx_mut(&mut self) -> &mut WasiHttpCtx;
}

pub type FieldsMap = HashMap<String, Vec<Vec<u8>>>;

#[derive(Clone, Debug)]
pub struct ActiveRequest {
    pub active: bool,
    pub method: Method,
    pub scheme: Option<Scheme>,
    pub path_with_query: String,
    pub authority: String,
    pub headers: Option<u32>,
    pub body: Option<u32>,
}

pub trait HttpRequest: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn as_any(&self) -> &dyn Any;

    fn method(&self) -> &Method;
    fn scheme(&self) -> &Option<Scheme>;
    fn path_with_query(&self) -> &str;
    fn authority(&self) -> &str;
    fn headers(&self) -> Option<u32>;
    fn set_headers(&mut self, headers: u32);
    fn body(&self) -> Option<u32>;
    fn set_body(&mut self, body: u32);
}

impl HttpRequest for ActiveRequest {
    fn new() -> Self {
        Self {
            active: false,
            method: Method::Get,
            scheme: Some(Scheme::Http),
            path_with_query: "".to_string(),
            authority: "".to_string(),
            headers: None,
            body: None,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn method(&self) -> &Method {
        &self.method
    }

    fn scheme(&self) -> &Option<Scheme> {
        &self.scheme
    }

    fn path_with_query(&self) -> &str {
        &self.path_with_query
    }

    fn authority(&self) -> &str {
        &self.authority
    }

    fn headers(&self) -> Option<u32> {
        self.headers
    }

    fn set_headers(&mut self, headers: u32) {
        self.headers = Some(headers);
    }

    fn body(&self) -> Option<u32> {
        self.body
    }

    fn set_body(&mut self, body: u32) {
        self.body = Some(body);
    }
}

#[derive(Clone, Debug)]
pub struct ActiveResponse {
    pub active: bool,
    pub status: u16,
    pub headers: Option<u32>,
    pub body: Option<u32>,
    pub trailers: Option<u32>,
}

pub trait HttpResponse: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn as_any(&self) -> &dyn Any;

    fn status(&self) -> u16;
    fn headers(&self) -> Option<u32>;
    fn set_headers(&mut self, headers: u32);
    fn body(&self) -> Option<u32>;
    fn set_body(&mut self, body: u32);
    fn trailers(&self) -> Option<u32>;
    fn set_trailers(&mut self, trailers: u32);
}

impl HttpResponse for ActiveResponse {
    fn new() -> Self {
        Self {
            active: false,
            status: 0,
            headers: None,
            body: None,
            trailers: None,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn status(&self) -> u16 {
        self.status
    }

    fn headers(&self) -> Option<u32> {
        self.headers
    }

    fn set_headers(&mut self, headers: u32) {
        self.headers = Some(headers);
    }

    fn body(&self) -> Option<u32> {
        self.body
    }

    fn set_body(&mut self, body: u32) {
        self.body = Some(body);
    }

    fn trailers(&self) -> Option<u32> {
        self.trailers
    }

    fn set_trailers(&mut self, trailers: u32) {
        self.trailers = Some(trailers);
    }
}

#[derive(Clone, Debug)]
pub struct ActiveFuture {
    request_id: OutgoingRequest,
    options: Option<RequestOptions>,
    response_id: Option<u32>,
    pollable_id: Option<u32>,
}

impl ActiveFuture {
    pub fn new(request_id: OutgoingRequest, options: Option<RequestOptions>) -> Self {
        Self {
            request_id,
            options,
            response_id: None,
            pollable_id: None,
        }
    }

    pub fn request_id(&self) -> u32 {
        self.request_id
    }

    pub fn options(&self) -> Option<RequestOptions> {
        self.options
    }

    pub fn response_id(&self) -> Option<u32> {
        self.response_id
    }

    pub fn set_response_id(&mut self, response_id: u32) {
        self.response_id = Some(response_id);
    }

    pub fn pollable_id(&self) -> Option<u32> {
        self.pollable_id
    }

    pub fn set_pollable_id(&mut self, pollable_id: u32) {
        self.pollable_id = Some(pollable_id);
    }
}

#[derive(Clone, Debug)]
pub struct ActiveFields(HashMap<String, Vec<Vec<u8>>>);

impl ActiveFields {
    pub fn new() -> Self {
        Self(FieldsMap::new())
    }
}

pub trait HttpFields: Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

impl HttpFields for ActiveFields {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Deref for ActiveFields {
    type Target = FieldsMap;
    fn deref(&self) -> &FieldsMap {
        &self.0
    }
}

impl DerefMut for ActiveFields {
    fn deref_mut(&mut self) -> &mut FieldsMap {
        &mut self.0
    }
}

#[derive(Clone, Debug)]
pub struct Stream {
    input_id: u32,
    output_id: u32,
    parent_id: u32,
}

impl Stream {
    pub fn new(input_id: u32, output_id: u32, parent_id: u32) -> Self {
        Self {
            input_id,
            output_id,
            parent_id,
        }
    }

    pub fn incoming(&self) -> IncomingStream {
        self.input_id
    }

    pub fn outgoing(&self) -> OutgoingStream {
        self.output_id
    }

    pub fn parent_id(&self) -> u32 {
        self.parent_id
    }
}

#[async_trait::async_trait]
pub trait TableHttpExt {
    fn push_request(&mut self, request: Box<dyn HttpRequest>) -> Result<u32, TableError>;
    fn get_request(&self, id: u32) -> Result<&(dyn HttpRequest), TableError>;
    fn get_request_mut(&mut self, id: u32) -> Result<&mut Box<dyn HttpRequest>, TableError>;
    fn delete_request(&mut self, id: u32) -> Result<(), TableError>;

    fn push_response(&mut self, response: Box<dyn HttpResponse>) -> Result<u32, TableError>;
    fn get_response(&self, id: u32) -> Result<&dyn HttpResponse, TableError>;
    fn get_response_mut(&mut self, id: u32) -> Result<&mut Box<dyn HttpResponse>, TableError>;
    fn delete_response(&mut self, id: u32) -> Result<(), TableError>;

    fn push_future(&mut self, future: Box<ActiveFuture>) -> Result<u32, TableError>;
    fn get_future(&self, id: u32) -> Result<&ActiveFuture, TableError>;
    fn get_future_mut(&mut self, id: u32) -> Result<&mut Box<ActiveFuture>, TableError>;
    fn delete_future(&mut self, id: u32) -> Result<(), TableError>;

    fn push_fields(&mut self, fields: Box<ActiveFields>) -> Result<u32, TableError>;
    fn get_fields(&self, id: u32) -> Result<&ActiveFields, TableError>;
    fn get_fields_mut(&mut self, id: u32) -> Result<&mut Box<ActiveFields>, TableError>;
    fn delete_fields(&mut self, id: u32) -> Result<(), TableError>;

    async fn push_stream(
        &mut self,
        content: Bytes,
        parent: u32,
    ) -> Result<(u32, Stream), TableError>;
    fn get_stream(&self, id: u32) -> Result<&Stream, TableError>;
    fn get_stream_mut(&mut self, id: u32) -> Result<&mut Box<Stream>, TableError>;
    fn delete_stream(&mut self, id: u32) -> Result<(), TableError>;
}

#[async_trait::async_trait]
impl TableHttpExt for Table {
    fn push_request(&mut self, request: Box<dyn HttpRequest>) -> Result<u32, TableError> {
        self.push(Box::new(request))
    }
    fn get_request(&self, id: u32) -> Result<&dyn HttpRequest, TableError> {
        self.get::<Box<dyn HttpRequest>>(id).map(|f| f.as_ref())
    }
    fn get_request_mut(&mut self, id: u32) -> Result<&mut Box<dyn HttpRequest>, TableError> {
        self.get_mut::<Box<dyn HttpRequest>>(id)
    }
    fn delete_request(&mut self, id: u32) -> Result<(), TableError> {
        self.delete::<Box<dyn HttpRequest>>(id).map(|_old| ())
    }

    fn push_response(&mut self, response: Box<dyn HttpResponse>) -> Result<u32, TableError> {
        self.push(Box::new(response))
    }
    fn get_response(&self, id: u32) -> Result<&dyn HttpResponse, TableError> {
        self.get::<Box<dyn HttpResponse>>(id).map(|f| f.as_ref())
    }
    fn get_response_mut(&mut self, id: u32) -> Result<&mut Box<dyn HttpResponse>, TableError> {
        self.get_mut::<Box<dyn HttpResponse>>(id)
    }
    fn delete_response(&mut self, id: u32) -> Result<(), TableError> {
        self.delete::<Box<dyn HttpResponse>>(id).map(|_old| ())
    }

    fn push_future(&mut self, future: Box<ActiveFuture>) -> Result<u32, TableError> {
        self.push(Box::new(future))
    }
    fn get_future(&self, id: u32) -> Result<&ActiveFuture, TableError> {
        self.get::<Box<ActiveFuture>>(id).map(|f| f.as_ref())
    }
    fn get_future_mut(&mut self, id: u32) -> Result<&mut Box<ActiveFuture>, TableError> {
        self.get_mut::<Box<ActiveFuture>>(id)
    }
    fn delete_future(&mut self, id: u32) -> Result<(), TableError> {
        self.delete::<Box<ActiveFuture>>(id).map(|_old| ())
    }

    fn push_fields(&mut self, fields: Box<ActiveFields>) -> Result<u32, TableError> {
        self.push(Box::new(fields))
    }
    fn get_fields(&self, id: u32) -> Result<&ActiveFields, TableError> {
        self.get::<Box<ActiveFields>>(id).map(|f| f.as_ref())
    }
    fn get_fields_mut(&mut self, id: u32) -> Result<&mut Box<ActiveFields>, TableError> {
        self.get_mut::<Box<ActiveFields>>(id)
    }
    fn delete_fields(&mut self, id: u32) -> Result<(), TableError> {
        self.delete::<Box<ActiveFields>>(id).map(|_old| ())
    }

    async fn push_stream(
        &mut self,
        mut content: Bytes,
        parent: u32,
    ) -> Result<(u32, Stream), TableError> {
        tracing::debug!("preparing http body stream");
        let (a, b) = tokio::io::duplex(MAX_BUF_SIZE);
        let (_, write_stream) = tokio::io::split(a);
        let (read_stream, _) = tokio::io::split(b);
        let input_stream = AsyncReadStream::new(read_stream);
        // TODO: more informed budget here
        let mut output_stream = AsyncWriteStream::new(4096, write_stream);

        while !content.is_empty() {
            let permit = output_stream
                .write_ready()
                .await
                .map_err(|_| TableError::NotPresent)?;

            let len = content.len().min(permit);
            let chunk = content.split_to(len);

            output_stream
                .write(chunk)
                .map_err(|_| TableError::NotPresent)?;
        }
        output_stream.flush().map_err(|_| TableError::NotPresent)?;
        let _readiness = tokio::time::timeout(
            std::time::Duration::from_millis(10),
            output_stream.write_ready(),
        )
        .await;

        let input_stream = Box::new(input_stream);
        let output_id = self.push_output_stream(Box::new(output_stream))?;
        let input_id = self.push_input_stream(input_stream)?;
        let stream = Stream::new(input_id, output_id, parent);
        let cloned_stream = stream.clone();
        let stream_id = self.push(Box::new(Box::new(stream)))?;
        tracing::trace!(
            "http body stream details ( id: {:?}, input: {:?}, output: {:?} )",
            stream_id,
            input_id,
            output_id
        );
        Ok((stream_id, cloned_stream))
    }
    fn get_stream(&self, id: u32) -> Result<&Stream, TableError> {
        self.get::<Box<Stream>>(id).map(|f| f.as_ref())
    }
    fn get_stream_mut(&mut self, id: u32) -> Result<&mut Box<Stream>, TableError> {
        self.get_mut::<Box<Stream>>(id)
    }
    fn delete_stream(&mut self, id: u32) -> Result<(), TableError> {
        let stream = self.get_stream_mut(id)?;
        let input_stream = stream.incoming();
        let output_stream = stream.outgoing();
        self.delete::<Box<Stream>>(id).map(|_old| ())?;
        self.delete::<Box<dyn HostInputStream>>(input_stream)
            .map(|_old| ())?;
        self.delete::<Box<dyn HostOutputStream>>(output_stream)
            .map(|_old| ())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn instantiate() {
        WasiHttpCtx::new();
    }
}

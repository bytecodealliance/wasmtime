//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::common::{Error, InputStream, OutputStream, Table};
use crate::wasi::http::types::{Method, RequestOptions, Scheme};
use std::any::Any;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

/// Capture the state necessary for use in the wasi-http API implementation.
pub struct WasiHttpCtx {
    pub table: Table,
}

impl WasiHttpCtx {
    /// Make a new context from the default state.
    pub fn new() -> Self {
        Self {
            table: Table::new(),
        }
    }

    pub fn table(&self) -> &Table {
        &self.table
    }

    pub fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }

    pub fn insert_input_stream(&mut self, id: u32, stream: Box<dyn InputStream>) {
        self.table_mut().insert_at(id, Box::new(stream));
    }

    pub fn push_input_stream(&mut self, stream: Box<dyn InputStream>) -> Result<u32, Error> {
        self.table_mut().push(Box::new(stream))
    }

    pub fn insert_output_stream(&mut self, id: u32, stream: Box<dyn OutputStream>) {
        self.table_mut().insert_at(id, Box::new(stream));
    }

    pub fn push_output_stream(&mut self, stream: Box<dyn OutputStream>) -> Result<u32, Error> {
        self.table_mut().push(Box::new(stream))
    }

    pub fn push_request(&mut self, request: Box<dyn HttpRequest>) -> Result<u32, Error> {
        self.table_mut().push(Box::new(request))
    }

    pub fn push_response(&mut self, response: Box<dyn HttpResponse>) -> Result<u32, Error> {
        self.table_mut().push(Box::new(response))
    }

    pub fn push_future(&mut self, future: Box<ActiveFuture>) -> Result<u32, Error> {
        self.table_mut().push(Box::new(future))
    }

    pub fn push_fields(&mut self, fields: Box<ActiveFields>) -> Result<u32, Error> {
        self.table_mut().push(Box::new(fields))
    }

    pub fn set_stdin(&mut self, s: Box<dyn InputStream>) {
        self.insert_input_stream(0, s);
    }

    pub fn set_stdout(&mut self, s: Box<dyn OutputStream>) {
        self.insert_output_stream(1, s);
    }

    pub fn set_stderr(&mut self, s: Box<dyn OutputStream>) {
        self.insert_output_stream(2, s);
    }
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

#[derive(Clone)]
pub struct ActiveFuture {
    pub request_id: u32,
    pub options: Option<RequestOptions>,
}

impl ActiveFuture {
    pub fn new(request_id: u32, options: Option<RequestOptions>) -> Self {
        Self {
            request_id,
            options,
        }
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

pub trait TableExt {
    fn get_request(&self, id: u32) -> Result<&(dyn HttpRequest), Error>;
    fn get_request_mut(&mut self, id: u32) -> Result<&mut Box<dyn HttpRequest>, Error>;
    fn delete_request(&mut self, id: u32) -> Result<(), Error>;

    fn get_response(&self, id: u32) -> Result<&dyn HttpResponse, Error>;
    fn get_response_mut(&mut self, id: u32) -> Result<&mut Box<dyn HttpResponse>, Error>;
    fn delete_response(&mut self, id: u32) -> Result<(), Error>;

    fn get_future(&self, id: u32) -> Result<&ActiveFuture, Error>;
    fn get_future_mut(&mut self, id: u32) -> Result<&mut Box<ActiveFuture>, Error>;
    fn delete_future(&mut self, id: u32) -> Result<(), Error>;

    fn get_fields(&self, id: u32) -> Result<&ActiveFields, Error>;
    fn get_fields_mut(&mut self, id: u32) -> Result<&mut Box<ActiveFields>, Error>;
    fn delete_fields(&mut self, id: u32) -> Result<(), Error>;

    fn get_response_by_stream(&self, id: u32) -> Result<&dyn HttpResponse, Error>;
}

impl TableExt for crate::common::Table {
    fn get_request(&self, id: u32) -> Result<&dyn HttpRequest, Error> {
        self.get::<Box<dyn HttpRequest>>(id).map(|f| f.as_ref())
    }
    fn get_request_mut(&mut self, id: u32) -> Result<&mut Box<dyn HttpRequest>, Error> {
        self.get_mut::<Box<dyn HttpRequest>>(id)
    }
    fn delete_request(&mut self, id: u32) -> Result<(), Error> {
        self.delete::<Box<dyn HttpRequest>>(id).map(|_old| ())
    }

    fn get_response(&self, id: u32) -> Result<&dyn HttpResponse, Error> {
        self.get::<Box<dyn HttpResponse>>(id).map(|f| f.as_ref())
    }
    fn get_response_mut(&mut self, id: u32) -> Result<&mut Box<dyn HttpResponse>, Error> {
        self.get_mut::<Box<dyn HttpResponse>>(id)
    }
    fn delete_response(&mut self, id: u32) -> Result<(), Error> {
        self.delete::<Box<dyn HttpResponse>>(id).map(|_old| ())
    }

    fn get_future(&self, id: u32) -> Result<&ActiveFuture, Error> {
        self.get::<Box<ActiveFuture>>(id).map(|f| f.as_ref())
    }
    fn get_future_mut(&mut self, id: u32) -> Result<&mut Box<ActiveFuture>, Error> {
        self.get_mut::<Box<ActiveFuture>>(id)
    }
    fn delete_future(&mut self, id: u32) -> Result<(), Error> {
        self.delete::<Box<ActiveFuture>>(id).map(|_old| ())
    }

    fn get_fields(&self, id: u32) -> Result<&ActiveFields, Error> {
        self.get::<Box<ActiveFields>>(id).map(|f| f.as_ref())
    }
    fn get_fields_mut(&mut self, id: u32) -> Result<&mut Box<ActiveFields>, Error> {
        self.get_mut::<Box<ActiveFields>>(id)
    }
    fn delete_fields(&mut self, id: u32) -> Result<(), Error> {
        self.delete::<Box<ActiveFields>>(id).map(|_old| ())
    }

    fn get_response_by_stream(&self, id: u32) -> Result<&dyn HttpResponse, Error> {
        for value in self.list::<Box<dyn HttpResponse>>().into_values() {
            if Some(id) == value.body() {
                return Ok(value.as_ref());
            }
        }
        Err(Error::trap(anyhow::Error::msg("response not found")))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn instantiate() {
        WasiHttpCtx::new().unwrap();
    }
}

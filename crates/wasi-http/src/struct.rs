use crate::wasi::http::types::{Method, RequestOptions, Scheme};
use bytes::{BufMut, Bytes, BytesMut};
use http_acl::HttpAcl;
use std::collections::HashMap;
use std::fmt;

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let scheme_str = match self {
            Scheme::Http => "http",
            Scheme::Https => "https",
            Scheme::Other(s) => s,
        };
        write!(f, "{}", scheme_str)
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let method_str = match self {
            Method::Get => "GET",
            Method::Put => "PUT",
            Method::Post => "POST",
            Method::Options => "OPTIONS",
            Method::Head => "HEAD",
            Method::Patch => "PATCH",
            Method::Connect => "CONNECT",
            Method::Delete => "DELETE",
            Method::Trace => "TRACE",
            Method::Other(s) => s,
        };
        write!(f, "{}", method_str)
    }
}

#[derive(Clone, Default)]
pub struct Stream {
    pub closed: bool,
    pub data: BytesMut,
}

#[derive(Clone)]
pub struct WasiHttp {
    pub request_id_base: u32,
    pub response_id_base: u32,
    pub fields_id_base: u32,
    pub streams_id_base: u32,
    pub future_id_base: u32,
    pub requests: HashMap<u32, ActiveRequest>,
    pub responses: HashMap<u32, ActiveResponse>,
    pub fields: HashMap<u32, HashMap<String, Vec<Vec<u8>>>>,
    pub streams: HashMap<u32, Stream>,
    pub futures: HashMap<u32, ActiveFuture>,
    pub acl: HttpAcl,
}

#[derive(Clone)]
pub struct ActiveRequest {
    pub id: u32,
    pub active_request: bool,
    pub method: Method,
    pub scheme: Option<Scheme>,
    pub path_with_query: String,
    pub authority: String,
    pub headers: HashMap<String, Vec<Vec<u8>>>,
    pub body: u32,
}

#[derive(Clone)]
pub struct ActiveResponse {
    pub id: u32,
    pub active_response: bool,
    pub status: u16,
    pub body: u32,
    pub response_headers: HashMap<String, Vec<Vec<u8>>>,
    pub trailers: u32,
}

#[derive(Clone)]
pub struct ActiveFuture {
    pub id: u32,
    pub request_id: u32,
    pub options: Option<RequestOptions>,
}

impl ActiveRequest {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            active_request: false,
            method: Method::Get,
            scheme: Some(Scheme::Http),
            path_with_query: "".to_string(),
            authority: "".to_string(),
            headers: HashMap::new(),
            body: 0,
        }
    }
}

impl ActiveResponse {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            active_response: false,
            status: 0,
            body: 0,
            response_headers: HashMap::new(),
            trailers: 0,
        }
    }
}

impl ActiveFuture {
    pub fn new(id: u32, request_id: u32, options: Option<RequestOptions>) -> Self {
        Self {
            id,
            request_id,
            options,
        }
    }
}

impl Stream {
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<Bytes> for Stream {
    fn from(bytes: Bytes) -> Self {
        let mut buf = BytesMut::with_capacity(bytes.len());
        buf.put(bytes);
        Self {
            closed: false,
            data: buf,
        }
    }
}

impl WasiHttp {
    pub fn new() -> Self {
        Self {
            request_id_base: 1,
            response_id_base: 1,
            fields_id_base: 1,
            streams_id_base: 1,
            future_id_base: 1,
            requests: HashMap::new(),
            responses: HashMap::new(),
            fields: HashMap::new(),
            streams: HashMap::new(),
            futures: HashMap::new(),
            acl: HttpAcl::builder()
                .host_acl_default(true)
                .port_acl_default(true)
                .ip_acl_default(true)
                .build(),
        }
    }

    pub fn new_with_acl(acl: HttpAcl) -> Self {
        Self {
            request_id_base: 1,
            response_id_base: 1,
            fields_id_base: 1,
            streams_id_base: 1,
            future_id_base: 1,
            requests: HashMap::new(),
            responses: HashMap::new(),
            fields: HashMap::new(),
            streams: HashMap::new(),
            futures: HashMap::new(),
            acl,
        }
    }
}

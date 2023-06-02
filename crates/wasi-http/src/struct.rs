use crate::wasi::http::types::{Method, RequestOptions, Scheme};
use bytes::{BufMut, Bytes, BytesMut};
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct Stream {
    pub closed: bool,
    pub data: BytesMut,
}

impl crate::wasi::http::types::Method {
    pub fn new(m: &hyper::Method) -> Self {
        match m {
            &hyper::Method::GET => Method::Get,
            &hyper::Method::PUT => Method::Put,
            &hyper::Method::POST => Method::Post,
            &hyper::Method::DELETE => Method::Delete,
            &hyper::Method::OPTIONS => Method::Options,
            &hyper::Method::HEAD => Method::Head,
            &hyper::Method::CONNECT => Method::Connect,
            &hyper::Method::TRACE => Method::Trace,
            &hyper::Method::PATCH => Method::Patch,
            _ => panic!("unknown method!"),
        }
    }
}

impl From<&http::uri::Scheme> for crate::wasi::http::types::Scheme {
    fn from(s: &http::uri::Scheme) -> crate::wasi::http::types::Scheme {
        match s.as_str() {
            "http" => crate::wasi::http::types::Scheme::Http,
            "https" => crate::wasi::http::types::Scheme::Https,
            _ => panic!("unsupported scheme!"),
        }
    }
}

impl From<crate::wasi::http::types::Method> for u32 {
    fn from(e: crate::wasi::http::types::Method) -> u32 {
        match e {
            Method::Get => 0,
            Method::Head => 1,
            Method::Post => 2,
            Method::Put => 3,
            Method::Delete => 4,
            Method::Connect => 5,
            Method::Options => 6,
            Method::Trace => 7,
            Method::Patch => 8,
            _ => panic!("unknown method"),
        }
    }
}

#[derive(Clone)]
pub struct WasiHttp {
    pub request_id_base: u32,
    pub response_id_base: u32,
    pub fields_id_base: u32,
    pub streams_id_base: u32,
    pub future_id_base: u32,
    pub outparams_id_base: u32,

    pub requests: HashMap<u32, ActiveRequest>,
    pub responses: HashMap<u32, ActiveResponse>,
    pub fields: HashMap<u32, HashMap<String, Vec<Vec<u8>>>>,
    pub streams: HashMap<u32, Stream>,
    pub futures: HashMap<u32, ActiveFuture>,
    pub response_outparams: HashMap<u32, Option<u32>>,
}

#[derive(Clone)]
pub struct ActiveRequest {
    pub id: u32,
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

impl From<&mut Stream> for bytes::Bytes {
    fn from(stream: &mut Stream) -> Self {
        stream.data.clone().into()
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
            outparams_id_base: 1,

            requests: HashMap::new(),
            responses: HashMap::new(),
            fields: HashMap::new(),
            streams: HashMap::new(),
            futures: HashMap::new(),
            response_outparams: HashMap::new(),
        }
    }
}

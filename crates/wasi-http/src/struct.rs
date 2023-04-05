use crate::types::{Method, Scheme};
use bytes::Bytes;
use std::collections::HashMap;

#[derive(Clone)]
pub struct WasiHttp {
    pub request_id_base: u32,
    pub response_id_base: u32,
    pub fields_id_base: u32,
    pub streams_id_base: u32,
    pub requests: HashMap<u32, ActiveRequest>,
    pub responses: HashMap<u32, ActiveResponse>,
    pub fields: HashMap<u32, HashMap<String, Vec<String>>>,
    pub streams: HashMap<u32, Bytes>,
}

#[derive(Clone)]
pub struct ActiveRequest {
    pub id: u32,
    pub active_request: bool,
    pub method: Method,
    pub scheme: Option<Scheme>,
    pub path: String,
    pub query: String,
    pub authority: String,
    pub headers: HashMap<String, Vec<String>>,
    pub body: u32,
}

#[derive(Clone)]
pub struct ActiveResponse {
    pub id: u32,
    pub active_response: bool,
    pub status: u16,
    pub body: u32,
    pub response_headers: HashMap<String, Vec<String>>,
}

impl ActiveRequest {
    pub fn new(id: u32) -> Self {
        Self {
            id: id,
            active_request: false,
            method: Method::Get,
            scheme: Some(Scheme::Http),
            path: "".to_string(),
            query: "".to_string(),
            authority: "".to_string(),
            headers: HashMap::new(),
            body: 0,
        }
    }
}

impl ActiveResponse {
    pub fn new(id: u32) -> Self {
        Self {
            id: id,
            active_response: false,
            status: 0,
            body: 0,
            response_headers: HashMap::new(),
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
            requests: HashMap::new(),
            responses: HashMap::new(),
            fields: HashMap::new(),
            streams: HashMap::new(),
        }
    }
}

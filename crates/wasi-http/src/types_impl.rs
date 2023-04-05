use crate::poll::Pollable;
use crate::r#struct::ActiveRequest;
use crate::types::{
    Error, Fields, FutureIncomingResponse, Headers, IncomingRequest, IncomingResponse,
    IncomingStream, Method, OutgoingRequest, OutgoingResponse, OutgoingStream, ResponseOutparam,
    Scheme, StatusCode, Trailers,
};
use crate::WasiHttp;
use anyhow::{anyhow, bail};
use std::collections::HashMap;

impl crate::types::Host for WasiHttp {
    fn drop_fields(&mut self, fields: Fields) -> wasmtime::Result<()> {
        self.fields.remove(&fields);
        Ok(())
    }
    fn new_fields(&mut self, entries: Vec<(String, String)>) -> wasmtime::Result<Fields> {
        let mut map = HashMap::new();
        for item in entries.iter() {
            let mut vec = std::vec::Vec::new();
            vec.push(item.1.clone());
            map.insert(item.0.clone(), vec);
        }

        let id = self.fields_id_base;
        self.fields_id_base = id + 1;
        self.fields.insert(id, map);

        Ok(id)
    }
    fn fields_get(&mut self, fields: Fields, name: String) -> wasmtime::Result<Vec<String>> {
        let res = self
            .fields
            .get(&fields)
            .ok_or_else(|| anyhow!("fields not found: {fields}"))?
            .get(&name)
            .ok_or_else(|| anyhow!("key not found: {name}"))?
            .clone();
        Ok(res)
    }
    fn fields_set(
        &mut self,
        fields: Fields,
        name: String,
        value: Vec<String>,
    ) -> wasmtime::Result<()> {
        match self.fields.get_mut(&fields) {
            Some(m) => {
                m.insert(name, value.clone());
                Ok(())
            }
            None => bail!("fields not found"),
        }
    }
    fn fields_delete(&mut self, fields: Fields, name: String) -> wasmtime::Result<()> {
        match self.fields.get_mut(&fields) {
            Some(m) => m.remove(&name),
            None => None,
        };
        Ok(())
    }
    fn fields_append(
        &mut self,
        fields: Fields,
        name: String,
        value: String,
    ) -> wasmtime::Result<()> {
        let m = self
            .fields
            .get_mut(&fields)
            .ok_or_else(|| anyhow!("unknown fields: {fields}"))?;
        match m.get_mut(&name) {
            Some(v) => v.push(value),
            None => {
                let mut vec = std::vec::Vec::new();
                vec.push(value);
                m.insert(name, vec);
            }
        };
        Ok(())
    }
    fn fields_entries(&mut self, fields: Fields) -> wasmtime::Result<Vec<(String, String)>> {
        let field_map = match self.fields.get(&fields) {
            Some(m) => m,
            None => bail!("fields not found."),
        };
        let mut result = Vec::new();
        for (name, value) in field_map {
            result.push((name.clone(), value[0].clone()));
        }
        Ok(result)
    }
    fn fields_clone(&mut self, fields: Fields) -> wasmtime::Result<Fields> {
        let id = self.fields_id_base;
        self.fields_id_base = self.fields_id_base + 1;

        let m = self
            .fields
            .get(&fields)
            .ok_or_else(|| anyhow!("fields not found: {fields}"))?;
        self.fields.insert(id, m.clone());
        Ok(id)
    }
    fn finish_incoming_stream(&mut self, _s: IncomingStream) -> wasmtime::Result<Option<Trailers>> {
        bail!("unimplemented: finish_incoming_stream")
    }
    fn finish_outgoing_stream(
        &mut self,
        _s: OutgoingStream,
        _trailers: Option<Trailers>,
    ) -> wasmtime::Result<()> {
        bail!("unimplemented: finish_outgoing_stream")
    }
    fn drop_incoming_request(&mut self, _request: IncomingRequest) -> wasmtime::Result<()> {
        bail!("unimplemented: drop_incoming_request")
    }
    fn drop_outgoing_request(&mut self, request: OutgoingRequest) -> wasmtime::Result<()> {
        self.requests.remove(&request);
        Ok(())
    }
    fn incoming_request_method(&mut self, _request: IncomingRequest) -> wasmtime::Result<Method> {
        bail!("unimplemented: incoming_request_method")
    }
    fn incoming_request_path(&mut self, _request: IncomingRequest) -> wasmtime::Result<String> {
        bail!("unimplemented: incoming_request_path")
    }
    fn incoming_request_scheme(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Option<Scheme>> {
        bail!("unimplemented: incoming_request_scheme")
    }
    fn incoming_request_authority(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<String> {
        bail!("unimplemented: incoming_request_authority")
    }
    fn incoming_request_headers(&mut self, _request: IncomingRequest) -> wasmtime::Result<Headers> {
        bail!("unimplemented: incoming_request_headers")
    }
    fn incoming_request_consume(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Result<IncomingStream, ()>> {
        bail!("unimplemented: incoming_request_consume")
    }
    fn incoming_request_query(&mut self, _request: IncomingRequest) -> wasmtime::Result<String> {
        bail!("unimplemented: incoming_request_query")
    }
    fn new_outgoing_request(
        &mut self,
        method: Method,
        path: String,
        query: String,
        scheme: Option<Scheme>,
        authority: String,
        headers: Headers,
    ) -> wasmtime::Result<OutgoingRequest> {
        let id = self.request_id_base;
        self.request_id_base = self.request_id_base + 1;

        let mut req = ActiveRequest::new(id);
        req.path = path;
        req.query = query;
        req.authority = authority;
        req.method = method;
        req.headers = match self.fields.get(&headers) {
            Some(h) => h.clone(),
            None => bail!("headers not found."),
        };
        req.scheme = scheme;
        self.requests.insert(id, req);
        Ok(id)
    }
    fn outgoing_request_write(
        &mut self,
        request: OutgoingRequest,
    ) -> wasmtime::Result<Result<OutgoingStream, ()>> {
        let mut req = self
            .requests
            .get_mut(&request)
            .ok_or_else(|| anyhow!("unknown request: {request}"))?;
        req.body = self.streams_id_base;
        self.streams_id_base = self.streams_id_base + 1;
        Ok(Ok(req.body))
    }
    fn drop_response_outparam(&mut self, _response: ResponseOutparam) -> wasmtime::Result<()> {
        bail!("unimplemented: drop_response_outparam")
    }
    fn set_response_outparam(
        &mut self,
        _outparam: ResponseOutparam,
        _response: Result<OutgoingResponse, Error>,
    ) -> wasmtime::Result<Result<(), ()>> {
        bail!("unimplemented: set_response_outparam")
    }
    fn drop_incoming_response(&mut self, response: IncomingResponse) -> wasmtime::Result<()> {
        self.responses.remove(&response);
        Ok(())
    }
    fn drop_outgoing_response(&mut self, _response: OutgoingResponse) -> wasmtime::Result<()> {
        bail!("unimplemented: drop_outgoing_response")
    }
    fn incoming_response_status(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<StatusCode> {
        let r = self
            .responses
            .get(&response)
            .ok_or_else(|| anyhow!("response not found: {response}"))?;
        Ok(r.status)
    }
    fn incoming_response_headers(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<Headers> {
        let r = self
            .responses
            .get(&response)
            .ok_or_else(|| anyhow!("response not found: {response}"))?;
        let id = self.fields_id_base;
        self.fields_id_base = self.fields_id_base + 1;

        self.fields.insert(id, r.response_headers.clone());
        Ok(id)
    }
    fn incoming_response_consume(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<Result<IncomingStream, ()>> {
        let r = self
            .responses
            .get(&response)
            .ok_or_else(|| anyhow!("response not found: {response}"))?;

        Ok(Ok(r.body))
    }
    fn new_outgoing_response(
        &mut self,
        _status_code: StatusCode,
        _headers: Headers,
    ) -> wasmtime::Result<OutgoingResponse> {
        bail!("unimplemented: new_outgoing_response")
    }
    fn outgoing_response_write(
        &mut self,
        _response: OutgoingResponse,
    ) -> wasmtime::Result<Result<OutgoingStream, ()>> {
        bail!("unimplemented: outgoing_response_write")
    }
    fn drop_future_incoming_response(
        &mut self,
        _f: FutureIncomingResponse,
    ) -> wasmtime::Result<()> {
        bail!("unimplemented: drop_future_incoming_response")
    }
    fn future_incoming_response_get(
        &mut self,
        _f: FutureIncomingResponse,
    ) -> wasmtime::Result<Option<Result<IncomingResponse, Error>>> {
        bail!("unimplemented: future_incoming_response_get")
    }
    fn listen_to_future_incoming_response(
        &mut self,
        _f: FutureIncomingResponse,
    ) -> wasmtime::Result<Pollable> {
        bail!("unimplemented: listen_to_future_incoming_response")
    }
}

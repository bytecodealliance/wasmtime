use crate::{
    proxy::wasi,
    proxy::wasi::poll::Pollable,
    proxy::wasi::types::{
        Error, Fields, FutureIncomingResponse, Headers, IncomingRequest, IncomingResponse,
        IncomingStream, Method, OutgoingRequest, OutgoingResponse, OutgoingStream,
        ResponseOutparam, Scheme, StatusCode, Trailers,
    },
    WasiCtx,
};

#[async_trait::async_trait]
impl wasi::types::Host for WasiCtx {
    async fn drop_fields(&mut self, _fields: Fields) -> wasmtime::Result<()> {
        anyhow::bail!("not implemented")
    }
    async fn new_fields(&mut self, _entries: Vec<(String, String)>) -> wasmtime::Result<Fields> {
        anyhow::bail!("not implemented")
    }
    async fn fields_get(
        &mut self,
        _fields: Fields,
        _name: String,
    ) -> wasmtime::Result<Vec<String>> {
        anyhow::bail!("not implemented")
    }
    async fn fields_set(
        &mut self,
        _fields: Fields,
        _name: String,
        _value: Vec<String>,
    ) -> wasmtime::Result<()> {
        anyhow::bail!("not implemented")
    }
    async fn fields_delete(&mut self, _fields: Fields, _name: String) -> wasmtime::Result<()> {
        anyhow::bail!("not implemented")
    }
    async fn fields_append(
        &mut self,
        _fields: Fields,
        _name: String,
        _value: String,
    ) -> wasmtime::Result<()> {
        anyhow::bail!("not implemented")
    }
    async fn fields_entries(&mut self, _fields: Fields) -> wasmtime::Result<Vec<(String, String)>> {
        anyhow::bail!("not implemented")
    }
    async fn fields_clone(&mut self, _fields: Fields) -> wasmtime::Result<Fields> {
        anyhow::bail!("not implemented")
    }
    async fn finish_incoming_stream(
        &mut self,
        _s: IncomingStream,
    ) -> wasmtime::Result<Option<Trailers>> {
        anyhow::bail!("not implemented")
    }
    async fn finish_outgoing_stream(
        &mut self,
        _s: OutgoingStream,
        _trailers: Option<Trailers>,
    ) -> wasmtime::Result<()> {
        anyhow::bail!("not implemented")
    }
    async fn drop_incoming_request(&mut self, _request: IncomingRequest) -> wasmtime::Result<()> {
        anyhow::bail!("not implemented")
    }
    async fn drop_outgoing_request(&mut self, _request: OutgoingRequest) -> wasmtime::Result<()> {
        anyhow::bail!("not implemented")
    }
    async fn incoming_request_method(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Method> {
        anyhow::bail!("not implemented")
    }
    async fn incoming_request_path(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<String> {
        anyhow::bail!("not implemented")
    }
    async fn incoming_request_scheme(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Option<Scheme>> {
        anyhow::bail!("not implemented")
    }
    async fn incoming_request_authority(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<String> {
        anyhow::bail!("not implemented")
    }
    async fn incoming_request_headers(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Headers> {
        anyhow::bail!("not implemented")
    }
    async fn incoming_request_consume(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Result<IncomingStream, ()>> {
        anyhow::bail!("not implemented")
    }
    async fn incoming_request_query(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<String> {
        anyhow::bail!("not implemented")
    }
    async fn new_outgoing_request(
        &mut self,
        _method: Method,
        _path: String,
        _query: String,
        _scheme: Option<Scheme>,
        _authority: String,
        _headers: Headers,
    ) -> wasmtime::Result<OutgoingRequest> {
        anyhow::bail!("not implemented")
    }
    async fn outgoing_request_write(
        &mut self,
        _request: OutgoingRequest,
    ) -> wasmtime::Result<Result<OutgoingStream, ()>> {
        anyhow::bail!("not implemented")
    }
    async fn drop_response_outparam(
        &mut self,
        _response: ResponseOutparam,
    ) -> wasmtime::Result<()> {
        anyhow::bail!("not implemented")
    }
    async fn set_response_outparam(
        &mut self,
        _response: Result<OutgoingResponse, Error>,
    ) -> wasmtime::Result<Result<(), ()>> {
        anyhow::bail!("not implemented")
    }
    async fn drop_incoming_response(
        &mut self,
        _response: IncomingResponse,
    ) -> wasmtime::Result<()> {
        anyhow::bail!("not implemented")
    }
    async fn drop_outgoing_response(
        &mut self,
        _response: OutgoingResponse,
    ) -> wasmtime::Result<()> {
        anyhow::bail!("not implemented")
    }
    async fn incoming_response_status(
        &mut self,
        _response: IncomingResponse,
    ) -> wasmtime::Result<StatusCode> {
        anyhow::bail!("not implemented")
    }
    async fn incoming_response_headers(
        &mut self,
        _response: IncomingResponse,
    ) -> wasmtime::Result<Headers> {
        anyhow::bail!("not implemented")
    }
    async fn incoming_response_consume(
        &mut self,
        _response: IncomingResponse,
    ) -> wasmtime::Result<Result<IncomingStream, ()>> {
        anyhow::bail!("not implemented")
    }
    async fn new_outgoing_response(
        &mut self,
        _status_code: StatusCode,
        _headers: Headers,
    ) -> wasmtime::Result<OutgoingResponse> {
        anyhow::bail!("not implemented")
    }
    async fn outgoing_response_write(
        &mut self,
        _response: OutgoingResponse,
    ) -> wasmtime::Result<Result<OutgoingStream, ()>> {
        anyhow::bail!("not implemented")
    }
    async fn drop_future_incoming_response(
        &mut self,
        _f: FutureIncomingResponse,
    ) -> wasmtime::Result<()> {
        anyhow::bail!("not implemented")
    }
    async fn future_incoming_response_get(
        &mut self,
        _f: FutureIncomingResponse,
    ) -> wasmtime::Result<Option<Result<IncomingResponse, Error>>> {
        anyhow::bail!("not implemented")
    }
    async fn listen_to_future_incoming_response(
        &mut self,
        _f: FutureIncomingResponse,
    ) -> wasmtime::Result<Pollable> {
        anyhow::bail!("not implemented")
    }
}

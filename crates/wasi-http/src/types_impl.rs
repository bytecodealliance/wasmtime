use crate::http_impl::WasiHttpViewExt;
use crate::types::{ActiveFields, ActiveRequest, HttpRequest, TableHttpExt};
use crate::wasi::http::types::{
    Error, Fields, FutureIncomingResponse, Headers, IncomingRequest, IncomingResponse,
    IncomingStream, Method, OutgoingRequest, OutgoingResponse, OutgoingStream, ResponseOutparam,
    Scheme, StatusCode, Trailers,
};
use crate::WasiHttpView;
use anyhow::{anyhow, bail, Context};
use bytes::Bytes;
use wasmtime_wasi::preview2::{bindings::poll::poll::Pollable, HostPollable, TablePollableExt};

#[async_trait::async_trait]
impl<T: WasiHttpView + WasiHttpViewExt> crate::wasi::http::types::Host for T {
    async fn drop_fields(&mut self, fields: Fields) -> wasmtime::Result<()> {
        self.table_mut()
            .delete_fields(fields)
            .context("[drop_fields] deleting fields")?;
        Ok(())
    }
    async fn new_fields(&mut self, entries: Vec<(String, String)>) -> wasmtime::Result<Fields> {
        let mut map = ActiveFields::new();
        for (key, value) in entries {
            map.insert(key, vec![value.clone().into_bytes()]);
        }

        let id = self
            .table_mut()
            .push_fields(Box::new(map))
            .context("[new_fields] pushing fields")?;
        Ok(id)
    }
    async fn fields_get(&mut self, fields: Fields, name: String) -> wasmtime::Result<Vec<Vec<u8>>> {
        let res = self
            .table_mut()
            .get_fields(fields)
            .context("[fields_get] getting fields")?
            .get(&name)
            .ok_or_else(|| anyhow!("key not found: {name}"))?
            .clone();
        Ok(res)
    }
    async fn fields_set(
        &mut self,
        fields: Fields,
        name: String,
        value: Vec<Vec<u8>>,
    ) -> wasmtime::Result<()> {
        match self.table_mut().get_fields_mut(fields) {
            Ok(m) => {
                m.insert(name, value.clone());
                Ok(())
            }
            Err(_) => bail!("fields not found"),
        }
    }
    async fn fields_delete(&mut self, fields: Fields, name: String) -> wasmtime::Result<()> {
        match self.table_mut().get_fields_mut(fields) {
            Ok(m) => m.remove(&name),
            Err(_) => None,
        };
        Ok(())
    }
    async fn fields_append(
        &mut self,
        fields: Fields,
        name: String,
        value: Vec<u8>,
    ) -> wasmtime::Result<()> {
        let m = self
            .table_mut()
            .get_fields_mut(fields)
            .context("[fields_append] getting mutable fields")?;
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
    async fn fields_entries(&mut self, fields: Fields) -> wasmtime::Result<Vec<(String, Vec<u8>)>> {
        let field_map = match self.table().get_fields(fields) {
            Ok(m) => m.iter(),
            Err(_) => bail!("fields not found."),
        };
        let mut result = Vec::new();
        for (name, value) in field_map {
            result.push((name.clone(), value[0].clone()));
        }
        Ok(result)
    }
    async fn fields_clone(&mut self, fields: Fields) -> wasmtime::Result<Fields> {
        let table = self.table_mut();
        let m = table
            .get_fields(fields)
            .context("[fields_clone] getting fields")?;
        let id = table
            .push_fields(Box::new(m.clone()))
            .context("[fields_clone] pushing fields")?;
        Ok(id)
    }
    async fn finish_incoming_stream(
        &mut self,
        stream_id: IncomingStream,
    ) -> wasmtime::Result<Option<Trailers>> {
        for (_, stream) in self.http_ctx().streams.iter() {
            if stream_id == stream.incoming() {
                let response = self
                    .table()
                    .get_response(stream.parent_id())
                    .context("[finish_incoming_stream] get trailers from response")?;
                return Ok(response.trailers());
            }
        }
        bail!("unknown stream!")
    }
    async fn finish_outgoing_stream(
        &mut self,
        _s: OutgoingStream,
        _trailers: Option<Trailers>,
    ) -> wasmtime::Result<()> {
        bail!("unimplemented: finish_outgoing_stream")
    }
    async fn drop_incoming_request(&mut self, _request: IncomingRequest) -> wasmtime::Result<()> {
        bail!("unimplemented: drop_incoming_request")
    }
    async fn drop_outgoing_request(&mut self, request: OutgoingRequest) -> wasmtime::Result<()> {
        let r = self
            .table_mut()
            .get_request(request)
            .context("[drop_outgoing_request] getting fields")?;

        // Cleanup dependent resources
        let body = r.body();
        let headers = r.headers();
        if let Some(b) = body {
            self.table_mut().delete_stream(b).ok();
        }
        if let Some(h) = headers {
            self.table_mut().delete_fields(h).ok();
        }

        self.table_mut()
            .delete_request(request)
            .context("[drop_outgoing_request] deleting request")?;

        Ok(())
    }
    async fn incoming_request_method(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Method> {
        bail!("unimplemented: incoming_request_method")
    }
    async fn incoming_request_path_with_query(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Option<String>> {
        bail!("unimplemented: incoming_request_path")
    }
    async fn incoming_request_scheme(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Option<Scheme>> {
        bail!("unimplemented: incoming_request_scheme")
    }
    async fn incoming_request_authority(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Option<String>> {
        bail!("unimplemented: incoming_request_authority")
    }
    async fn incoming_request_headers(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Headers> {
        bail!("unimplemented: incoming_request_headers")
    }
    async fn incoming_request_consume(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Result<IncomingStream, ()>> {
        bail!("unimplemented: incoming_request_consume")
    }
    async fn new_outgoing_request(
        &mut self,
        method: Method,
        path_with_query: Option<String>,
        scheme: Option<Scheme>,
        authority: Option<String>,
        headers: Headers,
    ) -> wasmtime::Result<OutgoingRequest> {
        let mut req = ActiveRequest::new();
        req.path_with_query = path_with_query.unwrap_or("".to_string());
        req.authority = authority.unwrap_or("".to_string());
        req.method = method;
        req.headers = Some(headers);
        req.scheme = scheme;
        let id = self
            .table_mut()
            .push_request(Box::new(req))
            .context("[new_outgoing_request] pushing request")?;
        Ok(id)
    }
    async fn outgoing_request_write(
        &mut self,
        request: OutgoingRequest,
    ) -> wasmtime::Result<Result<OutgoingStream, ()>> {
        let req = self
            .table()
            .get_request(request)
            .context("[outgoing_request_write] getting request")?;
        let stream_id = if let Some(stream_id) = req.body() {
            stream_id
        } else {
            let (new, stream) = self
                .table_mut()
                .push_stream(Bytes::new(), request)
                .await
                .expect("[outgoing_request_write] valid output stream");
            self.http_ctx_mut().streams.insert(new, stream);
            let req = self
                .table_mut()
                .get_request_mut(request)
                .expect("[outgoing_request_write] request to be found");
            req.set_body(new);
            new
        };
        let stream = self
            .table()
            .get_stream(stream_id)
            .context("[outgoing_request_write] getting stream")?;
        Ok(Ok(stream.outgoing()))
    }
    async fn drop_response_outparam(
        &mut self,
        _response: ResponseOutparam,
    ) -> wasmtime::Result<()> {
        bail!("unimplemented: drop_response_outparam")
    }
    async fn set_response_outparam(
        &mut self,
        _outparam: ResponseOutparam,
        _response: Result<OutgoingResponse, Error>,
    ) -> wasmtime::Result<Result<(), ()>> {
        bail!("unimplemented: set_response_outparam")
    }
    async fn drop_incoming_response(&mut self, response: IncomingResponse) -> wasmtime::Result<()> {
        let r = self
            .table()
            .get_response(response)
            .context("[drop_incoming_response] getting response")?;

        // Cleanup dependent resources
        let body = r.body();
        let headers = r.headers();
        if let Some(id) = body {
            let stream = self
                .table()
                .get_stream(id)
                .context("[drop_incoming_response] getting stream")?;
            let incoming_id = stream.incoming();
            if let Some(trailers) = self.finish_incoming_stream(incoming_id).await? {
                self.table_mut()
                    .delete_fields(trailers)
                    .context("[drop_incoming_response] deleting trailers")
                    .unwrap_or_else(|_| ());
            }
            self.table_mut().delete_stream(id).ok();
        }
        if let Some(h) = headers {
            self.table_mut().delete_fields(h).ok();
        }

        self.table_mut()
            .delete_response(response)
            .context("[drop_incoming_response] deleting response")?;
        Ok(())
    }
    async fn drop_outgoing_response(
        &mut self,
        _response: OutgoingResponse,
    ) -> wasmtime::Result<()> {
        bail!("unimplemented: drop_outgoing_response")
    }
    async fn incoming_response_status(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<StatusCode> {
        let r = self
            .table()
            .get_response(response)
            .context("[incoming_response_status] getting response")?;
        Ok(r.status())
    }
    async fn incoming_response_headers(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<Headers> {
        let r = self
            .table()
            .get_response(response)
            .context("[incoming_response_headers] getting response")?;
        Ok(r.headers().unwrap_or(0 as Headers))
    }
    async fn incoming_response_consume(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<Result<IncomingStream, ()>> {
        let table = self.table_mut();
        let r = table
            .get_response(response)
            .context("[incoming_response_consume] getting response")?;
        Ok(Ok(r
            .body()
            .map(|id| {
                table
                    .get_stream(id)
                    .map(|stream| stream.incoming())
                    .expect("[incoming_response_consume] response body stream")
            })
            .unwrap_or(0 as IncomingStream)))
    }
    async fn new_outgoing_response(
        &mut self,
        _status_code: StatusCode,
        _headers: Headers,
    ) -> wasmtime::Result<OutgoingResponse> {
        bail!("unimplemented: new_outgoing_response")
    }
    async fn outgoing_response_write(
        &mut self,
        _response: OutgoingResponse,
    ) -> wasmtime::Result<Result<OutgoingStream, ()>> {
        bail!("unimplemented: outgoing_response_write")
    }
    async fn drop_future_incoming_response(
        &mut self,
        future: FutureIncomingResponse,
    ) -> wasmtime::Result<()> {
        self.table_mut()
            .delete_future(future)
            .context("[drop_future_incoming_response] deleting future")?;
        Ok(())
    }
    async fn future_incoming_response_get(
        &mut self,
        future: FutureIncomingResponse,
    ) -> wasmtime::Result<Option<Result<IncomingResponse, Error>>> {
        let f = self
            .table()
            .get_future(future)
            .context("[future_incoming_response_get] getting future")?;
        Ok(match f.pollable_id() {
            Some(_) => {
                let result = match f.response_id() {
                    Some(id) => Ok(id),
                    None => {
                        let response = self.handle_async(f.request_id(), f.options()).await;
                        match response {
                            Ok(id) => {
                                let future_mut = self.table_mut().get_future_mut(future)?;
                                future_mut.set_response_id(id);
                            }
                            _ => {}
                        }
                        response
                    }
                };
                Some(result)
            }
            None => None,
        })
    }
    async fn listen_to_future_incoming_response(
        &mut self,
        future: FutureIncomingResponse,
    ) -> wasmtime::Result<Pollable> {
        let f = self
            .table()
            .get_future(future)
            .context("[listen_to_future_incoming_response] getting future")?;
        Ok(match f.pollable_id() {
            Some(pollable_id) => pollable_id,
            None => {
                let pollable =
                    HostPollable::Closure(Box::new(|| Box::pin(futures::future::ready(Ok(())))));
                let pollable_id = self
                    .table_mut()
                    .push_host_pollable(pollable)
                    .context("[listen_to_future_incoming_response] pushing host pollable")?;
                let f = self
                    .table_mut()
                    .get_future_mut(future)
                    .context("[listen_to_future_incoming_response] getting future")?;
                f.set_pollable_id(pollable_id);
                pollable_id
            }
        })
    }
}

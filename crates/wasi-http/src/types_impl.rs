use crate::http_impl::WasiHttpViewExt;
use crate::r#struct::{ActiveFields, ActiveRequest, HttpRequest, TableHttpExt};
use crate::wasi::http::types::{
    Error, Fields, FutureIncomingResponse, Headers, IncomingRequest, IncomingResponse,
    IncomingStream, Method, OutgoingRequest, OutgoingResponse, OutgoingStream, ResponseOutparam,
    Scheme, StatusCode, Trailers,
};
use crate::wasi::poll::poll::Pollable;
use crate::WasiHttpView;
use anyhow::{anyhow, bail, Context};
use bytes::Bytes;
use wasmtime_wasi::preview2::TableError;

fn convert(error: TableError) -> anyhow::Error {
    // if let Some(errno) = error.downcast_ref() {
    //     Error::UnexpectedError(errno.to_string())
    // } else {
    error.into()
    // }
}

impl<T: WasiHttpView + WasiHttpViewExt> crate::wasi::http::types::Host for T {
    fn drop_fields(&mut self, fields: Fields) -> wasmtime::Result<()> {
        self.table_mut().delete_fields(fields).map_err(convert)?;
        Ok(())
    }
    fn new_fields(&mut self, entries: Vec<(String, String)>) -> wasmtime::Result<Fields> {
        let mut map = ActiveFields::new();
        for (key, value) in entries {
            map.insert(key, vec![value.clone().into_bytes()]);
        }

        let id = self
            .table_mut()
            .push_fields(Box::new(map))
            .map_err(convert)?;
        Ok(id)
    }
    fn fields_get(&mut self, fields: Fields, name: String) -> wasmtime::Result<Vec<Vec<u8>>> {
        let res = self
            .table_mut()
            .get_fields(fields)
            .map_err(convert)?
            .get(&name)
            .ok_or_else(|| anyhow!("key not found: {name}"))?
            .clone();
        Ok(res)
    }
    fn fields_set(
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
    fn fields_delete(&mut self, fields: Fields, name: String) -> wasmtime::Result<()> {
        match self.table_mut().get_fields_mut(fields) {
            Ok(m) => m.remove(&name),
            Err(_) => None,
        };
        Ok(())
    }
    fn fields_append(
        &mut self,
        fields: Fields,
        name: String,
        value: Vec<u8>,
    ) -> wasmtime::Result<()> {
        let m = self.table_mut().get_fields_mut(fields).map_err(convert)?;
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
    fn fields_entries(&mut self, fields: Fields) -> wasmtime::Result<Vec<(String, Vec<u8>)>> {
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
    fn fields_clone(&mut self, fields: Fields) -> wasmtime::Result<Fields> {
        let table = self.table_mut();
        let m = table.get_fields(fields).map_err(convert)?;
        let id = table.push_fields(Box::new(m.clone())).map_err(convert)?;
        Ok(id)
    }
    fn finish_incoming_stream(
        &mut self,
        stream_id: IncomingStream,
    ) -> wasmtime::Result<Option<Trailers>> {
        for (_, stream) in self.http_ctx().streams.iter() {
            if stream_id == stream.incoming() {
                let response = self
                    .table()
                    .get_response(stream.parent_id())
                    .context("get trailers from response")?;
                return Ok(response.trailers());
            }
        }
        bail!("unknown stream!")
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
        let r = self.table_mut().get_request(request).map_err(convert)?;

        // Cleanup dependent resources
        let body = r.body();
        let headers = r.headers();
        if let Some(b) = body {
            self.table_mut().delete_stream(b).ok();
        }
        if let Some(h) = headers {
            self.table_mut().delete_fields(h).ok();
        }

        self.table_mut().delete_request(request).map_err(convert)?;

        Ok(())
    }
    fn incoming_request_method(&mut self, _request: IncomingRequest) -> wasmtime::Result<Method> {
        bail!("unimplemented: incoming_request_method")
    }
    fn incoming_request_path_with_query(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Option<String>> {
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
    ) -> wasmtime::Result<Option<String>> {
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
    fn new_outgoing_request(
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
            .map_err(convert)?;
        Ok(id)
    }
    fn outgoing_request_write(
        &mut self,
        request: OutgoingRequest,
    ) -> wasmtime::Result<Result<OutgoingStream, ()>> {
        let req = self.table().get_request(request).map_err(convert)?;
        let stream_id = req.body().unwrap_or_else(|| {
            let (new, stream) = self
                .table_mut()
                .push_stream(Bytes::new(), request)
                .expect("valid output stream");
            self.http_ctx_mut().streams.insert(new, stream);
            let req = self
                .table_mut()
                .get_request_mut(request)
                .expect("request to be found");
            req.set_body(new);
            new
        });
        let stream = self.table().get_stream(stream_id)?;
        Ok(Ok(stream.outgoing()))
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
        let r = self.table().get_response(response).map_err(convert)?;

        // Cleanup dependent resources
        let body = r.body();
        let headers = r.headers();
        if let Some(id) = body {
            let stream = self.table().get_stream(id)?;
            let incoming_id = stream.incoming();
            if let Some(trailers) = self.finish_incoming_stream(incoming_id)? {
                self.table_mut()
                    .delete_fields(trailers)
                    .context("[drop_incoming_response] deleting trailers")
                    .unwrap_or_else(|_| ());
            }
            self.table_mut()
                .delete_stream(id)
                .context("[drop_incoming_response] deleting input stream")
                .ok();
        }
        if let Some(h) = headers {
            self.table_mut()
                .delete_fields(h)
                .context("[drop_incoming_response] deleting fields")
                .ok();
        }

        self.table_mut()
            .delete_response(response)
            .context("[drop_incoming_response] deleting response")?;
        Ok(())
    }
    fn drop_outgoing_response(&mut self, _response: OutgoingResponse) -> wasmtime::Result<()> {
        bail!("unimplemented: drop_outgoing_response")
    }
    fn incoming_response_status(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<StatusCode> {
        let r = self.table().get_response(response).map_err(convert)?;
        Ok(r.status())
    }
    fn incoming_response_headers(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<Headers> {
        let r = self.table().get_response(response).map_err(convert)?;
        Ok(r.headers().unwrap_or(0 as Headers))
    }
    fn incoming_response_consume(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<Result<IncomingStream, ()>> {
        let table = self.table_mut();
        let r = table.get_response(response).map_err(convert)?;
        Ok(Ok(r
            .body()
            .map(|id| {
                table
                    .get_stream(id)
                    .map(|stream| stream.incoming())
                    .expect("response body stream")
            })
            .unwrap_or(0 as IncomingStream)))
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
        future: FutureIncomingResponse,
    ) -> wasmtime::Result<()> {
        self.table_mut().delete_future(future)?;
        Ok(())
    }
    fn future_incoming_response_get(
        &mut self,
        future: FutureIncomingResponse,
    ) -> wasmtime::Result<Option<Result<IncomingResponse, Error>>> {
        let f = self
            .table()
            .get_future(future)
            .context("[future_incoming_response_get] getting future")?;

        let response = futures::executor::block_on(self.handle_async(f.request_id, f.options))
            .map_err(|e| Error::UnexpectedError(e.to_string()));
        Ok(Some(response))
    }
    fn listen_to_future_incoming_response(
        &mut self,
        _f: FutureIncomingResponse,
    ) -> wasmtime::Result<Pollable> {
        bail!("unimplemented: listen_to_future_incoming_response")
    }
}

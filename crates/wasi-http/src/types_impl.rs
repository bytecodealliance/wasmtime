use std::any::Any;

use crate::bindings::http::types::{
    Error, Fields, FutureIncomingResponse, FutureTrailers, Headers, IncomingBody, IncomingRequest,
    IncomingResponse, Method, OutgoingBody, OutgoingRequest, OutgoingResponse, ResponseOutparam,
    Scheme, StatusCode, Trailers,
};
use crate::body::HostFutureTrailers;
use crate::types::FieldMap;
use crate::WasiHttpView;
use crate::{
    body::HostIncomingBody,
    types::{
        HostFields, HostFutureIncomingResponse, HostIncomingResponse, HostOutgoingBody,
        HostOutgoingRequest, OutgoingBodyRepr, TableHttpExt,
    },
};
use anyhow::{anyhow, Context};
use wasmtime_wasi::preview2::{
    bindings::io::streams::{InputStream, OutputStream},
    bindings::poll::poll::Pollable,
    HostPollable, PollableFuture, TablePollableExt, TableStreamExt,
};

#[async_trait::async_trait]
impl<T: WasiHttpView> crate::bindings::http::types::Host for T {
    async fn drop_fields(&mut self, fields: Fields) -> wasmtime::Result<()> {
        self.table()
            .delete_fields(fields)
            .context("[drop_fields] deleting fields")?;
        Ok(())
    }
    async fn new_fields(&mut self, entries: Vec<(String, Vec<u8>)>) -> wasmtime::Result<Fields> {
        use std::collections::{hash_map::Entry, HashMap};

        let mut map: HashMap<String, Vec<Vec<u8>>> = HashMap::new();

        for (key, value) in entries {
            match map.entry(key) {
                Entry::Occupied(mut entry) => entry.get_mut().push(value),
                Entry::Vacant(entry) => {
                    entry.insert(vec![value]);
                }
            }
        }

        let id = self
            .table()
            .push_fields(HostFields::Owned {
                fields: FieldMap(map),
            })
            .context("[new_fields] pushing fields")?;
        Ok(id)
    }
    async fn fields_get(&mut self, fields: Fields, name: String) -> wasmtime::Result<Vec<Vec<u8>>> {
        let res = self
            .table()
            .get_fields(fields)
            .context("[fields_get] getting fields")?
            .0
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
        let m = self.table().get_fields(fields)?;
        m.0.insert(name, value.clone());
        Ok(())
    }
    async fn fields_delete(&mut self, fields: Fields, name: String) -> wasmtime::Result<()> {
        let m = self.table().get_fields(fields)?;
        m.0.remove(&name);
        Ok(())
    }
    async fn fields_append(
        &mut self,
        fields: Fields,
        name: String,
        value: Vec<u8>,
    ) -> wasmtime::Result<()> {
        let m = self
            .table()
            .get_fields(fields)
            .context("[fields_append] getting mutable fields")?;
        match m.0.get_mut(&name) {
            Some(v) => v.push(value),
            None => {
                let mut vec = std::vec::Vec::new();
                vec.push(value);
                m.0.insert(name, vec);
            }
        };
        Ok(())
    }
    async fn fields_entries(&mut self, fields: Fields) -> wasmtime::Result<Vec<(String, Vec<u8>)>> {
        let result = self
            .table()
            .get_fields(fields)?
            .0
            .iter()
            .map(|(name, value)| (name.clone(), value[0].clone()))
            .collect();
        Ok(result)
    }
    async fn fields_clone(&mut self, fields: Fields) -> wasmtime::Result<Fields> {
        let fields = self
            .table()
            .get_fields(fields)
            .context("[fields_clone] getting fields")?
            .clone();
        let id = self
            .table()
            .push_fields(HostFields::Owned { fields })
            .context("[fields_clone] pushing fields")?;
        Ok(id)
    }
    async fn drop_incoming_request(&mut self, _request: IncomingRequest) -> wasmtime::Result<()> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    async fn drop_outgoing_request(&mut self, request: OutgoingRequest) -> wasmtime::Result<()> {
        self.table().delete_outgoing_request(request)?;
        Ok(())
    }
    async fn incoming_request_method(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Method> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    async fn incoming_request_path_with_query(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Option<String>> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    async fn incoming_request_scheme(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Option<Scheme>> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    async fn incoming_request_authority(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Option<String>> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    async fn incoming_request_headers(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Headers> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    async fn incoming_request_consume(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Result<InputStream, ()>> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    async fn new_outgoing_request(
        &mut self,
        method: Method,
        path_with_query: Option<String>,
        scheme: Option<Scheme>,
        authority: Option<String>,
        headers: Headers,
    ) -> wasmtime::Result<OutgoingRequest> {
        let headers = self.table().get_fields(headers)?.clone();

        let req = HostOutgoingRequest {
            path_with_query: path_with_query.unwrap_or("".to_string()),
            authority: authority.unwrap_or("".to_string()),
            method,
            headers,
            scheme,
            body: None,
        };
        let id = self
            .table()
            .push_outgoing_response(req)
            .context("[new_outgoing_request] pushing request")?;
        Ok(id)
    }
    async fn outgoing_request_write(
        &mut self,
        request: OutgoingRequest,
    ) -> wasmtime::Result<Result<OutgoingBody, ()>> {
        let req = self
            .table()
            .get_outgoing_request_mut(request)
            .context("[outgoing_request_write] getting request")?;

        if req.body.is_some() {
            return Ok(Err(()));
        }
        req.body = Some(OutgoingBodyRepr::new());

        fn get_body(entry: &mut dyn Any) -> &mut OutgoingBodyRepr {
            let req = entry.downcast_mut::<HostOutgoingRequest>().unwrap();
            req.body.as_mut().unwrap()
        }
        // The output stream will necessarily outlive the request, because we could be still
        // writing to the stream after `outgoing-handler.handle` is called.
        let outgoing_body = self.table().push_outgoing_body(HostOutgoingBody {
            parent: request,
            get_body,
        })?;

        Ok(Ok(outgoing_body))
    }
    async fn drop_response_outparam(
        &mut self,
        _response: ResponseOutparam,
    ) -> wasmtime::Result<()> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    async fn set_response_outparam(
        &mut self,
        _outparam: ResponseOutparam,
        _response: Result<OutgoingResponse, Error>,
    ) -> wasmtime::Result<()> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    async fn drop_incoming_response(&mut self, response: IncomingResponse) -> wasmtime::Result<()> {
        self.table()
            .delete_incoming_response(response)
            .context("[drop_incoming_response] deleting response")?;
        Ok(())
    }
    async fn drop_outgoing_response(
        &mut self,
        _response: OutgoingResponse,
    ) -> wasmtime::Result<()> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    async fn incoming_response_status(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<StatusCode> {
        let r = self
            .table()
            .get_incoming_response(response)
            .context("[incoming_response_status] getting response")?;
        Ok(r.status)
    }
    async fn incoming_response_headers(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<Headers> {
        let _ = self
            .table()
            .get_incoming_response_mut(response)
            .context("[incoming_response_headers] getting response")?;

        fn get_fields(elem: &mut dyn Any) -> &mut FieldMap {
            &mut elem.downcast_mut::<HostIncomingResponse>().unwrap().headers
        }

        let id = self.table().push_fields(HostFields::Ref {
            parent: response,
            get_fields,
        })?;
        Ok(id)
    }
    async fn incoming_response_consume(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<Result<IncomingBody, ()>> {
        let table = self.table();
        let r = table
            .get_incoming_response_mut(response)
            .context("[incoming_response_consume] getting response")?;

        match r.body.take() {
            Some(body) => {
                let id = self
                    .table()
                    .push_incoming_body(HostIncomingBody::new(body))?;
                Ok(Ok(id))
            }

            None => Ok(Err(())),
        }
    }
    async fn drop_future_trailers(&mut self, id: FutureTrailers) -> wasmtime::Result<()> {
        self.table()
            .delete_future_trailers(id)
            .context("[drop future-trailers] deleting future-trailers")?;
        Ok(())
    }

    async fn future_trailers_subscribe(
        &mut self,
        index: FutureTrailers,
    ) -> wasmtime::Result<Pollable> {
        // Eagerly force errors about the validity of the index.
        let _ = self.table().get_future_trailers(index)?;

        fn make_future(elem: &mut dyn Any) -> PollableFuture {
            Box::pin(elem.downcast_mut::<HostFutureTrailers>().unwrap().ready())
        }

        let id = self
            .table()
            .push_host_pollable(HostPollable::TableEntry { index, make_future })?;

        Ok(id)
    }

    async fn future_trailers_get(
        &mut self,
        id: FutureTrailers,
    ) -> wasmtime::Result<Option<Result<Trailers, Error>>> {
        let trailers = self.table().get_future_trailers(id)?;

        if trailers.received.is_none() {
            return Ok(None);
        }

        let res = trailers.received.as_mut().unwrap();
        if let Err(e) = res {
            return Ok(Some(Err(e.clone())));
        }

        fn get_fields(elem: &mut dyn Any) -> &mut FieldMap {
            let trailers = elem.downcast_mut::<HostFutureTrailers>().unwrap();
            trailers.received.as_mut().unwrap().as_mut().unwrap()
        }

        let hdrs = self.table().push_fields(HostFields::Ref {
            parent: id,
            get_fields,
        })?;

        Ok(Some(Ok(hdrs)))
    }

    async fn new_outgoing_response(
        &mut self,
        _status_code: StatusCode,
        _headers: Headers,
    ) -> wasmtime::Result<OutgoingResponse> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    async fn outgoing_response_write(
        &mut self,
        _response: OutgoingResponse,
    ) -> wasmtime::Result<Result<OutputStream, ()>> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    async fn drop_future_incoming_response(
        &mut self,
        _future: FutureIncomingResponse,
    ) -> wasmtime::Result<()> {
        todo!()
    }
    async fn future_incoming_response_get(
        &mut self,
        id: FutureIncomingResponse,
    ) -> wasmtime::Result<Option<Result<Result<IncomingResponse, Error>, ()>>> {
        if !self.table().get_future_incoming_response(id)?.is_ready() {
            return Ok(None);
        }

        let resp = match self
            .table()
            .delete_future_incoming_response(id)?
            .unwrap_ready()
        {
            Ok(resp) => resp,
            Err(e) => {
                // Trapping if it's not possible to downcast to an wasi-http error
                let e = e.downcast::<Error>()?;
                return Ok(Some(Ok(Err(e))));
            }
        };

        let (parts, body) = resp.resp.into_parts();

        let resp = self.table().push_incoming_response(HostIncomingResponse {
            status: parts.status.as_u16(),
            headers: FieldMap::from(parts.headers),
            body: Some(body),
            worker: resp.worker,
        })?;

        Ok(Some(Ok(Ok(resp))))
    }
    async fn listen_to_future_incoming_response(
        &mut self,
        id: FutureIncomingResponse,
    ) -> wasmtime::Result<Pollable> {
        let _ = self.table().get_future_incoming_response(id)?;

        fn make_future<'a>(elem: &'a mut dyn Any) -> PollableFuture<'a> {
            Box::pin(
                elem.downcast_mut::<HostFutureIncomingResponse>()
                    .expect("parent resource is HostFutureIncomingResponse"),
            )
        }

        let pollable = self.table().push_host_pollable(HostPollable::TableEntry {
            index: id,
            make_future,
        })?;

        Ok(pollable)
    }

    async fn incoming_body_stream(
        &mut self,
        id: IncomingBody,
    ) -> wasmtime::Result<Result<InputStream, ()>> {
        let body = self.table().get_incoming_body(id)?;

        if let Some(stream) = body.stream.take() {
            let stream = self.table().push_input_stream_child(Box::new(stream), id)?;
            return Ok(Ok(stream));
        }

        Ok(Err(()))
    }

    async fn incoming_body_finish(&mut self, id: IncomingBody) -> wasmtime::Result<FutureTrailers> {
        let body = self.table().delete_incoming_body(id)?;
        let trailers = self
            .table()
            .push_future_trailers(body.into_future_trailers())?;
        Ok(trailers)
    }

    async fn drop_incoming_body(&mut self, id: IncomingBody) -> wasmtime::Result<()> {
        let _ = self.table().delete_incoming_body(id)?;
        Ok(())
    }

    async fn outgoing_body_write(
        &mut self,
        id: OutgoingBody,
    ) -> wasmtime::Result<Result<OutputStream, ()>> {
        let body = self.table().get_outgoing_body(id)?;
        if let Some(stream) = body.body_output_stream.take() {
            let id = self.table().push_output_stream_child(stream, id)?;
            Ok(Ok(id))
        } else {
            Ok(Err(()))
        }
    }

    async fn outgoing_body_write_trailers(
        &mut self,
        id: OutgoingBody,
        ts: Trailers,
    ) -> wasmtime::Result<()> {
        let HostOutgoingBody { parent, get_body } = self.table().delete_outgoing_body(id)?;
        let trailers = self.table().get_fields(ts)?.clone();
        let body = get_body(self.table().get_any_mut(parent)?);

        match body
            .trailers_sender
            .take()
            // Should be unreachable - this is the only place we take the trailers sender,
            // at the end of the HostOutgoingBody's lifetime
            .ok_or_else(|| anyhow!("trailers_sender missing"))?
            .send(trailers.into())
        {
            Ok(()) => {}
            Err(_) => {} // Ignoring failure: receiver died sending body, but we can't report that
                         // here.
        }

        Ok(())
    }

    async fn drop_outgoing_body(&mut self, id: OutgoingBody) -> wasmtime::Result<()> {
        let _ = self.table().delete_outgoing_body(id)?;
        Ok(())
    }
}

use crate::bindings::http::types::{
    Error, Fields, FutureIncomingResponse, FutureTrailers, Headers, IncomingBody, IncomingRequest,
    IncomingResponse, Method, OutgoingBody, OutgoingRequest, OutgoingResponse, ResponseOutparam,
    Scheme, StatusCode, Trailers,
};
use crate::body::{FinishMessage, HostFutureTrailers, HostFutureTrailersState};
use crate::types::{HostIncomingRequest, HostOutgoingResponse};
use crate::WasiHttpView;
use crate::{
    body::{HostIncomingBodyBuilder, HostOutgoingBody},
    types::{
        self, FieldMap, HostFields, HostFutureIncomingResponse, HostIncomingResponse,
        HostOutgoingRequest, TableHttpExt,
    },
};
use anyhow::Context;
use std::any::Any;
use wasmtime::component::Resource;
use wasmtime_wasi::preview2::{
    bindings::io::poll::Pollable,
    bindings::io::streams::{InputStream, OutputStream},
    HostPollable, PollableFuture, TablePollableExt,
};

impl<T: WasiHttpView> crate::bindings::http::types::Host for T {
    fn drop_fields(&mut self, fields: Fields) -> wasmtime::Result<()> {
        self.table()
            .delete_fields(fields)
            .context("[drop_fields] deleting fields")?;
        Ok(())
    }
    fn new_fields(&mut self, entries: Vec<(String, Vec<u8>)>) -> wasmtime::Result<Fields> {
        let mut map = hyper::HeaderMap::new();

        for (header, value) in entries {
            let header = hyper::header::HeaderName::from_bytes(header.as_bytes())?;
            let value = hyper::header::HeaderValue::from_bytes(&value)?;
            map.append(header, value);
        }

        let id = self
            .table()
            .push_fields(HostFields::Owned { fields: map })
            .context("[new_fields] pushing fields")?;
        Ok(id)
    }
    fn fields_get(&mut self, fields: Fields, name: String) -> wasmtime::Result<Vec<Vec<u8>>> {
        let res = self
            .table()
            .get_fields(fields)
            .context("[fields_get] getting fields")?
            .get_all(hyper::header::HeaderName::from_bytes(name.as_bytes())?)
            .into_iter()
            .map(|val| val.as_bytes().to_owned())
            .collect();
        Ok(res)
    }
    fn fields_set(
        &mut self,
        fields: Fields,
        name: String,
        values: Vec<Vec<u8>>,
    ) -> wasmtime::Result<()> {
        let m = self.table().get_fields(fields)?;

        let header = hyper::header::HeaderName::from_bytes(name.as_bytes())?;

        m.remove(&header);
        for value in values {
            let value = hyper::header::HeaderValue::from_bytes(&value)?;
            m.append(&header, value);
        }

        Ok(())
    }
    fn fields_delete(&mut self, fields: Fields, name: String) -> wasmtime::Result<()> {
        let m = self.table().get_fields(fields)?;
        let header = hyper::header::HeaderName::from_bytes(name.as_bytes())?;
        m.remove(header);
        Ok(())
    }
    fn fields_append(
        &mut self,
        fields: Fields,
        name: String,
        value: Vec<u8>,
    ) -> wasmtime::Result<()> {
        let m = self
            .table()
            .get_fields(fields)
            .context("[fields_append] getting mutable fields")?;
        let header = hyper::header::HeaderName::from_bytes(name.as_bytes())?;
        let value = hyper::header::HeaderValue::from_bytes(&value)?;
        m.append(header, value);
        Ok(())
    }
    fn fields_entries(&mut self, fields: Fields) -> wasmtime::Result<Vec<(String, Vec<u8>)>> {
        let fields = self.table().get_fields(fields)?;
        let result = fields
            .iter()
            .map(|(name, value)| (name.as_str().to_owned(), value.as_bytes().to_owned()))
            .collect();
        Ok(result)
    }
    fn fields_clone(&mut self, fields: Fields) -> wasmtime::Result<Fields> {
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
    fn drop_incoming_request(&mut self, id: IncomingRequest) -> wasmtime::Result<()> {
        let _ = types::IncomingRequestLens::from(id).delete(self.table())?;
        Ok(())
    }
    fn drop_outgoing_request(&mut self, request: OutgoingRequest) -> wasmtime::Result<()> {
        types::OutgoingRequestLens::from(request).delete(self.table())?;
        Ok(())
    }
    fn incoming_request_method(&mut self, request: IncomingRequest) -> wasmtime::Result<Method> {
        let method = types::IncomingRequestLens::from(request)
            .get(self.table())?
            .parts
            .method
            .as_ref();

        if method == hyper::Method::GET {
            Ok(Method::Get)
        } else if method == hyper::Method::HEAD {
            Ok(Method::Head)
        } else if method == hyper::Method::POST {
            Ok(Method::Post)
        } else if method == hyper::Method::PUT {
            Ok(Method::Put)
        } else if method == hyper::Method::DELETE {
            Ok(Method::Delete)
        } else if method == hyper::Method::CONNECT {
            Ok(Method::Connect)
        } else if method == hyper::Method::OPTIONS {
            Ok(Method::Options)
        } else if method == hyper::Method::TRACE {
            Ok(Method::Trace)
        } else if method == hyper::Method::PATCH {
            Ok(Method::Patch)
        } else {
            Ok(Method::Other(method.to_owned()))
        }
    }
    fn incoming_request_path_with_query(
        &mut self,
        id: IncomingRequest,
    ) -> wasmtime::Result<Option<String>> {
        let req = types::IncomingRequestLens::from(id).get(self.table())?;
        Ok(req
            .parts
            .uri
            .path_and_query()
            .map(|path_and_query| path_and_query.as_str().to_owned()))
    }
    fn incoming_request_scheme(&mut self, id: IncomingRequest) -> wasmtime::Result<Option<Scheme>> {
        let req = types::IncomingRequestLens::from(id).get(self.table())?;
        Ok(req.parts.uri.scheme().map(|scheme| {
            if scheme == &http::uri::Scheme::HTTP {
                return Scheme::Http;
            }

            if scheme == &http::uri::Scheme::HTTPS {
                return Scheme::Https;
            }

            Scheme::Other(req.parts.uri.scheme_str().unwrap().to_owned())
        }))
    }
    fn incoming_request_authority(
        &mut self,
        id: IncomingRequest,
    ) -> wasmtime::Result<Option<String>> {
        let req = types::IncomingRequestLens::from(id).get(self.table())?;
        Ok(req
            .parts
            .uri
            .authority()
            .map(|auth| auth.as_str().to_owned()))
    }
    fn incoming_request_headers(&mut self, id: IncomingRequest) -> wasmtime::Result<Headers> {
        let _ = types::IncomingRequestLens::from(id).get(self.table())?;

        fn get_fields(elem: &mut dyn Any) -> &mut FieldMap {
            &mut elem
                .downcast_mut::<HostIncomingRequest>()
                .unwrap()
                .parts
                .headers
        }

        let headers = self.table().push_fields(HostFields::Ref {
            parent: id,
            get_fields,
        })?;

        Ok(headers)
    }
    fn incoming_request_consume(
        &mut self,
        id: IncomingRequest,
    ) -> wasmtime::Result<Result<Resource<IncomingBody>, ()>> {
        let req = types::IncomingRequestLens::from(id).get_mut(self.table())?;
        match req.body.take() {
            Some(builder) => {
                let id = self.table().push_incoming_body(builder.build())?;
                Ok(Ok(id))
            }

            None => Ok(Err(())),
        }
    }
    fn new_outgoing_request(
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
        let id = types::OutgoingRequestLens::push(self.table(), req)
            .context("[new_outgoing_request] pushing request")?
            .into();
        Ok(id)
    }
    fn outgoing_request_write(
        &mut self,
        request: OutgoingRequest,
    ) -> wasmtime::Result<Result<OutgoingBody, ()>> {
        let req = types::OutgoingRequestLens::from(request)
            .get_mut(self.table())
            .context("[outgoing_request_write] getting request")?;

        if req.body.is_some() {
            return Ok(Err(()));
        }

        let (host_body, hyper_body) = HostOutgoingBody::new();

        req.body = Some(hyper_body);

        // The output stream will necessarily outlive the request, because we could be still
        // writing to the stream after `outgoing-handler.handle` is called.
        let outgoing_body = self.table().push_outgoing_body(host_body)?;

        Ok(Ok(outgoing_body))
    }
    fn drop_response_outparam(&mut self, id: ResponseOutparam) -> wasmtime::Result<()> {
        let _ = types::ResponseOutparamLens::from(id).delete(self.table())?;
        Ok(())
    }
    fn set_response_outparam(
        &mut self,
        id: ResponseOutparam,
        resp: Result<OutgoingResponse, Error>,
    ) -> wasmtime::Result<()> {
        let val = match resp {
            Ok(resp) => Ok(self.table().delete_outgoing_response(resp)?.try_into()?),
            Err(e) => Err(e),
        };

        types::ResponseOutparamLens::from(id)
            .delete(self.table())?
            .result
            .send(val)
            .map_err(|_| anyhow::anyhow!("failed to initialize response"))
    }
    fn drop_incoming_response(&mut self, response: IncomingResponse) -> wasmtime::Result<()> {
        self.table()
            .delete_incoming_response(response)
            .context("[drop_incoming_response] deleting response")?;
        Ok(())
    }
    fn drop_outgoing_response(&mut self, id: OutgoingResponse) -> wasmtime::Result<()> {
        types::OutgoingResponseLens::from(id).delete(self.table())?;
        Ok(())
    }
    fn incoming_response_status(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<StatusCode> {
        let r = self
            .table()
            .get_incoming_response(response)
            .context("[incoming_response_status] getting response")?;
        Ok(r.status)
    }
    fn incoming_response_headers(
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
    fn incoming_response_consume(
        &mut self,
        response: IncomingResponse,
    ) -> wasmtime::Result<Result<Resource<IncomingBody>, ()>> {
        let table = self.table();
        let r = table
            .get_incoming_response_mut(response)
            .context("[incoming_response_consume] getting response")?;

        match r.body.take() {
            Some(builder) => {
                let id = self.table().push_incoming_body(builder.build())?;
                Ok(Ok(id))
            }

            None => Ok(Err(())),
        }
    }
    fn drop_future_trailers(&mut self, id: FutureTrailers) -> wasmtime::Result<()> {
        self.table()
            .delete_future_trailers(id)
            .context("[drop future-trailers] deleting future-trailers")?;
        Ok(())
    }

    fn future_trailers_subscribe(
        &mut self,
        index: FutureTrailers,
    ) -> wasmtime::Result<Resource<Pollable>> {
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

    fn future_trailers_get(
        &mut self,
        id: FutureTrailers,
    ) -> wasmtime::Result<Option<Result<Trailers, Error>>> {
        let trailers = self.table().get_future_trailers(id)?;
        match &trailers.state {
            HostFutureTrailersState::Waiting(_) => return Ok(None),
            HostFutureTrailersState::Done(Err(e)) => return Ok(Some(Err(e.clone()))),
            HostFutureTrailersState::Done(Ok(_)) => {}
        }

        fn get_fields(elem: &mut dyn Any) -> &mut FieldMap {
            let trailers = elem.downcast_mut::<HostFutureTrailers>().unwrap();
            match &mut trailers.state {
                HostFutureTrailersState::Done(Ok(e)) => e,
                _ => unreachable!(),
            }
        }

        let hdrs = self.table().push_fields(HostFields::Ref {
            parent: id,
            get_fields,
        })?;

        Ok(Some(Ok(hdrs)))
    }

    fn new_outgoing_response(
        &mut self,
        status: StatusCode,
        headers: Headers,
    ) -> wasmtime::Result<OutgoingResponse> {
        let fields = self.table().get_fields(headers)?.clone();
        self.table().delete_fields(headers)?;

        let id = types::OutgoingResponseLens::push(
            self.table(),
            HostOutgoingResponse {
                status,
                headers: fields,
                body: None,
            },
        )?
        .into();

        Ok(id)
    }
    fn outgoing_response_write(
        &mut self,
        id: OutgoingResponse,
    ) -> wasmtime::Result<Result<OutgoingBody, ()>> {
        let resp = types::OutgoingResponseLens::from(id).get_mut(self.table())?;

        if resp.body.is_some() {
            return Ok(Err(()));
        }

        let (host, body) = HostOutgoingBody::new();

        resp.body.replace(body);

        let id = self.table().push_outgoing_body(host)?;

        Ok(Ok(id))
    }
    fn drop_future_incoming_response(
        &mut self,
        id: FutureIncomingResponse,
    ) -> wasmtime::Result<()> {
        let _ = self.table().delete_future_incoming_response(id)?;
        Ok(())
    }
    fn future_incoming_response_get(
        &mut self,
        id: FutureIncomingResponse,
    ) -> wasmtime::Result<Option<Result<Result<IncomingResponse, Error>, ()>>> {
        let resp = self.table().get_future_incoming_response_mut(id)?;

        match resp {
            HostFutureIncomingResponse::Pending(_) => return Ok(None),
            HostFutureIncomingResponse::Consumed => return Ok(Some(Err(()))),
            HostFutureIncomingResponse::Ready(_) => {}
        }

        let resp =
            match std::mem::replace(resp, HostFutureIncomingResponse::Consumed).unwrap_ready() {
                Err(e) => {
                    // Trapping if it's not possible to downcast to an wasi-http error
                    let e = e.downcast::<Error>()?;
                    return Ok(Some(Ok(Err(e))));
                }

                Ok(resp) => resp,
            };

        let (parts, body) = resp.resp.into_parts();

        let resp = self.table().push_incoming_response(HostIncomingResponse {
            status: parts.status.as_u16(),
            headers: FieldMap::from(parts.headers),
            body: Some(HostIncomingBodyBuilder {
                body,
                between_bytes_timeout: resp.between_bytes_timeout,
            }),
            worker: resp.worker,
        })?;

        Ok(Some(Ok(Ok(resp))))
    }
    fn listen_to_future_incoming_response(
        &mut self,
        id: FutureIncomingResponse,
    ) -> wasmtime::Result<Resource<Pollable>> {
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

    fn outgoing_body_write(
        &mut self,
        id: OutgoingBody,
    ) -> wasmtime::Result<Result<Resource<OutputStream>, ()>> {
        let body = self.table().get_outgoing_body(id)?;
        if let Some(stream) = body.body_output_stream.take() {
            let dummy = Resource::<u32>::new_own(id);
            let id = self.table().push_child_resource(stream, &dummy)?;
            Ok(Ok(id))
        } else {
            Ok(Err(()))
        }
    }

    fn outgoing_body_finish(
        &mut self,
        id: OutgoingBody,
        ts: Option<Trailers>,
    ) -> wasmtime::Result<()> {
        let mut body = self.table().delete_outgoing_body(id)?;

        let sender = body
            .finish_sender
            .take()
            .expect("outgoing-body trailer_sender consumed by a non-owning function");

        let message = if let Some(ts) = ts {
            FinishMessage::Trailers(self.table().get_fields(ts)?.clone().into())
        } else {
            FinishMessage::Finished
        };

        // Ignoring failure: receiver died sending body, but we can't report that here.
        let _ = sender.send(message.into());

        Ok(())
    }

    fn drop_outgoing_body(&mut self, id: OutgoingBody) -> wasmtime::Result<()> {
        let mut body = self.table().delete_outgoing_body(id)?;

        let sender = body
            .finish_sender
            .take()
            .expect("outgoing-body trailer_sender consumed by a non-owning function");

        // Ignoring failure: receiver died sending body, but we can't report that here.
        let _ = sender.send(FinishMessage::Abort);

        Ok(())
    }
}

impl<T: WasiHttpView> crate::bindings::http::types::HostIncomingBody for T {
    fn stream(
        &mut self,
        id: Resource<IncomingBody>,
    ) -> wasmtime::Result<Result<Resource<InputStream>, ()>> {
        let body = self.table().get_incoming_body(&id)?;

        if let Some(stream) = body.stream.take() {
            let stream = InputStream::Host(Box::new(stream));
            let stream = self.table().push_child_resource(stream, &id)?;
            return Ok(Ok(stream));
        }

        Ok(Err(()))
    }

    fn finish(&mut self, id: Resource<IncomingBody>) -> wasmtime::Result<FutureTrailers> {
        let body = self.table().delete_incoming_body(id)?;
        let trailers = self
            .table()
            .push_future_trailers(body.into_future_trailers())?;
        Ok(trailers)
    }

    fn drop(&mut self, id: Resource<IncomingBody>) -> wasmtime::Result<()> {
        let _ = self.table().delete_incoming_body(id)?;
        Ok(())
    }
}

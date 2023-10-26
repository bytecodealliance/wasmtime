use crate::bindings::http::types::{Error, Headers, Method, Scheme, StatusCode, Trailers};
use crate::body::{FinishMessage, HostFutureTrailers, HostFutureTrailersState};
use crate::types::{HostIncomingRequest, HostOutgoingResponse};
use crate::WasiHttpView;
use crate::{
    body::{HostIncomingBody, HostIncomingBodyBuilder, HostOutgoingBody},
    types::{
        FieldMap, HostFields, HostFutureIncomingResponse, HostIncomingResponse,
        HostOutgoingRequest, HostResponseOutparam,
    },
};
use anyhow::Context;
use std::any::Any;
use wasmtime::component::Resource;
use wasmtime_wasi::preview2::{
    bindings::io::streams::{InputStream, OutputStream},
    Pollable, Table,
};

impl<T: WasiHttpView> crate::bindings::http::types::Host for T {}

fn get_fields_mut<'a>(
    table: &'a mut Table,
    id: &Resource<HostFields>,
) -> wasmtime::Result<&'a mut FieldMap> {
    let fields = table.get(&id)?;
    if let HostFields::Ref { parent, get_fields } = *fields {
        let entry = table.get_any_mut(parent)?;
        return Ok(get_fields(entry));
    }

    match table.get_mut(&id)? {
        HostFields::Owned { fields } => Ok(fields),
        // NB: ideally the `if let` above would go here instead. That makes
        // the borrow-checker unhappy. Unclear why. If you, dear reader, can
        // refactor this to remove the `unreachable!` please do.
        HostFields::Ref { .. } => unreachable!(),
    }
}

impl<T: WasiHttpView> crate::bindings::http::types::HostFields for T {
    fn new(&mut self, entries: Vec<(String, Vec<u8>)>) -> wasmtime::Result<Resource<HostFields>> {
        let mut map = hyper::HeaderMap::new();

        for (header, value) in entries {
            let header = hyper::header::HeaderName::from_bytes(header.as_bytes())?;
            let value = hyper::header::HeaderValue::from_bytes(&value)?;
            map.append(header, value);
        }

        let id = self
            .table()
            .push(HostFields::Owned { fields: map })
            .context("[new_fields] pushing fields")?;

        Ok(id)
    }

    fn drop(&mut self, fields: Resource<HostFields>) -> wasmtime::Result<()> {
        self.table()
            .delete(fields)
            .context("[drop_fields] deleting fields")?;
        Ok(())
    }

    fn get(
        &mut self,
        fields: Resource<HostFields>,
        name: String,
    ) -> wasmtime::Result<Vec<Vec<u8>>> {
        let res = get_fields_mut(self.table(), &fields)
            .context("[fields_get] getting fields")?
            .get_all(hyper::header::HeaderName::from_bytes(name.as_bytes())?)
            .into_iter()
            .map(|val| val.as_bytes().to_owned())
            .collect();
        Ok(res)
    }

    fn set(
        &mut self,
        fields: Resource<HostFields>,
        name: String,
        values: Vec<Vec<u8>>,
    ) -> wasmtime::Result<()> {
        let m = get_fields_mut(self.table(), &fields)?;

        let header = hyper::header::HeaderName::from_bytes(name.as_bytes())?;

        m.remove(&header);
        for value in values {
            let value = hyper::header::HeaderValue::from_bytes(&value)?;
            m.append(&header, value);
        }

        Ok(())
    }

    fn delete(&mut self, fields: Resource<HostFields>, name: String) -> wasmtime::Result<()> {
        let m = get_fields_mut(self.table(), &fields)?;
        let header = hyper::header::HeaderName::from_bytes(name.as_bytes())?;
        m.remove(header);
        Ok(())
    }

    fn append(
        &mut self,
        fields: Resource<HostFields>,
        name: String,
        value: Vec<u8>,
    ) -> wasmtime::Result<()> {
        let m = get_fields_mut(self.table(), &fields)
            .context("[fields_append] getting mutable fields")?;
        let header = hyper::header::HeaderName::from_bytes(name.as_bytes())?;
        let value = hyper::header::HeaderValue::from_bytes(&value)?;
        m.append(header, value);
        Ok(())
    }

    fn entries(
        &mut self,
        fields: Resource<HostFields>,
    ) -> wasmtime::Result<Vec<(String, Vec<u8>)>> {
        let fields = get_fields_mut(self.table(), &fields)?;
        let result = fields
            .iter()
            .map(|(name, value)| (name.as_str().to_owned(), value.as_bytes().to_owned()))
            .collect();
        Ok(result)
    }

    fn clone(&mut self, fields: Resource<HostFields>) -> wasmtime::Result<Resource<HostFields>> {
        let fields = get_fields_mut(self.table(), &fields)
            .context("[fields_clone] getting fields")?
            .clone();

        let id = self
            .table()
            .push(HostFields::Owned { fields })
            .context("[fields_clone] pushing fields")?;

        Ok(id)
    }
}

impl<T: WasiHttpView> crate::bindings::http::types::HostIncomingRequest for T {
    fn method(&mut self, id: Resource<HostIncomingRequest>) -> wasmtime::Result<Method> {
        let method = self.table().get(&id)?.parts.method.as_ref();

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
    fn path_with_query(
        &mut self,
        id: Resource<HostIncomingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        let req = self.table().get(&id)?;
        Ok(req
            .parts
            .uri
            .path_and_query()
            .map(|path_and_query| path_and_query.as_str().to_owned()))
    }
    fn scheme(&mut self, id: Resource<HostIncomingRequest>) -> wasmtime::Result<Option<Scheme>> {
        let req = self.table().get(&id)?;
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
    fn authority(&mut self, id: Resource<HostIncomingRequest>) -> wasmtime::Result<Option<String>> {
        let req = self.table().get(&id)?;
        Ok(req
            .parts
            .uri
            .authority()
            .map(|auth| auth.as_str().to_owned()))
    }

    fn headers(
        &mut self,
        id: Resource<HostIncomingRequest>,
    ) -> wasmtime::Result<Resource<Headers>> {
        let _ = self.table().get(&id)?;

        fn get_fields(elem: &mut dyn Any) -> &mut FieldMap {
            &mut elem
                .downcast_mut::<HostIncomingRequest>()
                .unwrap()
                .parts
                .headers
        }

        let headers = self.table().push_child(
            HostFields::Ref {
                parent: id.rep(),
                get_fields,
            },
            &id,
        )?;

        Ok(headers)
    }

    fn consume(
        &mut self,
        id: Resource<HostIncomingRequest>,
    ) -> wasmtime::Result<Result<Resource<HostIncomingBody>, ()>> {
        let req = self.table().get_mut(&id)?;
        match req.body.take() {
            Some(builder) => {
                let id = self.table().push(builder.build())?;
                Ok(Ok(id))
            }

            None => Ok(Err(())),
        }
    }

    fn drop(&mut self, id: Resource<HostIncomingRequest>) -> wasmtime::Result<()> {
        let _ = self.table().delete(id)?;
        Ok(())
    }
}

impl<T: WasiHttpView> crate::bindings::http::types::HostOutgoingRequest for T {
    fn new(
        &mut self,
        method: Method,
        path_with_query: Option<String>,
        scheme: Option<Scheme>,
        authority: Option<String>,
        headers: Resource<Headers>,
    ) -> wasmtime::Result<Resource<HostOutgoingRequest>> {
        let headers = get_fields_mut(self.table(), &headers)?.clone();
        self.table()
            .push(HostOutgoingRequest {
                path_with_query: path_with_query.unwrap_or("".to_string()),
                authority: authority.unwrap_or("".to_string()),
                method,
                headers,
                scheme,
                body: None,
            })
            .context("[new_outgoing_request] pushing request")
    }

    fn write(
        &mut self,
        request: Resource<HostOutgoingRequest>,
    ) -> wasmtime::Result<Result<Resource<HostOutgoingBody>, ()>> {
        let req = self
            .table()
            .get_mut(&request)
            .context("[outgoing_request_write] getting request")?;

        if req.body.is_some() {
            return Ok(Err(()));
        }

        let (host_body, hyper_body) = HostOutgoingBody::new();

        req.body = Some(hyper_body);

        // The output stream will necessarily outlive the request, because we could be still
        // writing to the stream after `outgoing-handler.handle` is called.
        let outgoing_body = self.table().push(host_body)?;

        Ok(Ok(outgoing_body))
    }

    fn drop(&mut self, request: Resource<HostOutgoingRequest>) -> wasmtime::Result<()> {
        let _ = self.table().delete(request)?;
        Ok(())
    }
}

impl<T: WasiHttpView> crate::bindings::http::types::HostResponseOutparam for T {
    fn drop(&mut self, id: Resource<HostResponseOutparam>) -> wasmtime::Result<()> {
        let _ = self.table().delete(id)?;
        Ok(())
    }
    fn set(
        &mut self,
        id: Resource<HostResponseOutparam>,
        resp: Result<Resource<HostOutgoingResponse>, Error>,
    ) -> wasmtime::Result<()> {
        let val = match resp {
            Ok(resp) => Ok(self.table().delete(resp)?.try_into()?),
            Err(e) => Err(e),
        };

        self.table()
            .delete(id)?
            .result
            .send(val)
            .map_err(|_| anyhow::anyhow!("failed to initialize response"))
    }
}

impl<T: WasiHttpView> crate::bindings::http::types::HostIncomingResponse for T {
    fn drop(&mut self, response: Resource<HostIncomingResponse>) -> wasmtime::Result<()> {
        let _ = self
            .table()
            .delete(response)
            .context("[drop_incoming_response] deleting response")?;
        Ok(())
    }

    fn status(&mut self, response: Resource<HostIncomingResponse>) -> wasmtime::Result<StatusCode> {
        let r = self
            .table()
            .get(&response)
            .context("[incoming_response_status] getting response")?;
        Ok(r.status)
    }

    fn headers(
        &mut self,
        response: Resource<HostIncomingResponse>,
    ) -> wasmtime::Result<Resource<Headers>> {
        let _ = self
            .table()
            .get(&response)
            .context("[incoming_response_headers] getting response")?;

        fn get_fields(elem: &mut dyn Any) -> &mut FieldMap {
            &mut elem.downcast_mut::<HostIncomingResponse>().unwrap().headers
        }

        let id = self.table().push_child(
            HostFields::Ref {
                parent: response.rep(),
                get_fields,
            },
            &response,
        )?;

        Ok(id)
    }

    fn consume(
        &mut self,
        response: Resource<HostIncomingResponse>,
    ) -> wasmtime::Result<Result<Resource<HostIncomingBody>, ()>> {
        let table = self.table();
        let r = table
            .get_mut(&response)
            .context("[incoming_response_consume] getting response")?;

        match r.body.take() {
            Some(builder) => {
                let id = self.table().push(builder.build())?;
                Ok(Ok(id))
            }

            None => Ok(Err(())),
        }
    }
}

impl<T: WasiHttpView> crate::bindings::http::types::HostFutureTrailers for T {
    fn drop(&mut self, id: Resource<HostFutureTrailers>) -> wasmtime::Result<()> {
        let _ = self
            .table()
            .delete(id)
            .context("[drop future-trailers] deleting future-trailers")?;
        Ok(())
    }

    fn subscribe(
        &mut self,
        index: Resource<HostFutureTrailers>,
    ) -> wasmtime::Result<Resource<Pollable>> {
        wasmtime_wasi::preview2::subscribe(self.table(), index)
    }

    fn get(
        &mut self,
        id: Resource<HostFutureTrailers>,
    ) -> wasmtime::Result<Option<Result<Option<Resource<Trailers>>, Error>>> {
        let trailers = self.table().get_mut(&id)?;
        match &trailers.state {
            HostFutureTrailersState::Waiting(_) => return Ok(None),
            HostFutureTrailersState::Done(Err(e)) => return Ok(Some(Err(e.clone()))),
            HostFutureTrailersState::Done(Ok(None)) => return Ok(Some(Ok(None))),
            HostFutureTrailersState::Done(Ok(Some(_))) => {}
        }

        fn get_fields(elem: &mut dyn Any) -> &mut FieldMap {
            let trailers = elem.downcast_mut::<HostFutureTrailers>().unwrap();
            match &mut trailers.state {
                HostFutureTrailersState::Done(Ok(Some(e))) => e,
                _ => unreachable!(),
            }
        }

        let hdrs = self.table().push_child(
            HostFields::Ref {
                parent: id.rep(),
                get_fields,
            },
            &id,
        )?;

        Ok(Some(Ok(Some(hdrs))))
    }
}

impl<T: WasiHttpView> crate::bindings::http::types::HostIncomingBody for T {
    fn stream(
        &mut self,
        id: Resource<HostIncomingBody>,
    ) -> wasmtime::Result<Result<Resource<InputStream>, ()>> {
        let body = self.table().get_mut(&id)?;

        if let Some(stream) = body.stream.take() {
            let stream = InputStream::Host(Box::new(stream));
            let stream = self.table().push_child(stream, &id)?;
            return Ok(Ok(stream));
        }

        Ok(Err(()))
    }

    fn finish(
        &mut self,
        id: Resource<HostIncomingBody>,
    ) -> wasmtime::Result<Resource<HostFutureTrailers>> {
        let body = self.table().delete(id)?;
        let trailers = self.table().push(body.into_future_trailers())?;
        Ok(trailers)
    }

    fn drop(&mut self, id: Resource<HostIncomingBody>) -> wasmtime::Result<()> {
        let _ = self.table().delete(id)?;
        Ok(())
    }
}

impl<T: WasiHttpView> crate::bindings::http::types::HostOutgoingResponse for T {
    fn new(
        &mut self,
        status: StatusCode,
        headers: Resource<Headers>,
    ) -> wasmtime::Result<Resource<HostOutgoingResponse>> {
        let fields = get_fields_mut(self.table(), &headers)?.clone();

        let id = self.table().push(HostOutgoingResponse {
            status,
            headers: fields,
            body: None,
        })?;

        Ok(id)
    }

    fn write(
        &mut self,
        id: Resource<HostOutgoingResponse>,
    ) -> wasmtime::Result<Result<Resource<HostOutgoingBody>, ()>> {
        let resp = self.table().get_mut(&id)?;

        if resp.body.is_some() {
            return Ok(Err(()));
        }

        let (host, body) = HostOutgoingBody::new();

        resp.body.replace(body);

        let id = self.table().push(host)?;

        Ok(Ok(id))
    }

    fn drop(&mut self, id: Resource<HostOutgoingResponse>) -> wasmtime::Result<()> {
        let _ = self.table().delete(id)?;
        Ok(())
    }
}

impl<T: WasiHttpView> crate::bindings::http::types::HostFutureIncomingResponse for T {
    fn drop(&mut self, id: Resource<HostFutureIncomingResponse>) -> wasmtime::Result<()> {
        let _ = self.table().delete(id)?;
        Ok(())
    }

    fn get(
        &mut self,
        id: Resource<HostFutureIncomingResponse>,
    ) -> wasmtime::Result<Option<Result<Result<Resource<HostIncomingResponse>, Error>, ()>>> {
        let resp = self.table().get_mut(&id)?;

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

        let resp = self.table().push(HostIncomingResponse {
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
    fn subscribe(
        &mut self,
        id: Resource<HostFutureIncomingResponse>,
    ) -> wasmtime::Result<Resource<Pollable>> {
        wasmtime_wasi::preview2::subscribe(self.table(), id)
    }
}

impl<T: WasiHttpView> crate::bindings::http::types::HostOutgoingBody for T {
    fn write(
        &mut self,
        id: Resource<HostOutgoingBody>,
    ) -> wasmtime::Result<Result<Resource<OutputStream>, ()>> {
        let body = self.table().get_mut(&id)?;
        if let Some(stream) = body.body_output_stream.take() {
            let id = self.table().push_child(stream, &id)?;
            Ok(Ok(id))
        } else {
            Ok(Err(()))
        }
    }

    fn finish(
        &mut self,
        id: Resource<HostOutgoingBody>,
        ts: Option<Resource<Trailers>>,
    ) -> wasmtime::Result<()> {
        let mut body = self.table().delete(id)?;

        let sender = body
            .finish_sender
            .take()
            .expect("outgoing-body trailer_sender consumed by a non-owning function");

        let message = if let Some(ts) = ts {
            FinishMessage::Trailers(get_fields_mut(self.table(), &ts)?.clone().into())
        } else {
            FinishMessage::Finished
        };

        // Ignoring failure: receiver died sending body, but we can't report that here.
        let _ = sender.send(message.into());

        Ok(())
    }

    fn drop(&mut self, id: Resource<HostOutgoingBody>) -> wasmtime::Result<()> {
        let mut body = self.table().delete(id)?;

        let sender = body
            .finish_sender
            .take()
            .expect("outgoing-body trailer_sender consumed by a non-owning function");

        // Ignoring failure: receiver died sending body, but we can't report that here.
        let _ = sender.send(FinishMessage::Abort);

        Ok(())
    }
}

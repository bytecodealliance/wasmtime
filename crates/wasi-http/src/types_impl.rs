use crate::bindings::http::types::{
    self, Error, HeaderError, Headers, Method, Scheme, StatusCode, Trailers,
};
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
use hyper::header::HeaderName;
use std::{any::Any, sync::Arc};
use wasmtime::component::Resource;
use wasmtime_wasi::preview2::{
    bindings::io::streams::{InputStream, OutputStream},
    Pollable, Table,
};

impl<T: WasiHttpView> crate::bindings::http::types::Host for T {}

/// Take ownership of the underlying [`FieldMap`] associated with this fields resource. If the
/// fields resource references another fields, the returned [`FieldMap`] will be cloned.
fn move_fields(table: &mut Table, id: Resource<HostFields>) -> wasmtime::Result<FieldMap> {
    match table.delete(id)? {
        HostFields::Ref { parent, get_fields } => {
            let entry = table.get_any_mut(parent)?;
            Ok(get_fields(entry).clone())
        }

        HostFields::Owned { fields } => Ok(fields),
    }
}

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

fn is_forbidden_header<T: WasiHttpView>(view: &mut T, name: &HeaderName) -> bool {
    static FORBIDDEN_HEADERS: [HeaderName; 9] = [
        hyper::header::CONNECTION,
        HeaderName::from_static("keep-alive"),
        hyper::header::PROXY_AUTHENTICATE,
        hyper::header::PROXY_AUTHORIZATION,
        HeaderName::from_static("proxy-connection"),
        hyper::header::TE,
        hyper::header::TRANSFER_ENCODING,
        hyper::header::UPGRADE,
        HeaderName::from_static("http2-settings"),
    ];

    FORBIDDEN_HEADERS.contains(name) || view.is_forbidden_header(name)
}

impl<T: WasiHttpView> crate::bindings::http::types::HostFields for T {
    fn new(&mut self) -> wasmtime::Result<Resource<HostFields>> {
        let id = self
            .table()
            .push(HostFields::Owned {
                fields: hyper::HeaderMap::new(),
            })
            .context("[new_fields] pushing fields")?;

        Ok(id)
    }

    fn from_list(
        &mut self,
        entries: Vec<(String, Vec<u8>)>,
    ) -> wasmtime::Result<Result<Resource<HostFields>, HeaderError>> {
        let mut map = hyper::HeaderMap::new();

        for (header, value) in entries {
            let header = match hyper::header::HeaderName::from_bytes(header.as_bytes()) {
                Ok(header) => header,
                Err(_) => return Ok(Err(HeaderError::InvalidSyntax)),
            };

            if is_forbidden_header(self, &header) {
                return Ok(Err(HeaderError::Forbidden));
            }

            let value = match hyper::header::HeaderValue::from_bytes(&value) {
                Ok(value) => value,
                Err(_) => return Ok(Err(HeaderError::InvalidSyntax)),
            };

            map.append(header, value);
        }

        let id = self
            .table()
            .push(HostFields::Owned { fields: map })
            .context("[new_fields] pushing fields")?;

        Ok(Ok(id))
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
        let header = match hyper::header::HeaderName::from_bytes(name.as_bytes()) {
            Ok(header) => header,
            Err(_) => return Ok(vec![]),
        };

        let res = get_fields_mut(self.table(), &fields)
            .context("[fields_get] getting fields")?
            .get_all(header)
            .into_iter()
            .map(|val| val.as_bytes().to_owned())
            .collect();
        Ok(res)
    }

    fn set(
        &mut self,
        fields: Resource<HostFields>,
        name: String,
        byte_values: Vec<Vec<u8>>,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        let header = match hyper::header::HeaderName::from_bytes(name.as_bytes()) {
            Ok(header) => header,
            Err(_) => return Ok(Err(HeaderError::InvalidSyntax)),
        };

        if is_forbidden_header(self, &header) {
            return Ok(Err(HeaderError::Forbidden));
        }

        let mut values = Vec::with_capacity(byte_values.len());
        for value in byte_values {
            match hyper::header::HeaderValue::from_bytes(&value) {
                Ok(value) => values.push(value),
                Err(_) => return Ok(Err(HeaderError::InvalidSyntax)),
            }
        }

        let m =
            get_fields_mut(self.table(), &fields).context("[fields_set] getting mutable fields")?;
        m.remove(&header);
        for value in values {
            m.append(&header, value);
        }

        Ok(Ok(()))
    }

    fn delete(&mut self, fields: Resource<HostFields>, name: String) -> wasmtime::Result<()> {
        let header = match hyper::header::HeaderName::from_bytes(name.as_bytes()) {
            Ok(header) => header,
            Err(_) => return Ok(()),
        };

        let m = get_fields_mut(self.table(), &fields)?;
        m.remove(header);
        Ok(())
    }

    fn append(
        &mut self,
        fields: Resource<HostFields>,
        name: String,
        value: Vec<u8>,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        let header = match hyper::header::HeaderName::from_bytes(name.as_bytes()) {
            Ok(header) => header,
            Err(_) => return Ok(Err(HeaderError::InvalidSyntax)),
        };

        if is_forbidden_header(self, &header) {
            return Ok(Err(HeaderError::Forbidden));
        }

        let value = match hyper::header::HeaderValue::from_bytes(&value) {
            Ok(value) => value,
            Err(_) => return Ok(Err(HeaderError::InvalidSyntax)),
        };

        let m = get_fields_mut(self.table(), &fields)
            .context("[fields_append] getting mutable fields")?;

        m.append(header, value);
        Ok(Ok(()))
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
        let method = self.table().get(&id)?.parts.method.clone();
        Ok(method.into())
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
        let headers = move_fields(self.table(), headers)?;

        self.table()
            .push(HostOutgoingRequest {
                path_with_query,
                authority,
                method,
                headers,
                scheme,
                body: None,
            })
            .context("[new_outgoing_request] pushing request")
    }

    fn body(
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

    fn method(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
    ) -> wasmtime::Result<Method> {
        Ok(self.table().get(&request)?.method.clone().try_into()?)
    }

    fn set_method(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
        method: Method,
    ) -> wasmtime::Result<()> {
        self.table().get_mut(&request)?.method = method.into();
        Ok(())
    }

    fn path_with_query(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        Ok(self.table().get(&request)?.path_with_query.clone())
    }

    fn set_path_with_query(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
        path_with_query: Option<String>,
    ) -> wasmtime::Result<()> {
        self.table().get_mut(&request)?.path_with_query = path_with_query;
        Ok(())
    }

    fn scheme(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
    ) -> wasmtime::Result<Option<Scheme>> {
        Ok(self.table().get(&request)?.scheme.clone())
    }

    fn set_scheme(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
        scheme: Option<Scheme>,
    ) -> wasmtime::Result<()> {
        self.table().get_mut(&request)?.scheme = scheme;
        Ok(())
    }

    fn authority(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        Ok(self.table().get(&request)?.authority.clone())
    }

    fn set_authority(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
        authority: Option<String>,
    ) -> wasmtime::Result<()> {
        self.table().get_mut(&request)?.authority = authority;
        Ok(())
    }

    fn headers(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Headers>> {
        let _ = self
            .table()
            .get(&request)
            .context("[outgoing_request_headers] getting request")?;

        fn get_fields(elem: &mut dyn Any) -> &mut FieldMap {
            &mut elem
                .downcast_mut::<types::OutgoingRequest>()
                .unwrap()
                .headers
        }

        let id = self.table().push_child(
            HostFields::Ref {
                parent: request.rep(),
                get_fields,
            },
            &request,
        )?;

        Ok(id)
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
        headers: Resource<Headers>,
    ) -> wasmtime::Result<Resource<HostOutgoingResponse>> {
        let fields = move_fields(self.table(), headers)?;

        let id = self.table().push(HostOutgoingResponse {
            status: http::StatusCode::OK,
            headers: fields,
            body: None,
        })?;

        Ok(id)
    }

    fn body(
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

    fn status_code(
        &mut self,
        id: Resource<HostOutgoingResponse>,
    ) -> wasmtime::Result<types::StatusCode> {
        Ok(self.table().get(&id)?.status.into())
    }

    fn set_status_code(
        &mut self,
        id: Resource<HostOutgoingResponse>,
        status: types::StatusCode,
    ) -> wasmtime::Result<Result<(), Error>> {
        let resp = self.table().get_mut(&id)?;

        match http::StatusCode::from_u16(status) {
            Ok(status) => resp.status = status,
            Err(_) => {
                return Ok(Err(Error::UnexpectedError(
                    "Invalid status code".to_string(),
                )))
            }
        };

        Ok(Ok(()))
    }

    fn headers(
        &mut self,
        id: Resource<HostOutgoingResponse>,
    ) -> wasmtime::Result<Resource<types::Headers>> {
        // Trap if the outgoing-response doesn't exist.
        let _ = self.table().get(&id)?;

        fn get_fields(elem: &mut dyn Any) -> &mut FieldMap {
            let resp = elem.downcast_mut::<HostOutgoingResponse>().unwrap();
            &mut resp.headers
        }

        Ok(self.table().push_child(
            HostFields::Ref {
                parent: id.rep(),
                get_fields,
            },
            &id,
        )?)
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

                Ok(Ok(resp)) => resp,
                Ok(Err(e)) => return Ok(Some(Ok(Err(e)))),
            };

        let (parts, body) = resp.resp.into_parts();

        let resp = self.table().push(HostIncomingResponse {
            status: parts.status.as_u16(),
            headers: FieldMap::from(parts.headers),
            body: Some(HostIncomingBodyBuilder {
                body,
                worker: Some(Arc::clone(&resp.worker)),
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

impl<T: WasiHttpView> crate::bindings::http::types::HostRequestOptions for T {
    fn new(&mut self) -> wasmtime::Result<Resource<types::RequestOptions>> {
        let id = self.table().push(types::RequestOptions::default())?;
        Ok(id)
    }

    fn connect_timeout_ms(
        &mut self,
        opts: Resource<types::RequestOptions>,
    ) -> wasmtime::Result<Option<types::Duration>> {
        let millis = self
            .table()
            .get(&opts)?
            .connect_timeout
            .map(|d| d.as_millis());

        if let Some(millis) = millis {
            Ok(Some(millis.try_into()?))
        } else {
            Ok(None)
        }
    }

    fn set_connect_timeout_ms(
        &mut self,
        opts: Resource<types::RequestOptions>,
        ms: Option<types::Duration>,
    ) -> wasmtime::Result<Result<(), ()>> {
        self.table().get_mut(&opts)?.connect_timeout =
            ms.map(|ms| std::time::Duration::from_millis(ms as u64));
        Ok(Ok(()))
    }

    fn first_byte_timeout_ms(
        &mut self,
        opts: Resource<types::RequestOptions>,
    ) -> wasmtime::Result<Option<types::Duration>> {
        let millis = self
            .table()
            .get(&opts)?
            .first_byte_timeout
            .map(|d| d.as_millis());

        if let Some(millis) = millis {
            Ok(Some(millis.try_into()?))
        } else {
            Ok(None)
        }
    }

    fn set_first_byte_timeout_ms(
        &mut self,
        opts: Resource<types::RequestOptions>,
        ms: Option<types::Duration>,
    ) -> wasmtime::Result<Result<(), ()>> {
        self.table().get_mut(&opts)?.first_byte_timeout =
            ms.map(|ms| std::time::Duration::from_millis(ms as u64));
        Ok(Ok(()))
    }

    fn between_bytes_timeout_ms(
        &mut self,
        opts: Resource<types::RequestOptions>,
    ) -> wasmtime::Result<Option<types::Duration>> {
        let millis = self
            .table()
            .get(&opts)?
            .between_bytes_timeout
            .map(|d| d.as_millis());

        if let Some(millis) = millis {
            Ok(Some(millis.try_into()?))
        } else {
            Ok(None)
        }
    }

    fn set_between_bytes_timeout_ms(
        &mut self,
        opts: Resource<types::RequestOptions>,
        ms: Option<types::Duration>,
    ) -> wasmtime::Result<Result<(), ()>> {
        self.table().get_mut(&opts)?.between_bytes_timeout =
            ms.map(|ms| std::time::Duration::from_millis(ms as u64));
        Ok(Ok(()))
    }

    fn drop(&mut self, rep: Resource<types::RequestOptions>) -> wasmtime::Result<()> {
        let _ = self.table().delete(rep)?;
        Ok(())
    }
}

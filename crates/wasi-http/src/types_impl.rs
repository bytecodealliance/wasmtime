//! Implementation for the `wasi:http/types` interface.

use crate::{
    bindings::http::types::{self, Headers, Method, Scheme, StatusCode, Trailers},
    body::{HostFutureTrailers, HostIncomingBody, HostOutgoingBody, StreamContext},
    types::{
        is_forbidden_header, remove_forbidden_headers, FieldMap, HostFields,
        HostFutureIncomingResponse, HostIncomingRequest, HostIncomingResponse, HostOutgoingRequest,
        HostOutgoingResponse, HostResponseOutparam,
    },
    WasiHttpImpl, WasiHttpView,
};
use anyhow::Context;
use std::any::Any;
use std::str::FromStr;
use wasmtime::component::{Resource, ResourceTable, ResourceTableError};
use wasmtime_wasi_io::{
    poll::Pollable,
    streams::{InputStream, OutputStream},
    IoView,
};

impl<T> crate::bindings::http::types::Host for WasiHttpImpl<T>
where
    T: WasiHttpView,
{
    fn convert_error_code(&mut self, err: crate::HttpError) -> wasmtime::Result<types::ErrorCode> {
        err.downcast()
    }

    fn http_error_code(
        &mut self,
        err: wasmtime::component::Resource<types::IoError>,
    ) -> wasmtime::Result<Option<types::ErrorCode>> {
        let e = self.table().get(&err)?;
        Ok(e.downcast_ref::<types::ErrorCode>().cloned())
    }
}

/// Extract the `Content-Length` header value from a [`FieldMap`], returning `None` if it's not
/// present. This function will return `Err` if it's not possible to parse the `Content-Length`
/// header.
fn get_content_length(fields: &FieldMap) -> Result<Option<u64>, ()> {
    let header_val = match fields.get(hyper::header::CONTENT_LENGTH) {
        Some(val) => val,
        None => return Ok(None),
    };

    let header_str = match header_val.to_str() {
        Ok(val) => val,
        Err(_) => return Err(()),
    };

    match header_str.parse() {
        Ok(len) => Ok(Some(len)),
        Err(_) => Err(()),
    }
}

/// Take ownership of the underlying [`FieldMap`] associated with this fields resource. If the
/// fields resource references another fields, the returned [`FieldMap`] will be cloned.
fn move_fields(
    table: &mut ResourceTable,
    id: Resource<HostFields>,
) -> Result<FieldMap, ResourceTableError> {
    match table.delete(id)? {
        HostFields::Ref { parent, get_fields } => {
            let entry = table.get_any_mut(parent)?;
            Ok(get_fields(entry).clone())
        }

        HostFields::Owned { fields } => Ok(fields),
    }
}

fn get_fields<'a>(
    table: &'a mut ResourceTable,
    id: &Resource<HostFields>,
) -> wasmtime::Result<&'a FieldMap> {
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

fn get_fields_mut<'a>(
    table: &'a mut ResourceTable,
    id: &Resource<HostFields>,
) -> wasmtime::Result<Result<&'a mut FieldMap, types::HeaderError>> {
    match table.get_mut(&id)? {
        HostFields::Owned { fields } => Ok(Ok(fields)),
        HostFields::Ref { .. } => Ok(Err(types::HeaderError::Immutable)),
    }
}

impl<T> crate::bindings::http::types::HostFields for WasiHttpImpl<T>
where
    T: WasiHttpView,
{
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
    ) -> wasmtime::Result<Result<Resource<HostFields>, types::HeaderError>> {
        let mut fields = hyper::HeaderMap::new();

        for (header, value) in entries {
            let header = match hyper::header::HeaderName::from_bytes(header.as_bytes()) {
                Ok(header) => header,
                Err(_) => return Ok(Err(types::HeaderError::InvalidSyntax)),
            };

            if is_forbidden_header(self, &header) {
                return Ok(Err(types::HeaderError::Forbidden));
            }

            let value = match hyper::header::HeaderValue::from_bytes(&value) {
                Ok(value) => value,
                Err(_) => return Ok(Err(types::HeaderError::InvalidSyntax)),
            };

            fields.append(header, value);
        }

        let id = self
            .table()
            .push(HostFields::Owned { fields })
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
        let fields = get_fields(self.table(), &fields).context("[fields_get] getting fields")?;

        let header = match hyper::header::HeaderName::from_bytes(name.as_bytes()) {
            Ok(header) => header,
            Err(_) => return Ok(vec![]),
        };

        if !fields.contains_key(&header) {
            return Ok(vec![]);
        }

        let res = fields
            .get_all(&header)
            .into_iter()
            .map(|val| val.as_bytes().to_owned())
            .collect();
        Ok(res)
    }

    fn has(&mut self, fields: Resource<HostFields>, name: String) -> wasmtime::Result<bool> {
        let fields = get_fields(self.table(), &fields).context("[fields_get] getting fields")?;

        match hyper::header::HeaderName::from_bytes(name.as_bytes()) {
            Ok(header) => Ok(fields.contains_key(&header)),
            Err(_) => Ok(false),
        }
    }

    fn set(
        &mut self,
        fields: Resource<HostFields>,
        name: String,
        byte_values: Vec<Vec<u8>>,
    ) -> wasmtime::Result<Result<(), types::HeaderError>> {
        let header = match hyper::header::HeaderName::from_bytes(name.as_bytes()) {
            Ok(header) => header,
            Err(_) => return Ok(Err(types::HeaderError::InvalidSyntax)),
        };

        if is_forbidden_header(self, &header) {
            return Ok(Err(types::HeaderError::Forbidden));
        }

        let mut values = Vec::with_capacity(byte_values.len());
        for value in byte_values {
            match hyper::header::HeaderValue::from_bytes(&value) {
                Ok(value) => values.push(value),
                Err(_) => return Ok(Err(types::HeaderError::InvalidSyntax)),
            }
        }

        Ok(get_fields_mut(self.table(), &fields)
            .context("[fields_set] getting mutable fields")?
            .map(|fields| {
                fields.remove(&header);
                for value in values {
                    fields.append(&header, value);
                }
            }))
    }

    fn delete(
        &mut self,
        fields: Resource<HostFields>,
        name: String,
    ) -> wasmtime::Result<Result<(), types::HeaderError>> {
        let header = match hyper::header::HeaderName::from_bytes(name.as_bytes()) {
            Ok(header) => header,
            Err(_) => return Ok(Err(types::HeaderError::InvalidSyntax)),
        };

        if is_forbidden_header(self, &header) {
            return Ok(Err(types::HeaderError::Forbidden));
        }

        Ok(get_fields_mut(self.table(), &fields)?.map(|fields| {
            fields.remove(header);
        }))
    }

    fn append(
        &mut self,
        fields: Resource<HostFields>,
        name: String,
        value: Vec<u8>,
    ) -> wasmtime::Result<Result<(), types::HeaderError>> {
        let header = match hyper::header::HeaderName::from_bytes(name.as_bytes()) {
            Ok(header) => header,
            Err(_) => return Ok(Err(types::HeaderError::InvalidSyntax)),
        };

        if is_forbidden_header(self, &header) {
            return Ok(Err(types::HeaderError::Forbidden));
        }

        let value = match hyper::header::HeaderValue::from_bytes(&value) {
            Ok(value) => value,
            Err(_) => return Ok(Err(types::HeaderError::InvalidSyntax)),
        };

        Ok(get_fields_mut(self.table(), &fields)
            .context("[fields_append] getting mutable fields")?
            .map(|fields| {
                fields.append(header, value);
            }))
    }

    fn entries(
        &mut self,
        fields: Resource<HostFields>,
    ) -> wasmtime::Result<Vec<(String, Vec<u8>)>> {
        Ok(get_fields(self.table(), &fields)?
            .iter()
            .map(|(name, value)| (name.as_str().to_owned(), value.as_bytes().to_owned()))
            .collect())
    }

    fn clone(&mut self, fields: Resource<HostFields>) -> wasmtime::Result<Resource<HostFields>> {
        let fields = get_fields(self.table(), &fields)
            .context("[fields_clone] getting fields")?
            .clone();

        let id = self
            .table()
            .push(HostFields::Owned { fields })
            .context("[fields_clone] pushing fields")?;

        Ok(id)
    }
}

impl<T> crate::bindings::http::types::HostIncomingRequest for WasiHttpImpl<T>
where
    T: WasiHttpView,
{
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
        Ok(Some(req.scheme.clone()))
    }
    fn authority(&mut self, id: Resource<HostIncomingRequest>) -> wasmtime::Result<Option<String>> {
        let req = self.table().get(&id)?;
        Ok(Some(req.authority.clone()))
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
            Some(body) => {
                let id = self.table().push(body)?;
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

impl<T> crate::bindings::http::types::HostOutgoingRequest for WasiHttpImpl<T>
where
    T: WasiHttpView,
{
    fn new(
        &mut self,
        headers: Resource<Headers>,
    ) -> wasmtime::Result<Resource<HostOutgoingRequest>> {
        let headers = move_fields(self.table(), headers)?;

        self.table()
            .push(HostOutgoingRequest {
                path_with_query: None,
                authority: None,
                method: types::Method::Get,
                headers,
                scheme: None,
                body: None,
            })
            .context("[new_outgoing_request] pushing request")
    }

    fn body(
        &mut self,
        request: Resource<HostOutgoingRequest>,
    ) -> wasmtime::Result<Result<Resource<HostOutgoingBody>, ()>> {
        let buffer_chunks = self.outgoing_body_buffer_chunks();
        let chunk_size = self.outgoing_body_chunk_size();
        let req = self
            .table()
            .get_mut(&request)
            .context("[outgoing_request_write] getting request")?;

        if req.body.is_some() {
            return Ok(Err(()));
        }

        let size = match get_content_length(&req.headers) {
            Ok(size) => size,
            Err(e) => return Ok(Err(e)),
        };

        let (host_body, hyper_body) =
            HostOutgoingBody::new(StreamContext::Request, size, buffer_chunks, chunk_size);

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
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = self.table().get_mut(&request)?;

        if let Method::Other(s) = &method {
            if let Err(_) = http::Method::from_str(s) {
                return Ok(Err(()));
            }
        }

        req.method = method;

        Ok(Ok(()))
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
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = self.table().get_mut(&request)?;

        if let Some(s) = path_with_query.as_ref() {
            if let Err(_) = http::uri::PathAndQuery::from_str(s) {
                return Ok(Err(()));
            }
        }

        req.path_with_query = path_with_query;

        Ok(Ok(()))
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
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = self.table().get_mut(&request)?;

        if let Some(types::Scheme::Other(s)) = scheme.as_ref() {
            if let Err(_) = http::uri::Scheme::from_str(s.as_str()) {
                return Ok(Err(()));
            }
        }

        req.scheme = scheme;

        Ok(Ok(()))
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
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = self.table().get_mut(&request)?;

        if let Some(s) = authority.as_ref() {
            let auth = match http::uri::Authority::from_str(s.as_str()) {
                Ok(auth) => auth,
                Err(_) => return Ok(Err(())),
            };

            if s.contains(':') && auth.port_u16().is_none() {
                return Ok(Err(()));
            }
        }

        req.authority = authority;

        Ok(Ok(()))
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

impl<T> crate::bindings::http::types::HostResponseOutparam for WasiHttpImpl<T>
where
    T: WasiHttpView,
{
    fn drop(&mut self, id: Resource<HostResponseOutparam>) -> wasmtime::Result<()> {
        let _ = self.table().delete(id)?;
        Ok(())
    }
    fn set(
        &mut self,
        id: Resource<HostResponseOutparam>,
        resp: Result<Resource<HostOutgoingResponse>, types::ErrorCode>,
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

impl<T> crate::bindings::http::types::HostIncomingResponse for WasiHttpImpl<T>
where
    T: WasiHttpView,
{
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
            Some(body) => {
                let id = self.table().push(body)?;
                Ok(Ok(id))
            }

            None => Ok(Err(())),
        }
    }
}

impl<T> crate::bindings::http::types::HostFutureTrailers for WasiHttpImpl<T>
where
    T: WasiHttpView,
{
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
        wasmtime_wasi_io::poll::subscribe(self.table(), index)
    }

    fn get(
        &mut self,
        id: Resource<HostFutureTrailers>,
    ) -> wasmtime::Result<Option<Result<Result<Option<Resource<Trailers>>, types::ErrorCode>, ()>>>
    {
        let trailers = self.table().get_mut(&id)?;
        match trailers {
            HostFutureTrailers::Waiting(_) => return Ok(None),
            HostFutureTrailers::Consumed => return Ok(Some(Err(()))),
            HostFutureTrailers::Done(_) => {}
        };

        let res = match std::mem::replace(trailers, HostFutureTrailers::Consumed) {
            HostFutureTrailers::Done(res) => res,
            _ => unreachable!(),
        };

        let mut fields = match res {
            Ok(Some(fields)) => fields,
            Ok(None) => return Ok(Some(Ok(Ok(None)))),
            Err(e) => return Ok(Some(Ok(Err(e)))),
        };

        remove_forbidden_headers(self, &mut fields);

        let ts = self.table().push(HostFields::Owned { fields })?;

        Ok(Some(Ok(Ok(Some(ts)))))
    }
}

impl<T> crate::bindings::http::types::HostIncomingBody for WasiHttpImpl<T>
where
    T: WasiHttpView,
{
    fn stream(
        &mut self,
        id: Resource<HostIncomingBody>,
    ) -> wasmtime::Result<Result<Resource<InputStream>, ()>> {
        let body = self.table().get_mut(&id)?;

        if let Some(stream) = body.take_stream() {
            let stream: InputStream = Box::new(stream);
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

impl<T> crate::bindings::http::types::HostOutgoingResponse for WasiHttpImpl<T>
where
    T: WasiHttpView,
{
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
        let buffer_chunks = self.outgoing_body_buffer_chunks();
        let chunk_size = self.outgoing_body_chunk_size();
        let resp = self.table().get_mut(&id)?;

        if resp.body.is_some() {
            return Ok(Err(()));
        }

        let size = match get_content_length(&resp.headers) {
            Ok(size) => size,
            Err(e) => return Ok(Err(e)),
        };

        let (host, body) =
            HostOutgoingBody::new(StreamContext::Response, size, buffer_chunks, chunk_size);

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
    ) -> wasmtime::Result<Result<(), ()>> {
        let resp = self.table().get_mut(&id)?;

        match http::StatusCode::from_u16(status) {
            Ok(status) => resp.status = status,
            Err(_) => return Ok(Err(())),
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

impl<T> crate::bindings::http::types::HostFutureIncomingResponse for WasiHttpImpl<T>
where
    T: WasiHttpView,
{
    fn drop(&mut self, id: Resource<HostFutureIncomingResponse>) -> wasmtime::Result<()> {
        let _ = self.table().delete(id)?;
        Ok(())
    }

    fn get(
        &mut self,
        id: Resource<HostFutureIncomingResponse>,
    ) -> wasmtime::Result<
        Option<Result<Result<Resource<HostIncomingResponse>, types::ErrorCode>, ()>>,
    > {
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
                    let e = e.downcast::<types::ErrorCode>()?;
                    return Ok(Some(Ok(Err(e))));
                }

                Ok(Ok(resp)) => resp,
                Ok(Err(e)) => return Ok(Some(Ok(Err(e)))),
            };

        let (mut parts, body) = resp.resp.into_parts();

        remove_forbidden_headers(self, &mut parts.headers);

        let resp = self.table().push(HostIncomingResponse {
            status: parts.status.as_u16(),
            headers: parts.headers,
            body: Some({
                let mut body = HostIncomingBody::new(body, resp.between_bytes_timeout);
                if let Some(worker) = resp.worker {
                    body.retain_worker(worker);
                }
                body
            }),
        })?;

        Ok(Some(Ok(Ok(resp))))
    }

    fn subscribe(
        &mut self,
        id: Resource<HostFutureIncomingResponse>,
    ) -> wasmtime::Result<Resource<Pollable>> {
        wasmtime_wasi_io::poll::subscribe(self.table(), id)
    }
}

impl<T> crate::bindings::http::types::HostOutgoingBody for WasiHttpImpl<T>
where
    T: WasiHttpView,
{
    fn write(
        &mut self,
        id: Resource<HostOutgoingBody>,
    ) -> wasmtime::Result<Result<Resource<OutputStream>, ()>> {
        let body = self.table().get_mut(&id)?;
        if let Some(stream) = body.take_output_stream() {
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
    ) -> crate::HttpResult<()> {
        let body = self.table().delete(id)?;

        let ts = if let Some(ts) = ts {
            Some(move_fields(self.table(), ts)?)
        } else {
            None
        };

        body.finish(ts)?;
        Ok(())
    }

    fn drop(&mut self, id: Resource<HostOutgoingBody>) -> wasmtime::Result<()> {
        self.table().delete(id)?.abort();
        Ok(())
    }
}

impl<T> crate::bindings::http::types::HostRequestOptions for WasiHttpImpl<T>
where
    T: WasiHttpView,
{
    fn new(&mut self) -> wasmtime::Result<Resource<types::RequestOptions>> {
        let id = self.table().push(types::RequestOptions::default())?;
        Ok(id)
    }

    fn connect_timeout(
        &mut self,
        opts: Resource<types::RequestOptions>,
    ) -> wasmtime::Result<Option<types::Duration>> {
        let nanos = self
            .table()
            .get(&opts)?
            .connect_timeout
            .map(|d| d.as_nanos());

        if let Some(nanos) = nanos {
            Ok(Some(nanos.try_into()?))
        } else {
            Ok(None)
        }
    }

    fn set_connect_timeout(
        &mut self,
        opts: Resource<types::RequestOptions>,
        duration: Option<types::Duration>,
    ) -> wasmtime::Result<Result<(), ()>> {
        self.table().get_mut(&opts)?.connect_timeout =
            duration.map(std::time::Duration::from_nanos);
        Ok(Ok(()))
    }

    fn first_byte_timeout(
        &mut self,
        opts: Resource<types::RequestOptions>,
    ) -> wasmtime::Result<Option<types::Duration>> {
        let nanos = self
            .table()
            .get(&opts)?
            .first_byte_timeout
            .map(|d| d.as_nanos());

        if let Some(nanos) = nanos {
            Ok(Some(nanos.try_into()?))
        } else {
            Ok(None)
        }
    }

    fn set_first_byte_timeout(
        &mut self,
        opts: Resource<types::RequestOptions>,
        duration: Option<types::Duration>,
    ) -> wasmtime::Result<Result<(), ()>> {
        self.table().get_mut(&opts)?.first_byte_timeout =
            duration.map(std::time::Duration::from_nanos);
        Ok(Ok(()))
    }

    fn between_bytes_timeout(
        &mut self,
        opts: Resource<types::RequestOptions>,
    ) -> wasmtime::Result<Option<types::Duration>> {
        let nanos = self
            .table()
            .get(&opts)?
            .between_bytes_timeout
            .map(|d| d.as_nanos());

        if let Some(nanos) = nanos {
            Ok(Some(nanos.try_into()?))
        } else {
            Ok(None)
        }
    }

    fn set_between_bytes_timeout(
        &mut self,
        opts: Resource<types::RequestOptions>,
        duration: Option<types::Duration>,
    ) -> wasmtime::Result<Result<(), ()>> {
        self.table().get_mut(&opts)?.between_bytes_timeout =
            duration.map(std::time::Duration::from_nanos);
        Ok(Ok(()))
    }

    fn drop(&mut self, rep: Resource<types::RequestOptions>) -> wasmtime::Result<()> {
        let _ = self.table().delete(rep)?;
        Ok(())
    }
}

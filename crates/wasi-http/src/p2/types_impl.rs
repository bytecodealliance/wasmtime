//! Implementation for the `wasi:http/types` interface.

use crate::p2::bindings::http::types::{self, Method, Scheme, StatusCode, Trailers};
use crate::p2::body::{HostFutureTrailers, HostIncomingBody, HostOutgoingBody, StreamContext};
use crate::p2::types::{
    HostFutureIncomingResponse, HostIncomingRequest, HostIncomingResponse, HostOutgoingRequest,
    HostOutgoingResponse, HostResponseOutparam,
};
use crate::p2::{HeaderError, HeaderResult, HttpError, HttpResult};
use crate::{FieldMap, WasiHttpCtxView, get_content_length};
use http::HeaderName;
use std::str::FromStr;
use wasmtime::component::Resource;
use wasmtime::{error::Context as _, format_err};
use wasmtime_wasi::p2::{DynInputStream, DynOutputStream, DynPollable};

impl types::Host for WasiHttpCtxView<'_> {
    fn convert_error_code(&mut self, err: HttpError) -> wasmtime::Result<types::ErrorCode> {
        err.downcast()
    }

    fn convert_header_error(&mut self, err: HeaderError) -> wasmtime::Result<types::HeaderError> {
        err.downcast()
    }

    fn http_error_code(
        &mut self,
        err: wasmtime::component::Resource<types::IoError>,
    ) -> wasmtime::Result<Option<types::ErrorCode>> {
        let e = self.table.get(&err)?;
        Ok(e.downcast_ref::<types::ErrorCode>().cloned())
    }
}

impl types::HostFields for WasiHttpCtxView<'_> {
    fn new(&mut self) -> wasmtime::Result<Resource<FieldMap>> {
        let limit = self.ctx.field_size_limit;
        let id = self
            .table
            .push(FieldMap::new_mutable(limit))
            .context("[new_fields] pushing fields")?;

        Ok(id)
    }

    fn from_list(&mut self, entries: Vec<(String, Vec<u8>)>) -> HeaderResult<Resource<FieldMap>> {
        let mut fields = FieldMap::new_mutable(self.ctx.field_size_limit);

        for (header, value) in entries {
            fields.append(self.hooks, header, value)?;
        }

        Ok(self.table.push(fields)?)
    }

    fn drop(&mut self, fields: Resource<FieldMap>) -> wasmtime::Result<()> {
        self.table
            .delete(fields)
            .context("[drop_fields] deleting fields")?;
        Ok(())
    }

    fn get(&mut self, fields: Resource<FieldMap>, name: String) -> wasmtime::Result<Vec<Vec<u8>>> {
        let fields = self.table.get(&fields)?;

        let header = match HeaderName::from_bytes(name.as_bytes()) {
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

    fn has(&mut self, fields: Resource<FieldMap>, name: String) -> wasmtime::Result<bool> {
        let fields = self.table.get(&fields)?;

        match HeaderName::from_bytes(name.as_bytes()) {
            Ok(header) => Ok(fields.contains_key(&header)),
            Err(_) => Ok(false),
        }
    }

    fn set(
        &mut self,
        fields: Resource<FieldMap>,
        name: String,
        values: Vec<Vec<u8>>,
    ) -> HeaderResult<()> {
        let fields = self.table.get_mut(&fields)?;
        fields.set(self.hooks, name, values)?;
        Ok(())
    }

    fn delete(&mut self, fields: Resource<FieldMap>, name: String) -> HeaderResult<()> {
        let fields = self.table.get_mut(&fields)?;
        fields.remove_all(self.hooks, name)?;
        Ok(())
    }

    fn append(
        &mut self,
        fields: Resource<FieldMap>,
        name: String,
        value: Vec<u8>,
    ) -> HeaderResult<()> {
        let fields = self.table.get_mut(&fields)?;
        fields.append(self.hooks, name, value)?;
        Ok(())
    }

    fn entries(&mut self, fields: Resource<FieldMap>) -> wasmtime::Result<Vec<(String, Vec<u8>)>> {
        Ok(self
            .table
            .get(&fields)?
            .iter()
            .map(|(name, value)| (name.as_str().to_owned(), value.as_bytes().to_owned()))
            .collect())
    }

    fn clone(&mut self, fields: Resource<FieldMap>) -> wasmtime::Result<Resource<FieldMap>> {
        let mut fields = self.table.get(&fields)?.clone();
        fields.set_mutable(self.ctx.field_size_limit);
        let id = self.table.push(fields)?;
        Ok(id)
    }
}

impl types::HostIncomingRequest for WasiHttpCtxView<'_> {
    fn method(&mut self, id: Resource<HostIncomingRequest>) -> wasmtime::Result<Method> {
        let method = self.table.get(&id)?.method.clone();
        Ok(method.into())
    }
    fn path_with_query(
        &mut self,
        id: Resource<HostIncomingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        let req = self.table.get(&id)?;
        Ok(req
            .uri
            .path_and_query()
            .map(|path_and_query| path_and_query.as_str().to_owned()))
    }
    fn scheme(&mut self, id: Resource<HostIncomingRequest>) -> wasmtime::Result<Option<Scheme>> {
        let req = self.table.get(&id)?;
        Ok(Some(req.scheme.clone()))
    }
    fn authority(&mut self, id: Resource<HostIncomingRequest>) -> wasmtime::Result<Option<String>> {
        let req = self.table.get(&id)?;
        Ok(Some(req.authority.clone()))
    }

    fn headers(
        &mut self,
        id: Resource<HostIncomingRequest>,
    ) -> wasmtime::Result<Resource<FieldMap>> {
        let req = self.table.get(&id)?;
        Ok(self.table.push(req.headers.clone())?)
    }

    fn consume(
        &mut self,
        id: Resource<HostIncomingRequest>,
    ) -> wasmtime::Result<Result<Resource<HostIncomingBody>, ()>> {
        let req = self.table.get_mut(&id)?;
        match req.body.take() {
            Some(body) => {
                let id = self.table.push(body)?;
                Ok(Ok(id))
            }

            None => Ok(Err(())),
        }
    }

    fn drop(&mut self, id: Resource<HostIncomingRequest>) -> wasmtime::Result<()> {
        let _ = self.table.delete(id)?;
        Ok(())
    }
}

impl types::HostOutgoingRequest for WasiHttpCtxView<'_> {
    fn new(
        &mut self,
        headers: Resource<FieldMap>,
    ) -> wasmtime::Result<Resource<HostOutgoingRequest>> {
        let mut headers = self.table.delete(headers)?;
        headers.set_immutable();

        self.table
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
        let buffer_chunks = self.hooks.p2_outgoing_body_buffer_chunks();
        let chunk_size = self.hooks.p2_outgoing_body_chunk_size();
        let req = self
            .table
            .get_mut(&request)
            .context("[outgoing_request_write] getting request")?;

        if req.body.is_some() {
            return Ok(Err(()));
        }

        let size = match get_content_length(&req.headers) {
            Ok(size) => size,
            Err(..) => return Ok(Err(())),
        };

        let (host_body, hyper_body) =
            HostOutgoingBody::new(StreamContext::Request, size, buffer_chunks, chunk_size);

        req.body = Some(hyper_body);

        // The output stream will necessarily outlive the request, because we could be still
        // writing to the stream after `outgoing-handler.handle` is called.
        let outgoing_body = self.table.push(host_body)?;

        Ok(Ok(outgoing_body))
    }

    fn drop(&mut self, request: Resource<HostOutgoingRequest>) -> wasmtime::Result<()> {
        let _ = self.table.delete(request)?;
        Ok(())
    }

    fn method(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
    ) -> wasmtime::Result<Method> {
        Ok(self.table.get(&request)?.method.clone())
    }

    fn set_method(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
        method: Method,
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = self.table.get_mut(&request)?;

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
        Ok(self.table.get(&request)?.path_with_query.clone())
    }

    fn set_path_with_query(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
        path_with_query: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = self.table.get_mut(&request)?;

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
        Ok(self.table.get(&request)?.scheme.clone())
    }

    fn set_scheme(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
        scheme: Option<Scheme>,
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = self.table.get_mut(&request)?;

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
        Ok(self.table.get(&request)?.authority.clone())
    }

    fn set_authority(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
        authority: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = self.table.get_mut(&request)?;

        // Match p3: reject empty / non-numeric / out-of-range ports that
        // `http::uri::Authority` alone would accept (see crate::parse_authority).
        if let Some(s) = authority {
            let Ok(parsed) = crate::parse_authority(s) else {
                return Ok(Err(()));
            };
            req.authority = Some(parsed.as_str().into());
        } else {
            req.authority = None;
        }

        Ok(Ok(()))
    }

    fn headers(
        &mut self,
        request: wasmtime::component::Resource<types::OutgoingRequest>,
    ) -> wasmtime::Result<wasmtime::component::Resource<FieldMap>> {
        let req = self.table.get(&request)?;
        let id = self.table.push(req.headers.clone())?;
        Ok(id)
    }
}

impl types::HostResponseOutparam for WasiHttpCtxView<'_> {
    fn drop(&mut self, id: Resource<HostResponseOutparam>) -> wasmtime::Result<()> {
        let _ = self.table.delete(id)?;
        Ok(())
    }
    fn set(
        &mut self,
        id: Resource<HostResponseOutparam>,
        resp: Result<Resource<HostOutgoingResponse>, types::ErrorCode>,
    ) -> wasmtime::Result<()> {
        let val = match resp {
            Ok(resp) => Ok(self.table.delete(resp)?.try_into()?),
            Err(e) => Err(e),
        };

        let resp = self.table.delete(id)?;
        (resp.send)(val);
        Ok(())
    }

    fn send_informational(
        &mut self,
        _id: Resource<HostResponseOutparam>,
        _status: u16,
        _headers: Resource<FieldMap>,
    ) -> HttpResult<()> {
        Err(HttpError::trap(format_err!("not implemented")))
    }
}

impl types::HostIncomingResponse for WasiHttpCtxView<'_> {
    fn drop(&mut self, response: Resource<HostIncomingResponse>) -> wasmtime::Result<()> {
        let _ = self
            .table
            .delete(response)
            .context("[drop_incoming_response] deleting response")?;
        Ok(())
    }

    fn status(&mut self, response: Resource<HostIncomingResponse>) -> wasmtime::Result<StatusCode> {
        let r = self
            .table
            .get(&response)
            .context("[incoming_response_status] getting response")?;
        Ok(r.status)
    }

    fn headers(
        &mut self,
        response: Resource<HostIncomingResponse>,
    ) -> wasmtime::Result<Resource<FieldMap>> {
        let resp = self.table.get(&response)?;
        let id = self.table.push(resp.headers.clone())?;
        Ok(id)
    }

    fn consume(
        &mut self,
        response: Resource<HostIncomingResponse>,
    ) -> wasmtime::Result<Result<Resource<HostIncomingBody>, ()>> {
        let r = self
            .table
            .get_mut(&response)
            .context("[incoming_response_consume] getting response")?;

        match r.body.take() {
            Some(body) => {
                let id = self.table.push(body)?;
                Ok(Ok(id))
            }

            None => Ok(Err(())),
        }
    }
}

impl types::HostFutureTrailers for WasiHttpCtxView<'_> {
    fn drop(&mut self, id: Resource<HostFutureTrailers>) -> wasmtime::Result<()> {
        let _ = self
            .table
            .delete(id)
            .context("[drop future-trailers] deleting future-trailers")?;
        Ok(())
    }

    fn subscribe(
        &mut self,
        index: Resource<HostFutureTrailers>,
    ) -> wasmtime::Result<Resource<DynPollable>> {
        wasmtime_wasi::p2::subscribe(self.table, index)
    }

    fn get(
        &mut self,
        id: Resource<HostFutureTrailers>,
    ) -> wasmtime::Result<Option<Result<Result<Option<Resource<Trailers>>, types::ErrorCode>, ()>>>
    {
        let trailers = self.table.get_mut(&id)?;
        match trailers {
            HostFutureTrailers::Waiting { .. } => return Ok(None),
            HostFutureTrailers::Consumed => return Ok(Some(Err(()))),
            HostFutureTrailers::Done(_) => {}
        };

        let res = match std::mem::replace(trailers, HostFutureTrailers::Consumed) {
            HostFutureTrailers::Done(res) => res,
            _ => unreachable!(),
        };

        let fields = match res {
            Ok(Some(fields)) => fields,
            Ok(None) => return Ok(Some(Ok(Ok(None)))),
            Err(e) => {
                let e = self.error_to_p2(e);
                return Ok(Some(Ok(Err(e))));
            }
        };

        let ts = self
            .table
            .push(FieldMap::new_immutable(self.hooks, fields))?;

        Ok(Some(Ok(Ok(Some(ts)))))
    }
}

impl types::HostIncomingBody for WasiHttpCtxView<'_> {
    fn stream(
        &mut self,
        id: Resource<HostIncomingBody>,
    ) -> wasmtime::Result<Result<Resource<DynInputStream>, ()>> {
        let body = self.table.get_mut(&id)?;

        if let Some(stream) = body.take_stream() {
            let stream: DynInputStream = Box::new(stream);
            let stream = self.table.push_child(stream, &id)?;
            return Ok(Ok(stream));
        }

        Ok(Err(()))
    }

    fn finish(
        &mut self,
        id: Resource<HostIncomingBody>,
    ) -> wasmtime::Result<Resource<HostFutureTrailers>> {
        let body = self.table.delete(id)?;
        let trailers = self.table.push(body.into_future_trailers())?;
        Ok(trailers)
    }

    fn drop(&mut self, id: Resource<HostIncomingBody>) -> wasmtime::Result<()> {
        let _ = self.table.delete(id)?;
        Ok(())
    }
}

impl types::HostOutgoingResponse for WasiHttpCtxView<'_> {
    fn new(
        &mut self,
        headers: Resource<FieldMap>,
    ) -> wasmtime::Result<Resource<HostOutgoingResponse>> {
        let mut fields = self.table.delete(headers)?;
        fields.set_immutable();

        let id = self.table.push(HostOutgoingResponse {
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
        let buffer_chunks = self.hooks.p2_outgoing_body_buffer_chunks();
        let chunk_size = self.hooks.p2_outgoing_body_chunk_size();
        let resp = self.table.get_mut(&id)?;

        if resp.body.is_some() {
            return Ok(Err(()));
        }

        let size = match get_content_length(&resp.headers) {
            Ok(size) => size,
            Err(..) => return Ok(Err(())),
        };

        let (host, body) =
            HostOutgoingBody::new(StreamContext::Response, size, buffer_chunks, chunk_size);

        resp.body.replace(body);

        let id = self.table.push(host)?;

        Ok(Ok(id))
    }

    fn status_code(
        &mut self,
        id: Resource<HostOutgoingResponse>,
    ) -> wasmtime::Result<types::StatusCode> {
        Ok(self.table.get(&id)?.status.into())
    }

    fn set_status_code(
        &mut self,
        id: Resource<HostOutgoingResponse>,
        status: types::StatusCode,
    ) -> wasmtime::Result<Result<(), ()>> {
        let resp = self.table.get_mut(&id)?;

        match http::StatusCode::from_u16(status) {
            Ok(status) => resp.status = status,
            Err(_) => return Ok(Err(())),
        };

        Ok(Ok(()))
    }

    fn headers(
        &mut self,
        id: Resource<HostOutgoingResponse>,
    ) -> wasmtime::Result<Resource<FieldMap>> {
        let resp = self.table.get(&id)?;
        Ok(self.table.push(resp.headers.clone())?)
    }

    fn drop(&mut self, id: Resource<HostOutgoingResponse>) -> wasmtime::Result<()> {
        let _ = self.table.delete(id)?;
        Ok(())
    }
}

impl types::HostFutureIncomingResponse for WasiHttpCtxView<'_> {
    fn drop(&mut self, id: Resource<HostFutureIncomingResponse>) -> wasmtime::Result<()> {
        let _ = self.table.delete(id)?;
        Ok(())
    }

    fn get(
        &mut self,
        id: Resource<HostFutureIncomingResponse>,
    ) -> wasmtime::Result<
        Option<Result<Result<Resource<HostIncomingResponse>, types::ErrorCode>, ()>>,
    > {
        let resp = self.table.get_mut(&id)?;

        match resp {
            HostFutureIncomingResponse::Pending(_) => return Ok(None),
            HostFutureIncomingResponse::Consumed => return Ok(Some(Err(()))),
            HostFutureIncomingResponse::Ready(_) => {}
        }

        let (resp, io) =
            match std::mem::replace(resp, HostFutureIncomingResponse::Consumed).unwrap_ready() {
                Ok(pair) => pair,
                Err(e) => {
                    let e = self.error_to_p2(e);
                    return Ok(Some(Ok(Err(e))));
                }
            };

        let (parts, body) = resp.into_parts();
        let headers = FieldMap::new_immutable(self.hooks, parts.headers);

        let resp = self.table.push(HostIncomingResponse {
            status: parts.status.as_u16(),
            headers,
            body: Some({
                let mut body = HostIncomingBody::new(body);
                body.retain_worker(io);
                body
            }),
        })?;

        Ok(Some(Ok(Ok(resp))))
    }

    fn subscribe(
        &mut self,
        id: Resource<HostFutureIncomingResponse>,
    ) -> wasmtime::Result<Resource<DynPollable>> {
        wasmtime_wasi::p2::subscribe(self.table, id)
    }
}

impl types::HostOutgoingBody for WasiHttpCtxView<'_> {
    fn write(
        &mut self,
        id: Resource<HostOutgoingBody>,
    ) -> wasmtime::Result<Result<Resource<DynOutputStream>, ()>> {
        let body = self.table.get_mut(&id)?;
        if let Some(stream) = body.take_output_stream() {
            let id = self.table.push_child(stream, &id)?;
            Ok(Ok(id))
        } else {
            Ok(Err(()))
        }
    }

    fn finish(
        &mut self,
        id: Resource<HostOutgoingBody>,
        ts: Option<Resource<Trailers>>,
    ) -> HttpResult<()> {
        let body = self.table.delete(id)?;

        let ts = if let Some(ts) = ts {
            Some(self.table.delete(ts)?)
        } else {
            None
        };

        body.finish(ts)?;
        Ok(())
    }

    fn drop(&mut self, id: Resource<HostOutgoingBody>) -> wasmtime::Result<()> {
        self.table.delete(id)?.abort();
        Ok(())
    }
}

impl types::HostRequestOptions for WasiHttpCtxView<'_> {
    fn new(&mut self) -> wasmtime::Result<Resource<types::RequestOptions>> {
        let id = self.table.push(types::RequestOptions::default())?;
        Ok(id)
    }

    fn connect_timeout(
        &mut self,
        opts: Resource<types::RequestOptions>,
    ) -> wasmtime::Result<Option<types::Duration>> {
        let nanos = self.table.get(&opts)?.connect_timeout.map(|d| d.as_nanos());

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
        self.table.get_mut(&opts)?.connect_timeout = duration.map(std::time::Duration::from_nanos);
        Ok(Ok(()))
    }

    fn first_byte_timeout(
        &mut self,
        opts: Resource<types::RequestOptions>,
    ) -> wasmtime::Result<Option<types::Duration>> {
        let nanos = self
            .table
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
        self.table.get_mut(&opts)?.first_byte_timeout =
            duration.map(std::time::Duration::from_nanos);
        Ok(Ok(()))
    }

    fn between_bytes_timeout(
        &mut self,
        opts: Resource<types::RequestOptions>,
    ) -> wasmtime::Result<Option<types::Duration>> {
        let nanos = self
            .table
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
        self.table.get_mut(&opts)?.between_bytes_timeout =
            duration.map(std::time::Duration::from_nanos);
        Ok(Ok(()))
    }

    fn drop(&mut self, rep: Resource<types::RequestOptions>) -> wasmtime::Result<()> {
        let _ = self.table.delete(rep)?;
        Ok(())
    }
}

mod named {
    use crate::p2::bindings::http::types::{self, Method, Scheme, StatusCode, Trailers};
    use crate::p2::bindings::named_imports::wasi::http::types as named_types;
    use crate::p2::body::{HostFutureTrailers, HostIncomingBody, HostOutgoingBody};
    use crate::p2::types::{
        HostFutureIncomingResponse, HostIncomingRequest, HostIncomingResponse, HostOutgoingRequest,
        HostOutgoingResponse, HostResponseOutparam,
    };
    use crate::p2::{HeaderError, HeaderResult, HttpError, HttpResult};
    use crate::{FieldMap, WasiHttpNamedView};
    use wasmtime::component::Resource;
    use wasmtime_wasi::p2::{DynInputStream, DynOutputStream, DynPollable};
    use wasmtime_wasi::{NamedId, WasiCtxNamedView};

    impl<T> named_types::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn convert_error_code(&mut self, err: HttpError) -> wasmtime::Result<types::ErrorCode> {
            err.downcast()
        }

        fn convert_header_error(
            &mut self,
            err: HeaderError,
        ) -> wasmtime::Result<types::HeaderError> {
            err.downcast()
        }

        fn http_error_code(
            &mut self,
            id: NamedId,
            err: wasmtime::component::Resource<types::IoError>,
        ) -> wasmtime::Result<Option<types::ErrorCode>> {
            super::types::Host::http_error_code(&mut self.0.http(id), err)
        }
    }

    impl<T> named_types::HostFields for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn new(&mut self, id: NamedId) -> wasmtime::Result<Resource<FieldMap>> {
            super::types::HostFields::new(&mut self.0.http(id))
        }

        fn from_list(
            &mut self,
            id: NamedId,
            entries: Vec<(String, Vec<u8>)>,
        ) -> HeaderResult<Resource<FieldMap>> {
            super::types::HostFields::from_list(&mut self.0.http(id), entries)
        }

        fn drop(&mut self, id: NamedId, fields: Resource<FieldMap>) -> wasmtime::Result<()> {
            super::types::HostFields::drop(&mut self.0.http(id), fields)
        }

        fn get(
            &mut self,
            id: NamedId,
            fields: Resource<FieldMap>,
            name: String,
        ) -> wasmtime::Result<Vec<Vec<u8>>> {
            super::types::HostFields::get(&mut self.0.http(id), fields, name)
        }

        fn has(
            &mut self,
            id: NamedId,
            fields: Resource<FieldMap>,
            name: String,
        ) -> wasmtime::Result<bool> {
            super::types::HostFields::has(&mut self.0.http(id), fields, name)
        }

        fn set(
            &mut self,
            id: NamedId,
            fields: Resource<FieldMap>,
            name: String,
            values: Vec<Vec<u8>>,
        ) -> HeaderResult<()> {
            super::types::HostFields::set(&mut self.0.http(id), fields, name, values)
        }

        fn delete(
            &mut self,
            id: NamedId,
            fields: Resource<FieldMap>,
            name: String,
        ) -> HeaderResult<()> {
            super::types::HostFields::delete(&mut self.0.http(id), fields, name)
        }

        fn append(
            &mut self,
            id: NamedId,
            fields: Resource<FieldMap>,
            name: String,
            value: Vec<u8>,
        ) -> HeaderResult<()> {
            super::types::HostFields::append(&mut self.0.http(id), fields, name, value)
        }

        fn entries(
            &mut self,
            id: NamedId,
            fields: Resource<FieldMap>,
        ) -> wasmtime::Result<Vec<(String, Vec<u8>)>> {
            super::types::HostFields::entries(&mut self.0.http(id), fields)
        }

        fn clone(
            &mut self,
            id: NamedId,
            fields: Resource<FieldMap>,
        ) -> wasmtime::Result<Resource<FieldMap>> {
            super::types::HostFields::clone(&mut self.0.http(id), fields)
        }
    }

    impl<T> named_types::HostIncomingRequest for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn method(
            &mut self,
            id: NamedId,
            this: Resource<HostIncomingRequest>,
        ) -> wasmtime::Result<Method> {
            super::types::HostIncomingRequest::method(&mut self.0.http(id), this)
        }

        fn path_with_query(
            &mut self,
            id: NamedId,
            this: Resource<HostIncomingRequest>,
        ) -> wasmtime::Result<Option<String>> {
            super::types::HostIncomingRequest::path_with_query(&mut self.0.http(id), this)
        }

        fn scheme(
            &mut self,
            id: NamedId,
            this: Resource<HostIncomingRequest>,
        ) -> wasmtime::Result<Option<Scheme>> {
            super::types::HostIncomingRequest::scheme(&mut self.0.http(id), this)
        }

        fn authority(
            &mut self,
            id: NamedId,
            this: Resource<HostIncomingRequest>,
        ) -> wasmtime::Result<Option<String>> {
            super::types::HostIncomingRequest::authority(&mut self.0.http(id), this)
        }

        fn headers(
            &mut self,
            id: NamedId,
            this: Resource<HostIncomingRequest>,
        ) -> wasmtime::Result<Resource<FieldMap>> {
            super::types::HostIncomingRequest::headers(&mut self.0.http(id), this)
        }

        fn consume(
            &mut self,
            id: NamedId,
            this: Resource<HostIncomingRequest>,
        ) -> wasmtime::Result<Result<Resource<HostIncomingBody>, ()>> {
            super::types::HostIncomingRequest::consume(&mut self.0.http(id), this)
        }

        fn drop(
            &mut self,
            id: NamedId,
            this: Resource<HostIncomingRequest>,
        ) -> wasmtime::Result<()> {
            super::types::HostIncomingRequest::drop(&mut self.0.http(id), this)
        }
    }

    impl<T> named_types::HostOutgoingRequest for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn new(
            &mut self,
            id: NamedId,
            headers: Resource<FieldMap>,
        ) -> wasmtime::Result<Resource<HostOutgoingRequest>> {
            super::types::HostOutgoingRequest::new(&mut self.0.http(id), headers)
        }

        fn body(
            &mut self,
            id: NamedId,
            request: Resource<HostOutgoingRequest>,
        ) -> wasmtime::Result<Result<Resource<HostOutgoingBody>, ()>> {
            super::types::HostOutgoingRequest::body(&mut self.0.http(id), request)
        }

        fn drop(
            &mut self,
            id: NamedId,
            request: Resource<HostOutgoingRequest>,
        ) -> wasmtime::Result<()> {
            super::types::HostOutgoingRequest::drop(&mut self.0.http(id), request)
        }

        fn method(
            &mut self,
            id: NamedId,
            request: wasmtime::component::Resource<types::OutgoingRequest>,
        ) -> wasmtime::Result<Method> {
            super::types::HostOutgoingRequest::method(&mut self.0.http(id), request)
        }

        fn set_method(
            &mut self,
            id: NamedId,
            request: wasmtime::component::Resource<types::OutgoingRequest>,
            method: Method,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::types::HostOutgoingRequest::set_method(&mut self.0.http(id), request, method)
        }

        fn path_with_query(
            &mut self,
            id: NamedId,
            request: wasmtime::component::Resource<types::OutgoingRequest>,
        ) -> wasmtime::Result<Option<String>> {
            super::types::HostOutgoingRequest::path_with_query(&mut self.0.http(id), request)
        }

        fn set_path_with_query(
            &mut self,
            id: NamedId,
            request: wasmtime::component::Resource<types::OutgoingRequest>,
            path_with_query: Option<String>,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::types::HostOutgoingRequest::set_path_with_query(
                &mut self.0.http(id),
                request,
                path_with_query,
            )
        }

        fn scheme(
            &mut self,
            id: NamedId,
            request: wasmtime::component::Resource<types::OutgoingRequest>,
        ) -> wasmtime::Result<Option<Scheme>> {
            super::types::HostOutgoingRequest::scheme(&mut self.0.http(id), request)
        }

        fn set_scheme(
            &mut self,
            id: NamedId,
            request: wasmtime::component::Resource<types::OutgoingRequest>,
            scheme: Option<Scheme>,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::types::HostOutgoingRequest::set_scheme(&mut self.0.http(id), request, scheme)
        }

        fn authority(
            &mut self,
            id: NamedId,
            request: wasmtime::component::Resource<types::OutgoingRequest>,
        ) -> wasmtime::Result<Option<String>> {
            super::types::HostOutgoingRequest::authority(&mut self.0.http(id), request)
        }

        fn set_authority(
            &mut self,
            id: NamedId,
            request: wasmtime::component::Resource<types::OutgoingRequest>,
            authority: Option<String>,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::types::HostOutgoingRequest::set_authority(
                &mut self.0.http(id),
                request,
                authority,
            )
        }

        fn headers(
            &mut self,
            id: NamedId,
            request: wasmtime::component::Resource<types::OutgoingRequest>,
        ) -> wasmtime::Result<wasmtime::component::Resource<FieldMap>> {
            super::types::HostOutgoingRequest::headers(&mut self.0.http(id), request)
        }
    }

    impl<T> named_types::HostResponseOutparam for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn drop(
            &mut self,
            id: NamedId,
            this: Resource<HostResponseOutparam>,
        ) -> wasmtime::Result<()> {
            super::types::HostResponseOutparam::drop(&mut self.0.http(id), this)
        }

        fn set(
            &mut self,
            id: NamedId,
            this: Resource<HostResponseOutparam>,
            resp: Result<Resource<HostOutgoingResponse>, types::ErrorCode>,
        ) -> wasmtime::Result<()> {
            super::types::HostResponseOutparam::set(&mut self.0.http(id), this, resp)
        }

        fn send_informational(
            &mut self,
            id: NamedId,
            _id: Resource<HostResponseOutparam>,
            _status: u16,
            _headers: Resource<FieldMap>,
        ) -> HttpResult<()> {
            super::types::HostResponseOutparam::send_informational(
                &mut self.0.http(id),
                _id,
                _status,
                _headers,
            )
        }
    }

    impl<T> named_types::HostIncomingResponse for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn drop(
            &mut self,
            id: NamedId,
            response: Resource<HostIncomingResponse>,
        ) -> wasmtime::Result<()> {
            super::types::HostIncomingResponse::drop(&mut self.0.http(id), response)
        }

        fn status(
            &mut self,
            id: NamedId,
            response: Resource<HostIncomingResponse>,
        ) -> wasmtime::Result<StatusCode> {
            super::types::HostIncomingResponse::status(&mut self.0.http(id), response)
        }

        fn headers(
            &mut self,
            id: NamedId,
            response: Resource<HostIncomingResponse>,
        ) -> wasmtime::Result<Resource<FieldMap>> {
            super::types::HostIncomingResponse::headers(&mut self.0.http(id), response)
        }

        fn consume(
            &mut self,
            id: NamedId,
            response: Resource<HostIncomingResponse>,
        ) -> wasmtime::Result<Result<Resource<HostIncomingBody>, ()>> {
            super::types::HostIncomingResponse::consume(&mut self.0.http(id), response)
        }
    }

    impl<T> named_types::HostFutureTrailers for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn drop(
            &mut self,
            id: NamedId,
            this: Resource<HostFutureTrailers>,
        ) -> wasmtime::Result<()> {
            super::types::HostFutureTrailers::drop(&mut self.0.http(id), this)
        }

        fn subscribe(
            &mut self,
            id: NamedId,
            index: Resource<HostFutureTrailers>,
        ) -> wasmtime::Result<Resource<DynPollable>> {
            super::types::HostFutureTrailers::subscribe(&mut self.0.http(id), index)
        }

        fn get(
            &mut self,
            id: NamedId,
            this: Resource<HostFutureTrailers>,
        ) -> wasmtime::Result<
            Option<Result<Result<Option<Resource<Trailers>>, types::ErrorCode>, ()>>,
        > {
            super::types::HostFutureTrailers::get(&mut self.0.http(id), this)
        }
    }

    impl<T> named_types::HostIncomingBody for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn stream(
            &mut self,
            id: NamedId,
            this: Resource<HostIncomingBody>,
        ) -> wasmtime::Result<Result<Resource<DynInputStream>, ()>> {
            super::types::HostIncomingBody::stream(&mut self.0.http(id), this)
        }

        fn finish(
            &mut self,
            id: NamedId,
            this: Resource<HostIncomingBody>,
        ) -> wasmtime::Result<Resource<HostFutureTrailers>> {
            super::types::HostIncomingBody::finish(&mut self.0.http(id), this)
        }

        fn drop(&mut self, id: NamedId, this: Resource<HostIncomingBody>) -> wasmtime::Result<()> {
            super::types::HostIncomingBody::drop(&mut self.0.http(id), this)
        }
    }

    impl<T> named_types::HostOutgoingResponse for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn new(
            &mut self,
            id: NamedId,
            headers: Resource<FieldMap>,
        ) -> wasmtime::Result<Resource<HostOutgoingResponse>> {
            super::types::HostOutgoingResponse::new(&mut self.0.http(id), headers)
        }

        fn body(
            &mut self,
            id: NamedId,
            this: Resource<HostOutgoingResponse>,
        ) -> wasmtime::Result<Result<Resource<HostOutgoingBody>, ()>> {
            super::types::HostOutgoingResponse::body(&mut self.0.http(id), this)
        }

        fn status_code(
            &mut self,
            id: NamedId,
            this: Resource<HostOutgoingResponse>,
        ) -> wasmtime::Result<types::StatusCode> {
            super::types::HostOutgoingResponse::status_code(&mut self.0.http(id), this)
        }

        fn set_status_code(
            &mut self,
            id: NamedId,
            this: Resource<HostOutgoingResponse>,
            status: types::StatusCode,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::types::HostOutgoingResponse::set_status_code(&mut self.0.http(id), this, status)
        }

        fn headers(
            &mut self,
            id: NamedId,
            this: Resource<HostOutgoingResponse>,
        ) -> wasmtime::Result<Resource<FieldMap>> {
            super::types::HostOutgoingResponse::headers(&mut self.0.http(id), this)
        }

        fn drop(
            &mut self,
            id: NamedId,
            this: Resource<HostOutgoingResponse>,
        ) -> wasmtime::Result<()> {
            super::types::HostOutgoingResponse::drop(&mut self.0.http(id), this)
        }
    }

    impl<T> named_types::HostFutureIncomingResponse for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn drop(
            &mut self,
            id: NamedId,
            this: Resource<HostFutureIncomingResponse>,
        ) -> wasmtime::Result<()> {
            super::types::HostFutureIncomingResponse::drop(&mut self.0.http(id), this)
        }

        fn get(
            &mut self,
            id: NamedId,
            this: Resource<HostFutureIncomingResponse>,
        ) -> wasmtime::Result<
            Option<Result<Result<Resource<HostIncomingResponse>, types::ErrorCode>, ()>>,
        > {
            super::types::HostFutureIncomingResponse::get(&mut self.0.http(id), this)
        }

        fn subscribe(
            &mut self,
            id: NamedId,
            this: Resource<HostFutureIncomingResponse>,
        ) -> wasmtime::Result<Resource<DynPollable>> {
            super::types::HostFutureIncomingResponse::subscribe(&mut self.0.http(id), this)
        }
    }

    impl<T> named_types::HostOutgoingBody for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn write(
            &mut self,
            id: NamedId,
            this: Resource<HostOutgoingBody>,
        ) -> wasmtime::Result<Result<Resource<DynOutputStream>, ()>> {
            super::types::HostOutgoingBody::write(&mut self.0.http(id), this)
        }

        fn finish(
            &mut self,
            id: NamedId,
            this: Resource<HostOutgoingBody>,
            ts: Option<Resource<Trailers>>,
        ) -> HttpResult<()> {
            super::types::HostOutgoingBody::finish(&mut self.0.http(id), this, ts)
        }

        fn drop(&mut self, id: NamedId, this: Resource<HostOutgoingBody>) -> wasmtime::Result<()> {
            super::types::HostOutgoingBody::drop(&mut self.0.http(id), this)
        }
    }

    impl<T> named_types::HostRequestOptions for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn new(&mut self, id: NamedId) -> wasmtime::Result<Resource<types::RequestOptions>> {
            super::types::HostRequestOptions::new(&mut self.0.http(id))
        }

        fn connect_timeout(
            &mut self,
            id: NamedId,
            opts: Resource<types::RequestOptions>,
        ) -> wasmtime::Result<Option<types::Duration>> {
            super::types::HostRequestOptions::connect_timeout(&mut self.0.http(id), opts)
        }

        fn set_connect_timeout(
            &mut self,
            id: NamedId,
            opts: Resource<types::RequestOptions>,
            duration: Option<types::Duration>,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::types::HostRequestOptions::set_connect_timeout(
                &mut self.0.http(id),
                opts,
                duration,
            )
        }

        fn first_byte_timeout(
            &mut self,
            id: NamedId,
            opts: Resource<types::RequestOptions>,
        ) -> wasmtime::Result<Option<types::Duration>> {
            super::types::HostRequestOptions::first_byte_timeout(&mut self.0.http(id), opts)
        }

        fn set_first_byte_timeout(
            &mut self,
            id: NamedId,
            opts: Resource<types::RequestOptions>,
            duration: Option<types::Duration>,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::types::HostRequestOptions::set_first_byte_timeout(
                &mut self.0.http(id),
                opts,
                duration,
            )
        }

        fn between_bytes_timeout(
            &mut self,
            id: NamedId,
            opts: Resource<types::RequestOptions>,
        ) -> wasmtime::Result<Option<types::Duration>> {
            super::types::HostRequestOptions::between_bytes_timeout(&mut self.0.http(id), opts)
        }

        fn set_between_bytes_timeout(
            &mut self,
            id: NamedId,
            opts: Resource<types::RequestOptions>,
            duration: Option<types::Duration>,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::types::HostRequestOptions::set_between_bytes_timeout(
                &mut self.0.http(id),
                opts,
                duration,
            )
        }

        fn drop(
            &mut self,
            id: NamedId,
            rep: Resource<types::RequestOptions>,
        ) -> wasmtime::Result<()> {
            super::types::HostRequestOptions::drop(&mut self.0.http(id), rep)
        }
    }
}

use crate::bindings::http::types::{
    Error, Fields, FutureIncomingResponse, FutureTrailers, Headers, IncomingBody, IncomingRequest,
    IncomingResponse, Method, OutgoingBody, OutgoingRequest, OutgoingResponse, ResponseOutparam,
    Scheme, StatusCode, Trailers,
};
use crate::body::{HostFutureTrailers, HostFutureTrailersState};
use crate::types::FieldMap;
use crate::WasiHttpView;
use crate::{
    body::{HostIncomingBodyBuilder, HostOutgoingBody},
    types::{
        HostFields, HostFutureIncomingResponse, HostIncomingResponse, HostOutgoingRequest,
        TableHttpExt,
    },
};
use anyhow::{anyhow, Context};
use std::any::Any;
use wasmtime_wasi::preview2::{
    bindings::io::streams::{InputStream, OutputStream},
    bindings::poll::poll::Pollable,
    HostPollable, PollableFuture, TablePollableExt, TableStreamExt,
};
use wasmtime::component::Resource;

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
    fn drop_incoming_request(&mut self, _request: IncomingRequest) -> wasmtime::Result<()> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    fn drop_outgoing_request(&mut self, request: OutgoingRequest) -> wasmtime::Result<()> {
        self.table().delete_outgoing_request(request)?;
    async fn finish_outgoing_stream(
        &mut self,
        _s: Resource<OutgoingStream>,
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
    fn incoming_request_method(&mut self, _request: IncomingRequest) -> wasmtime::Result<Method> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    fn incoming_request_path_with_query(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Option<String>> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    fn incoming_request_scheme(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Option<Scheme>> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    fn incoming_request_authority(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Option<String>> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    fn incoming_request_headers(&mut self, _request: IncomingRequest) -> wasmtime::Result<Headers> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    fn incoming_request_consume(
        &mut self,
        _request: IncomingRequest,
    ) -> wasmtime::Result<Result<InputStream, ()>> {
        todo!("we haven't implemented the server side of wasi-http yet")
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
        let id = self
            .table()
            .push_outgoing_response(req)
            .context("[new_outgoing_request] pushing request")?;
        Ok(id)
    }
    fn outgoing_request_write(
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

        let (host_body, hyper_body) = HostOutgoingBody::new();

        req.body = Some(hyper_body);

        // The output stream will necessarily outlive the request, because we could be still
        // writing to the stream after `outgoing-handler.handle` is called.
        let outgoing_body = self.table().push_outgoing_body(host_body)?;

        Ok(Ok(outgoing_body))
    }
    fn drop_response_outparam(&mut self, _response: ResponseOutparam) -> wasmtime::Result<()> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    fn set_response_outparam(
        &mut self,
        _outparam: ResponseOutparam,
        _response: Result<OutgoingResponse, Error>,
    ) -> wasmtime::Result<()> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    fn drop_incoming_response(&mut self, response: IncomingResponse) -> wasmtime::Result<()> {
        self.table()
            .delete_incoming_response(response)
            .context("[drop_incoming_response] deleting response")?;
        Ok(())
    }
    fn drop_outgoing_response(&mut self, _response: OutgoingResponse) -> wasmtime::Result<()> {
        todo!("we haven't implemented the server side of wasi-http yet")
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
    ) -> wasmtime::Result<Result<IncomingBody, ()>> {
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

    fn future_trailers_subscribe(&mut self, index: FutureTrailers) -> wasmtime::Result<Pollable> {
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
        _status_code: StatusCode,
        _headers: Headers,
    ) -> wasmtime::Result<OutgoingResponse> {
        todo!("we haven't implemented the server side of wasi-http yet")
    }
    fn outgoing_response_write(
        &mut self,
        _response: OutgoingResponse,
    ) -> wasmtime::Result<Result<OutputStream, ()>> {
        todo!("we haven't implemented the server side of wasi-http yet")
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

    fn incoming_body_stream(
        &mut self,
        id: IncomingBody,
    ) -> wasmtime::Result<Result<Resource<InputStream>, ()>> {
        let body = self.table().get_incoming_body(id)?;

        if let Some(stream) = body.stream.take() {
            let stream = self.table().push_input_stream_child(Box::new(stream), id)?;
            return Ok(Ok(stream));
        }

        Ok(Err(()))
    }

    fn incoming_body_finish(&mut self, id: IncomingBody) -> wasmtime::Result<FutureTrailers> {
        let body = self.table().delete_incoming_body(id)?;
        let trailers = self
            .table()
            .push_future_trailers(body.into_future_trailers())?;
        Ok(trailers)
    }

    fn drop_incoming_body(&mut self, id: IncomingBody) -> wasmtime::Result<()> {
        let _ = self.table().delete_incoming_body(id)?;
        Ok(())
    }

    fn outgoing_body_write(
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

    fn outgoing_body_write_trailers(
        &mut self,
        id: OutgoingBody,
        ts: Trailers,
    ) -> wasmtime::Result<()> {
        let mut body = self.table().delete_outgoing_body(id)?;
        let trailers = self.table().get_fields(ts)?.clone();

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

    fn drop_outgoing_body(&mut self, id: OutgoingBody) -> wasmtime::Result<()> {
        let _ = self.table().delete_outgoing_body(id)?;
        Ok(())
||||||| parent of 7880fb3ac (Start adapting the wasi-http code to the new bindings.)
            .get_future(future)
            .context("[listen_to_future_incoming_response] getting future")?;
        Ok(match f.pollable_id() {
            Some(pollable_id) => pollable_id,
            None => {
                tracing::debug!("including pollable id to future incoming response");
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
                tracing::trace!("future incoming response details {:?}", *f);
                pollable_id
            }
        })
    }
}

#[cfg(feature = "sync")]
pub mod sync {
    use crate::bindings::http::types::{
        Error as AsyncError, Host as AsyncHost, Method as AsyncMethod, Scheme as AsyncScheme,
    };
    use crate::bindings::sync::http::types::{
        Error, Fields, FutureIncomingResponse, Headers, IncomingRequest, IncomingResponse,
        IncomingStream, Method, OutgoingRequest, OutgoingResponse, OutgoingStream,
        ResponseOutparam, Scheme, StatusCode, Trailers,
    };
    use crate::http_impl::WasiHttpViewExt;
    use crate::WasiHttpView;
    use wasmtime_wasi::preview2::{bindings::poll::poll::Pollable, in_tokio};

    // same boilerplate everywhere, converting between two identical types with different
    // definition sites. one day wasmtime-wit-bindgen will make all this unnecessary
    impl From<AsyncError> for Error {
        fn from(other: AsyncError) -> Self {
            match other {
                AsyncError::InvalidUrl(v) => Self::InvalidUrl(v),
                AsyncError::ProtocolError(v) => Self::ProtocolError(v),
                AsyncError::TimeoutError(v) => Self::TimeoutError(v),
                AsyncError::UnexpectedError(v) => Self::UnexpectedError(v),
            }
        }
    }

    impl From<Error> for AsyncError {
        fn from(other: Error) -> Self {
            match other {
                Error::InvalidUrl(v) => Self::InvalidUrl(v),
                Error::ProtocolError(v) => Self::ProtocolError(v),
                Error::TimeoutError(v) => Self::TimeoutError(v),
                Error::UnexpectedError(v) => Self::UnexpectedError(v),
            }
        }
    }

    impl From<AsyncMethod> for Method {
        fn from(other: AsyncMethod) -> Self {
            match other {
                AsyncMethod::Connect => Self::Connect,
                AsyncMethod::Delete => Self::Delete,
                AsyncMethod::Get => Self::Get,
                AsyncMethod::Head => Self::Head,
                AsyncMethod::Options => Self::Options,
                AsyncMethod::Patch => Self::Patch,
                AsyncMethod::Post => Self::Post,
                AsyncMethod::Put => Self::Put,
                AsyncMethod::Trace => Self::Trace,
                AsyncMethod::Other(v) => Self::Other(v),
            }
        }
    }

    impl From<Method> for AsyncMethod {
        fn from(other: Method) -> Self {
            match other {
                Method::Connect => Self::Connect,
                Method::Delete => Self::Delete,
                Method::Get => Self::Get,
                Method::Head => Self::Head,
                Method::Options => Self::Options,
                Method::Patch => Self::Patch,
                Method::Post => Self::Post,
                Method::Put => Self::Put,
                Method::Trace => Self::Trace,
                Method::Other(v) => Self::Other(v),
            }
        }
    }

    impl From<AsyncScheme> for Scheme {
        fn from(other: AsyncScheme) -> Self {
            match other {
                AsyncScheme::Http => Self::Http,
                AsyncScheme::Https => Self::Https,
                AsyncScheme::Other(v) => Self::Other(v),
            }
        }
    }

    impl From<Scheme> for AsyncScheme {
        fn from(other: Scheme) -> Self {
            match other {
                Scheme::Http => Self::Http,
                Scheme::Https => Self::Https,
                Scheme::Other(v) => Self::Other(v),
            }
        }
    }

    impl<T: WasiHttpView + WasiHttpViewExt> crate::bindings::sync::http::types::Host for T {
        fn drop_fields(&mut self, fields: Fields) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHost::drop_fields(self, fields).await })
        }
        fn new_fields(&mut self, entries: Vec<(String, String)>) -> wasmtime::Result<Fields> {
            in_tokio(async { AsyncHost::new_fields(self, entries).await })
        }
        fn fields_get(&mut self, fields: Fields, name: String) -> wasmtime::Result<Vec<Vec<u8>>> {
            in_tokio(async { AsyncHost::fields_get(self, fields, name).await })
        }
        fn fields_set(
            &mut self,
            fields: Fields,
            name: String,
            value: Vec<Vec<u8>>,
        ) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHost::fields_set(self, fields, name, value).await })
        }
        fn fields_delete(&mut self, fields: Fields, name: String) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHost::fields_delete(self, fields, name).await })
        }
        fn fields_append(
            &mut self,
            fields: Fields,
            name: String,
            value: Vec<u8>,
        ) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHost::fields_append(self, fields, name, value).await })
        }
        fn fields_entries(&mut self, fields: Fields) -> wasmtime::Result<Vec<(String, Vec<u8>)>> {
            in_tokio(async { AsyncHost::fields_entries(self, fields).await })
        }
        fn fields_clone(&mut self, fields: Fields) -> wasmtime::Result<Fields> {
            in_tokio(async { AsyncHost::fields_clone(self, fields).await })
        }
        fn finish_incoming_stream(
            &mut self,
            stream_id: IncomingStream,
        ) -> wasmtime::Result<Option<Trailers>> {
            in_tokio(async { AsyncHost::finish_incoming_stream(self, stream_id).await })
        }
        fn finish_outgoing_stream(
            &mut self,
            stream: OutgoingStream,
            trailers: Option<Trailers>,
        ) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHost::finish_outgoing_stream(self, stream, trailers).await })
        }
        fn drop_incoming_request(&mut self, request: IncomingRequest) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHost::drop_incoming_request(self, request).await })
        }
        fn drop_outgoing_request(&mut self, request: OutgoingRequest) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHost::drop_outgoing_request(self, request).await })
        }
        fn incoming_request_method(
            &mut self,
            request: IncomingRequest,
        ) -> wasmtime::Result<Method> {
            in_tokio(async { AsyncHost::incoming_request_method(self, request).await })
                .map(Method::from)
        }
        fn incoming_request_path_with_query(
            &mut self,
            request: IncomingRequest,
        ) -> wasmtime::Result<Option<String>> {
            in_tokio(async { AsyncHost::incoming_request_path_with_query(self, request).await })
        }
        fn incoming_request_scheme(
            &mut self,
            request: IncomingRequest,
        ) -> wasmtime::Result<Option<Scheme>> {
            Ok(
                in_tokio(async { AsyncHost::incoming_request_scheme(self, request).await })?
                    .map(Scheme::from),
            )
        }
        fn incoming_request_authority(
            &mut self,
            request: IncomingRequest,
        ) -> wasmtime::Result<Option<String>> {
            in_tokio(async { AsyncHost::incoming_request_authority(self, request).await })
        }
        fn incoming_request_headers(
            &mut self,
            request: IncomingRequest,
        ) -> wasmtime::Result<Headers> {
            in_tokio(async { AsyncHost::incoming_request_headers(self, request).await })
        }
        fn incoming_request_consume(
            &mut self,
            request: IncomingRequest,
        ) -> wasmtime::Result<Result<IncomingStream, ()>> {
            in_tokio(async { AsyncHost::incoming_request_consume(self, request).await })
        }
        fn new_outgoing_request(
            &mut self,
            method: Method,
            path_with_query: Option<String>,
            scheme: Option<Scheme>,
            authority: Option<String>,
            headers: Headers,
        ) -> wasmtime::Result<OutgoingRequest> {
            in_tokio(async {
                AsyncHost::new_outgoing_request(
                    self,
                    method.into(),
                    path_with_query,
                    scheme.map(AsyncScheme::from),
                    authority,
                    headers,
                )
                .await
            })
        }
        fn outgoing_request_write(
            &mut self,
            request: OutgoingRequest,
        ) -> wasmtime::Result<Result<OutgoingStream, ()>> {
            in_tokio(async { AsyncHost::outgoing_request_write(self, request).await })
        }
        fn drop_response_outparam(&mut self, response: ResponseOutparam) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHost::drop_response_outparam(self, response).await })
        }
        fn set_response_outparam(
            &mut self,
            outparam: ResponseOutparam,
            response: Result<OutgoingResponse, Error>,
        ) -> wasmtime::Result<Result<(), ()>> {
            in_tokio(async {
                AsyncHost::set_response_outparam(self, outparam, response.map_err(AsyncError::from))
                    .await
            })
        }
        fn drop_incoming_response(&mut self, response: IncomingResponse) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHost::drop_incoming_response(self, response).await })
        }
        fn drop_outgoing_response(&mut self, response: OutgoingResponse) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHost::drop_outgoing_response(self, response).await })
        }
        fn incoming_response_status(
            &mut self,
            response: IncomingResponse,
        ) -> wasmtime::Result<StatusCode> {
            in_tokio(async { AsyncHost::incoming_response_status(self, response).await })
        }
        fn incoming_response_headers(
            &mut self,
            response: IncomingResponse,
        ) -> wasmtime::Result<Headers> {
            in_tokio(async { AsyncHost::incoming_response_headers(self, response).await })
        }
        fn incoming_response_consume(
            &mut self,
            response: IncomingResponse,
        ) -> wasmtime::Result<Result<IncomingStream, ()>> {
            in_tokio(async { AsyncHost::incoming_response_consume(self, response).await })
        }
        fn new_outgoing_response(
            &mut self,
            status_code: StatusCode,
            headers: Headers,
        ) -> wasmtime::Result<OutgoingResponse> {
            in_tokio(async { AsyncHost::new_outgoing_response(self, status_code, headers).await })
        }
        fn outgoing_response_write(
            &mut self,
            response: OutgoingResponse,
        ) -> wasmtime::Result<Result<OutgoingStream, ()>> {
            in_tokio(async { AsyncHost::outgoing_response_write(self, response).await })
        }
        fn drop_future_incoming_response(
            &mut self,
            future: FutureIncomingResponse,
        ) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHost::drop_future_incoming_response(self, future).await })
        }
        fn future_incoming_response_get(
            &mut self,
            future: FutureIncomingResponse,
        ) -> wasmtime::Result<Option<Result<IncomingResponse, Error>>> {
            Ok(
                in_tokio(async { AsyncHost::future_incoming_response_get(self, future).await })?
                    .map(|v| v.map_err(Error::from)),
            )
        }
        fn listen_to_future_incoming_response(
            &mut self,
            future: FutureIncomingResponse,
        ) -> wasmtime::Result<Pollable> {
            in_tokio(async { AsyncHost::listen_to_future_incoming_response(self, future).await })
        }
    }
}

use crate::FieldMap;
use crate::p3::bindings::clocks::monotonic_clock::Duration;
use crate::p3::bindings::http::types::{
    ErrorCode, FieldName, FieldValue, Fields, HeaderError, Headers, Host, HostFields, HostRequest,
    HostRequestOptions, HostRequestWithStore, HostResponse, HostResponseWithStore, Method, Request,
    RequestOptions, RequestOptionsError, Response, Scheme, StatusCode, Trailers,
};
use crate::p3::body::Body;
use crate::p3::{HeaderResult, HttpError, RequestOptionsResult};
use crate::{WasiHttp, WasiHttpCtxView};
use std::sync::Arc;
use wasmtime::AsContextMut;
use wasmtime::component::{Access, FutureReader, Resource, ResourceTable, StreamReader};
use wasmtime::error::Context as _;

fn get_fields<'a>(
    table: &'a ResourceTable,
    fields: &Resource<Fields>,
) -> wasmtime::Result<&'a Fields> {
    table
        .get(&fields)
        .context("failed to get fields from table")
}

fn get_fields_mut<'a>(
    table: &'a mut ResourceTable,
    fields: &Resource<Fields>,
) -> HeaderResult<&'a mut Fields> {
    table
        .get_mut(&fields)
        .context("failed to get fields from table")
        .map_err(crate::p3::HeaderError::trap)
}

fn push_fields(table: &mut ResourceTable, fields: Fields) -> wasmtime::Result<Resource<Fields>> {
    table.push(fields).context("failed to push fields to table")
}

fn delete_fields(table: &mut ResourceTable, fields: Resource<Fields>) -> wasmtime::Result<Fields> {
    let mut fields = table
        .delete(fields)
        .context("failed to delete fields from table")?;
    // When fields are passed by ownership to the host that flags them as
    // immutable within `wasi:http`, and this semantically means that putting
    // fields in a request, then getting them back out, will return an immutable
    // view of the headers rather than mutable for example.
    fields.set_immutable();
    Ok(fields)
}

fn get_request<'a>(
    table: &'a ResourceTable,
    req: &Resource<Request>,
) -> wasmtime::Result<&'a Request> {
    table.get(req).context("failed to get request from table")
}

fn get_request_mut<'a>(
    table: &'a mut ResourceTable,
    req: &Resource<Request>,
) -> wasmtime::Result<&'a mut Request> {
    table
        .get_mut(req)
        .context("failed to get request from table")
}

fn get_response<'a>(
    table: &'a ResourceTable,
    res: &Resource<Response>,
) -> wasmtime::Result<&'a Response> {
    table.get(res).context("failed to get response from table")
}

fn get_response_mut<'a>(
    table: &'a mut ResourceTable,
    res: &Resource<Response>,
) -> wasmtime::Result<&'a mut Response> {
    table
        .get_mut(res)
        .context("failed to get response from table")
}

fn get_request_options<'a>(
    table: &'a ResourceTable,
    opts: &Resource<RequestOptions>,
) -> wasmtime::Result<&'a RequestOptions> {
    table
        .get(opts)
        .context("failed to get request options from table")
}

fn get_request_options_mut<'a>(
    table: &'a mut ResourceTable,
    opts: &Resource<RequestOptions>,
) -> RequestOptionsResult<&'a mut RequestOptions> {
    table
        .get_mut(opts)
        .context("failed to get request options from table")
        .map_err(crate::p3::RequestOptionsError::trap)
}

fn push_request_options(
    table: &mut ResourceTable,
    opts: RequestOptions,
) -> wasmtime::Result<Resource<RequestOptions>> {
    table
        .push(opts)
        .context("failed to push request options to table")
}

fn delete_request_options(
    table: &mut ResourceTable,
    opts: Resource<RequestOptions>,
) -> wasmtime::Result<RequestOptions> {
    table
        .delete(opts)
        .context("failed to delete request options from table")
}

impl HostFields for WasiHttpCtxView<'_> {
    fn new(&mut self) -> wasmtime::Result<Resource<Fields>> {
        push_fields(self.table, FieldMap::new_mutable(self.ctx.field_size_limit))
    }

    fn from_list(
        &mut self,
        entries: Vec<(FieldName, FieldValue)>,
    ) -> HeaderResult<Resource<Fields>> {
        let mut fields = FieldMap::new_mutable(self.ctx.field_size_limit);
        for (name, value) in entries {
            fields.append(self.hooks, name, value)?;
        }
        let fields = push_fields(self.table, fields).map_err(crate::p3::HeaderError::trap)?;
        Ok(fields)
    }

    fn get(
        &mut self,
        fields: Resource<Fields>,
        name: FieldName,
    ) -> wasmtime::Result<Vec<FieldValue>> {
        let fields = get_fields(self.table, &fields)?;
        Ok(fields
            .get_all(name)
            .into_iter()
            .map(|val| val.as_bytes().into())
            .collect())
    }

    fn has(&mut self, fields: Resource<Fields>, name: FieldName) -> wasmtime::Result<bool> {
        let fields = get_fields(self.table, &fields)?;
        Ok(fields.contains_key(name))
    }

    fn set(
        &mut self,
        fields: Resource<Fields>,
        name: FieldName,
        values: Vec<FieldValue>,
    ) -> HeaderResult<()> {
        get_fields_mut(self.table, &fields)?.set(self.hooks, name, values)?;
        Ok(())
    }

    fn delete(&mut self, fields: Resource<Fields>, name: FieldName) -> HeaderResult<()> {
        get_fields_mut(self.table, &fields)?.remove_all(self.hooks, name)?;
        Ok(())
    }

    fn get_and_delete(
        &mut self,
        fields: Resource<Fields>,
        name: FieldName,
    ) -> HeaderResult<Vec<FieldValue>> {
        let name = name.parse().or(Err(HeaderError::InvalidSyntax))?;
        let values = get_fields_mut(self.table, &fields)?
            .remove_all(self.hooks, name)?
            .into_iter();
        Ok(values.map(|value| value.as_bytes().into()).collect())
    }

    fn append(
        &mut self,
        fields: Resource<Fields>,
        name: FieldName,
        value: FieldValue,
    ) -> HeaderResult<()> {
        get_fields_mut(self.table, &fields)?.append(self.hooks, name, value)?;
        Ok(())
    }

    fn copy_all(
        &mut self,
        fields: Resource<Fields>,
    ) -> wasmtime::Result<Vec<(FieldName, FieldValue)>> {
        let fields = get_fields(self.table, &fields)?;
        let fields = fields
            .iter()
            .map(|(name, value)| (name.as_str().into(), value.as_bytes().into()))
            .collect();
        Ok(fields)
    }

    fn clone(&mut self, fields: Resource<Fields>) -> wasmtime::Result<Resource<Fields>> {
        let mut fields = get_fields(self.table, &fields)?.clone();
        fields.set_mutable(self.ctx.field_size_limit);
        push_fields(self.table, fields)
    }

    fn drop(&mut self, fields: Resource<Fields>) -> wasmtime::Result<()> {
        delete_fields(self.table, fields)?;
        Ok(())
    }
}

fn new_request<S>(
    mut store: S,
    mut getter: impl FnMut(&mut S::Data) -> WasiHttpCtxView<'_> + Copy + Send + 'static,
    headers: Resource<Headers>,
    contents: Option<StreamReader<u8>>,
    trailers: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
    options: Option<Resource<RequestOptions>>,
) -> wasmtime::Result<(Resource<Request>, FutureReader<Result<(), ErrorCode>>)>
where
    S: AsContextMut,
{
    let mut store = store.as_context_mut();
    let (body, body_result) = Body::new_guest(&mut store, getter, contents, trailers)?;
    let cx = getter(store.data_mut());
    let headers = delete_fields(cx.table, headers)?;
    let options = options
        .map(|options| delete_request_options(cx.table, options))
        .transpose()?;
    let req = Request {
        method: http::Method::GET,
        scheme: None,
        authority: None,
        path_with_query: None,
        headers,
        options: options.map(Into::into),
        body,
    };
    let req = cx
        .table
        .push(req)
        .context("failed to push request to table")?;
    Ok((req, body_result))
}

impl<T> HostRequestWithStore<T> for WasiHttp {
    fn new(
        store: Access<T, Self>,
        headers: Resource<Headers>,
        contents: Option<StreamReader<u8>>,
        trailers: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
        options: Option<Resource<RequestOptions>>,
    ) -> wasmtime::Result<(Resource<Request>, FutureReader<Result<(), ErrorCode>>)> {
        let getter = store.getter();
        new_request(store, getter, headers, contents, trailers, options)
    }

    fn consume_body(
        mut store: Access<T, Self>,
        req: Resource<Request>,
        fut: FutureReader<Result<(), ErrorCode>>,
    ) -> wasmtime::Result<(
        StreamReader<u8>,
        FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
    )> {
        let getter = store.getter();
        let Request { body, .. } = store.get().table.delete(req)?;
        body.consume(store, fut, getter)
    }

    fn drop(mut store: Access<'_, T, Self>, req: Resource<Request>) -> wasmtime::Result<()> {
        let Request { body, .. } = store.get().table.delete(req)?;
        body.drop(store)?;
        Ok(())
    }
}

impl HostRequest for WasiHttpCtxView<'_> {
    fn get_method(&mut self, req: Resource<Request>) -> wasmtime::Result<Method> {
        let Request { method, .. } = get_request(self.table, &req)?;
        Ok(method.into())
    }

    fn set_method(
        &mut self,
        req: Resource<Request>,
        method: Method,
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = get_request_mut(self.table, &req)?;
        let Ok(method) = method.try_into() else {
            return Ok(Err(()));
        };
        req.method = method;
        Ok(Ok(()))
    }

    fn get_path_with_query(&mut self, req: Resource<Request>) -> wasmtime::Result<Option<String>> {
        let Request {
            path_with_query, ..
        } = get_request(self.table, &req)?;
        Ok(path_with_query.as_ref().map(|pq| pq.as_str().into()))
    }

    fn set_path_with_query(
        &mut self,
        req: Resource<Request>,
        path_with_query: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = get_request_mut(self.table, &req)?;
        let Some(path_with_query) = path_with_query else {
            req.path_with_query = None;
            return Ok(Ok(()));
        };
        let Ok(path_with_query) = path_with_query.try_into() else {
            return Ok(Err(()));
        };
        req.path_with_query = Some(path_with_query);
        Ok(Ok(()))
    }

    fn get_scheme(&mut self, req: Resource<Request>) -> wasmtime::Result<Option<Scheme>> {
        let Request { scheme, .. } = get_request(self.table, &req)?;
        Ok(scheme.as_ref().map(Into::into))
    }

    fn set_scheme(
        &mut self,
        req: Resource<Request>,
        scheme: Option<Scheme>,
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = get_request_mut(self.table, &req)?;
        let Some(scheme) = scheme else {
            req.scheme = None;
            return Ok(Ok(()));
        };
        let Ok(scheme) = scheme.try_into() else {
            return Ok(Err(()));
        };
        req.scheme = Some(scheme);
        Ok(Ok(()))
    }

    fn get_authority(&mut self, req: Resource<Request>) -> wasmtime::Result<Option<String>> {
        let Request { authority, .. } = get_request(self.table, &req)?;
        Ok(authority.as_ref().map(|auth| auth.as_str().into()))
    }

    fn set_authority(
        &mut self,
        req: Resource<Request>,
        authority: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = get_request_mut(self.table, &req)?;
        let Some(authority) = authority else {
            req.authority = None;
            return Ok(Ok(()));
        };
        let Ok(authority) = crate::parse_authority(authority) else {
            return Ok(Err(()));
        };
        req.authority = Some(authority);
        Ok(Ok(()))
    }

    fn get_options(
        &mut self,
        req: Resource<Request>,
    ) -> wasmtime::Result<Option<Resource<RequestOptions>>> {
        let Request { options, .. } = get_request(self.table, &req)?;
        if let Some(options) = options {
            let options = push_request_options(
                self.table,
                RequestOptions::new_immutable(Arc::clone(options)),
            )?;
            Ok(Some(options))
        } else {
            Ok(None)
        }
    }

    fn get_headers(&mut self, req: Resource<Request>) -> wasmtime::Result<Resource<Headers>> {
        let Request { headers, .. } = get_request(self.table, &req)?;
        push_fields(self.table, headers.clone())
    }
}

impl HostRequestOptions for WasiHttpCtxView<'_> {
    fn new(&mut self) -> wasmtime::Result<Resource<RequestOptions>> {
        push_request_options(self.table, RequestOptions::new_mutable_default())
    }

    fn get_connect_timeout(
        &mut self,
        opts: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        let opts = get_request_options(self.table, &opts)?;
        let Some(connect_timeout) = opts.connect_timeout else {
            return Ok(None);
        };
        let ns = connect_timeout.as_nanos();
        let ns = Duration::try_from(ns)
            .context("connect timeout duration nanoseconds do not fit in u64")?;
        Ok(Some(ns))
    }

    fn set_connect_timeout(
        &mut self,
        opts: Resource<RequestOptions>,
        duration: Option<Duration>,
    ) -> RequestOptionsResult<()> {
        let opts = get_request_options_mut(self.table, &opts)?;
        let opts = opts.get_mut().ok_or(RequestOptionsError::Immutable)?;
        opts.connect_timeout = duration.map(core::time::Duration::from_nanos);
        Ok(())
    }

    fn get_first_byte_timeout(
        &mut self,
        opts: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        let opts = get_request_options(self.table, &opts)?;
        let Some(first_byte_timeout) = opts.first_byte_timeout else {
            return Ok(None);
        };
        let ns = first_byte_timeout.as_nanos();
        let ns = Duration::try_from(ns)
            .context("first byte timeout duration nanoseconds do not fit in u64")?;
        Ok(Some(ns))
    }

    fn set_first_byte_timeout(
        &mut self,
        opts: Resource<RequestOptions>,
        duration: Option<Duration>,
    ) -> RequestOptionsResult<()> {
        let opts = get_request_options_mut(self.table, &opts)?;
        let opts = opts.get_mut().ok_or(RequestOptionsError::Immutable)?;
        opts.first_byte_timeout = duration.map(core::time::Duration::from_nanos);
        Ok(())
    }

    fn get_between_bytes_timeout(
        &mut self,
        opts: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        let opts = get_request_options(self.table, &opts)?;
        let Some(between_bytes_timeout) = opts.between_bytes_timeout else {
            return Ok(None);
        };
        let ns = between_bytes_timeout.as_nanos();
        let ns = Duration::try_from(ns)
            .context("between bytes timeout duration nanoseconds do not fit in u64")?;
        Ok(Some(ns))
    }

    fn set_between_bytes_timeout(
        &mut self,
        opts: Resource<RequestOptions>,
        duration: Option<Duration>,
    ) -> RequestOptionsResult<()> {
        let opts = get_request_options_mut(self.table, &opts)?;
        let opts = opts.get_mut().ok_or(RequestOptionsError::Immutable)?;
        opts.between_bytes_timeout = duration.map(core::time::Duration::from_nanos);
        Ok(())
    }

    fn clone(
        &mut self,
        opts: Resource<RequestOptions>,
    ) -> wasmtime::Result<Resource<RequestOptions>> {
        let opts = get_request_options(self.table, &opts)?;
        push_request_options(self.table, RequestOptions::new_mutable(Arc::clone(opts)))
    }

    fn drop(&mut self, opts: Resource<RequestOptions>) -> wasmtime::Result<()> {
        delete_request_options(self.table, opts)?;
        Ok(())
    }
}

fn new_response<S>(
    mut store: S,
    mut getter: impl FnMut(&mut S::Data) -> WasiHttpCtxView<'_> + Copy + Send + 'static,
    headers: Resource<Headers>,
    contents: Option<StreamReader<u8>>,
    trailers: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
) -> wasmtime::Result<(Resource<Response>, FutureReader<Result<(), ErrorCode>>)>
where
    S: AsContextMut,
{
    let mut store = store.as_context_mut();
    let (body, body_result) = Body::new_guest(&mut store, getter, contents, trailers)?;
    let cx = getter(store.data_mut());
    let headers = delete_fields(cx.table, headers)?;
    let res = Response {
        status: http::StatusCode::OK,
        headers,
        body,
    };
    let res = cx
        .table
        .push(res)
        .context("failed to push response to table")?;
    Ok((res, body_result))
}

impl<T> HostResponseWithStore<T> for WasiHttp {
    fn new(
        store: Access<T, Self>,
        headers: Resource<Headers>,
        contents: Option<StreamReader<u8>>,
        trailers: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
    ) -> wasmtime::Result<(Resource<Response>, FutureReader<Result<(), ErrorCode>>)> {
        let getter = store.getter();
        new_response(store, getter, headers, contents, trailers)
    }

    fn consume_body(
        mut store: Access<T, Self>,
        res: Resource<Response>,
        fut: FutureReader<Result<(), ErrorCode>>,
    ) -> wasmtime::Result<(
        StreamReader<u8>,
        FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
    )> {
        let getter = store.getter();
        let Response { body, .. } = store.get().table.delete(res)?;
        body.consume(store, fut, getter)
    }

    fn drop(mut store: Access<'_, T, Self>, res: Resource<Response>) -> wasmtime::Result<()> {
        let Response { body, .. } = store.get().table.delete(res)?;
        body.drop(store)?;
        Ok(())
    }
}

impl HostResponse for WasiHttpCtxView<'_> {
    fn get_status_code(&mut self, res: Resource<Response>) -> wasmtime::Result<StatusCode> {
        let res = get_response(self.table, &res)?;
        Ok(res.status.into())
    }

    fn set_status_code(
        &mut self,
        res: Resource<Response>,
        status_code: StatusCode,
    ) -> wasmtime::Result<Result<(), ()>> {
        let res = get_response_mut(self.table, &res)?;
        match http::StatusCode::from_u16(status_code) {
            Ok(status) if matches!(status_code, 100..=599) => {
                res.status = status;
                Ok(Ok(()))
            }
            _ => Ok(Err(())),
        }
    }

    fn get_headers(&mut self, res: Resource<Response>) -> wasmtime::Result<Resource<Headers>> {
        let Response { headers, .. } = get_response(self.table, &res)?;
        push_fields(self.table, headers.clone())
    }
}

impl Host for WasiHttpCtxView<'_> {
    fn convert_error_code(&mut self, error: HttpError) -> wasmtime::Result<ErrorCode> {
        error.downcast()
    }

    fn convert_header_error(
        &mut self,
        error: crate::p3::HeaderError,
    ) -> wasmtime::Result<HeaderError> {
        error.downcast()
    }

    fn convert_request_options_error(
        &mut self,
        error: crate::p3::RequestOptionsError,
    ) -> wasmtime::Result<RequestOptionsError> {
        error.downcast()
    }
}

mod named {
    use crate::p3::bindings::clocks::monotonic_clock::Duration;
    use crate::p3::bindings::http::types::{
        ErrorCode, FieldName, FieldValue, Fields, HeaderError, Headers, Method, Request,
        RequestOptions, RequestOptionsError, Response, Scheme, StatusCode, Trailers,
    };
    use crate::p3::bindings::named_imports::wasi::http::types;
    use crate::p3::{HeaderResult, HttpError, RequestOptionsResult};
    use crate::{WasiHttpNamed, WasiHttpNamedView};
    use wasmtime::component::{Access, FutureReader, Resource, StreamReader};
    use wasmtime_wasi::{NamedId, WasiCtxNamedView};

    impl<T> types::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn convert_error_code(&mut self, error: HttpError) -> wasmtime::Result<ErrorCode> {
            error.downcast()
        }

        fn convert_header_error(
            &mut self,
            error: crate::p3::HeaderError,
        ) -> wasmtime::Result<HeaderError> {
            error.downcast()
        }

        fn convert_request_options_error(
            &mut self,
            error: crate::p3::RequestOptionsError,
        ) -> wasmtime::Result<RequestOptionsError> {
            error.downcast()
        }
    }

    impl<T> types::HostFields for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn new(&mut self, id: NamedId) -> wasmtime::Result<Resource<Fields>> {
            super::HostFields::new(&mut self.0.http(id))
        }

        fn from_list(
            &mut self,
            id: NamedId,
            entries: Vec<(FieldName, FieldValue)>,
        ) -> HeaderResult<Resource<Fields>> {
            super::HostFields::from_list(&mut self.0.http(id), entries)
        }

        fn get(
            &mut self,
            id: NamedId,
            fields: Resource<Fields>,
            name: FieldName,
        ) -> wasmtime::Result<Vec<FieldValue>> {
            super::HostFields::get(&mut self.0.http(id), fields, name)
        }

        fn has(
            &mut self,
            id: NamedId,
            fields: Resource<Fields>,
            name: FieldName,
        ) -> wasmtime::Result<bool> {
            super::HostFields::has(&mut self.0.http(id), fields, name)
        }

        fn set(
            &mut self,
            id: NamedId,
            fields: Resource<Fields>,
            name: FieldName,
            values: Vec<FieldValue>,
        ) -> HeaderResult<()> {
            super::HostFields::set(&mut self.0.http(id), fields, name, values)
        }

        fn delete(
            &mut self,
            id: NamedId,
            fields: Resource<Fields>,
            name: FieldName,
        ) -> HeaderResult<()> {
            super::HostFields::delete(&mut self.0.http(id), fields, name)
        }

        fn get_and_delete(
            &mut self,
            id: NamedId,
            fields: Resource<Fields>,
            name: FieldName,
        ) -> HeaderResult<Vec<FieldValue>> {
            super::HostFields::get_and_delete(&mut self.0.http(id), fields, name)
        }

        fn append(
            &mut self,
            id: NamedId,
            fields: Resource<Fields>,
            name: FieldName,
            value: FieldValue,
        ) -> HeaderResult<()> {
            super::HostFields::append(&mut self.0.http(id), fields, name, value)
        }

        fn copy_all(
            &mut self,
            id: NamedId,
            fields: Resource<Fields>,
        ) -> wasmtime::Result<Vec<(FieldName, FieldValue)>> {
            super::HostFields::copy_all(&mut self.0.http(id), fields)
        }

        fn clone(
            &mut self,
            id: NamedId,
            fields: Resource<Fields>,
        ) -> wasmtime::Result<Resource<Fields>> {
            super::HostFields::clone(&mut self.0.http(id), fields)
        }

        fn drop(&mut self, id: NamedId, fields: Resource<Fields>) -> wasmtime::Result<()> {
            super::HostFields::drop(&mut self.0.http(id), fields)
        }
    }

    impl<T> types::HostRequest for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn get_method(&mut self, id: NamedId, req: Resource<Request>) -> wasmtime::Result<Method> {
            super::HostRequest::get_method(&mut self.0.http(id), req)
        }

        fn set_method(
            &mut self,
            id: NamedId,
            req: Resource<Request>,
            method: Method,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::HostRequest::set_method(&mut self.0.http(id), req, method)
        }

        fn get_path_with_query(
            &mut self,
            id: NamedId,
            req: Resource<Request>,
        ) -> wasmtime::Result<Option<String>> {
            super::HostRequest::get_path_with_query(&mut self.0.http(id), req)
        }

        fn set_path_with_query(
            &mut self,
            id: NamedId,
            req: Resource<Request>,
            path_with_query: Option<String>,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::HostRequest::set_path_with_query(&mut self.0.http(id), req, path_with_query)
        }

        fn get_scheme(
            &mut self,
            id: NamedId,
            req: Resource<Request>,
        ) -> wasmtime::Result<Option<Scheme>> {
            super::HostRequest::get_scheme(&mut self.0.http(id), req)
        }

        fn set_scheme(
            &mut self,
            id: NamedId,
            req: Resource<Request>,
            scheme: Option<Scheme>,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::HostRequest::set_scheme(&mut self.0.http(id), req, scheme)
        }

        fn get_authority(
            &mut self,
            id: NamedId,
            req: Resource<Request>,
        ) -> wasmtime::Result<Option<String>> {
            super::HostRequest::get_authority(&mut self.0.http(id), req)
        }

        fn set_authority(
            &mut self,
            id: NamedId,
            req: Resource<Request>,
            authority: Option<String>,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::HostRequest::set_authority(&mut self.0.http(id), req, authority)
        }

        fn get_options(
            &mut self,
            id: NamedId,
            req: Resource<Request>,
        ) -> wasmtime::Result<Option<Resource<RequestOptions>>> {
            super::HostRequest::get_options(&mut self.0.http(id), req)
        }

        fn get_headers(
            &mut self,
            id: NamedId,
            req: Resource<Request>,
        ) -> wasmtime::Result<Resource<Headers>> {
            super::HostRequest::get_headers(&mut self.0.http(id), req)
        }
    }

    impl<T> types::HostRequestOptions for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn new(&mut self, id: NamedId) -> wasmtime::Result<Resource<RequestOptions>> {
            super::HostRequestOptions::new(&mut self.0.http(id))
        }

        fn get_connect_timeout(
            &mut self,
            id: NamedId,
            opts: Resource<RequestOptions>,
        ) -> wasmtime::Result<Option<Duration>> {
            super::HostRequestOptions::get_connect_timeout(&mut self.0.http(id), opts)
        }

        fn set_connect_timeout(
            &mut self,
            id: NamedId,
            opts: Resource<RequestOptions>,
            duration: Option<Duration>,
        ) -> RequestOptionsResult<()> {
            super::HostRequestOptions::set_connect_timeout(&mut self.0.http(id), opts, duration)
        }

        fn get_first_byte_timeout(
            &mut self,
            id: NamedId,
            opts: Resource<RequestOptions>,
        ) -> wasmtime::Result<Option<Duration>> {
            super::HostRequestOptions::get_first_byte_timeout(&mut self.0.http(id), opts)
        }

        fn set_first_byte_timeout(
            &mut self,
            id: NamedId,
            opts: Resource<RequestOptions>,
            duration: Option<Duration>,
        ) -> RequestOptionsResult<()> {
            super::HostRequestOptions::set_first_byte_timeout(&mut self.0.http(id), opts, duration)
        }

        fn get_between_bytes_timeout(
            &mut self,
            id: NamedId,
            opts: Resource<RequestOptions>,
        ) -> wasmtime::Result<Option<Duration>> {
            super::HostRequestOptions::get_between_bytes_timeout(&mut self.0.http(id), opts)
        }

        fn set_between_bytes_timeout(
            &mut self,
            id: NamedId,
            opts: Resource<RequestOptions>,
            duration: Option<Duration>,
        ) -> RequestOptionsResult<()> {
            super::HostRequestOptions::set_between_bytes_timeout(
                &mut self.0.http(id),
                opts,
                duration,
            )
        }

        fn clone(
            &mut self,
            id: NamedId,
            opts: Resource<RequestOptions>,
        ) -> wasmtime::Result<Resource<RequestOptions>> {
            super::HostRequestOptions::clone(&mut self.0.http(id), opts)
        }

        fn drop(&mut self, id: NamedId, opts: Resource<RequestOptions>) -> wasmtime::Result<()> {
            super::HostRequestOptions::drop(&mut self.0.http(id), opts)
        }
    }

    impl<T> types::HostResponse for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn get_status_code(
            &mut self,
            id: NamedId,
            res: Resource<Response>,
        ) -> wasmtime::Result<StatusCode> {
            super::HostResponse::get_status_code(&mut self.0.http(id), res)
        }

        fn set_status_code(
            &mut self,
            id: NamedId,
            res: Resource<Response>,
            status_code: StatusCode,
        ) -> wasmtime::Result<Result<(), ()>> {
            super::HostResponse::set_status_code(&mut self.0.http(id), res, status_code)
        }

        fn get_headers(
            &mut self,
            id: NamedId,
            res: Resource<Response>,
        ) -> wasmtime::Result<Resource<Headers>> {
            super::HostResponse::get_headers(&mut self.0.http(id), res)
        }
    }

    impl<T, U> types::HostRequestWithStore<U> for WasiHttpNamed<T>
    where
        T: WasiHttpNamedView,
        U: 'static,
    {
        fn new(
            store: Access<U, Self>,
            id: NamedId,
            headers: Resource<Headers>,
            contents: Option<StreamReader<u8>>,
            trailers: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
            options: Option<Resource<RequestOptions>>,
        ) -> wasmtime::Result<(Resource<Request>, FutureReader<Result<(), ErrorCode>>)> {
            let getter = store.getter();
            super::new_request(
                store,
                move |data| getter(data).0.http(id),
                headers,
                contents,
                trailers,
                options,
            )
        }

        fn consume_body(
            mut store: Access<U, Self>,
            id: NamedId,
            req: Resource<Request>,
            fut: FutureReader<Result<(), ErrorCode>>,
        ) -> wasmtime::Result<(
            StreamReader<u8>,
            FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
        )> {
            let getter = store.getter();
            let Request { body, .. } = store.get().0.http(id).table.delete(req)?;
            body.consume(store, fut, move |data: &mut U| getter(data).0.http(id))
        }

        fn drop(
            mut store: Access<'_, U, Self>,
            id: NamedId,
            req: Resource<Request>,
        ) -> wasmtime::Result<()> {
            let Request { body, .. } = store.get().0.http(id).table.delete(req)?;
            body.drop(store)?;
            Ok(())
        }
    }

    impl<T, U> types::HostResponseWithStore<U> for WasiHttpNamed<T>
    where
        T: WasiHttpNamedView,
        U: 'static,
    {
        fn new(
            store: Access<U, Self>,
            id: NamedId,
            headers: Resource<Headers>,
            contents: Option<StreamReader<u8>>,
            trailers: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
        ) -> wasmtime::Result<(Resource<Response>, FutureReader<Result<(), ErrorCode>>)> {
            let getter = store.getter();
            super::new_response(
                store,
                move |data| getter(data).0.http(id),
                headers,
                contents,
                trailers,
            )
        }

        fn consume_body(
            mut store: Access<U, Self>,
            id: NamedId,
            res: Resource<Response>,
            fut: FutureReader<Result<(), ErrorCode>>,
        ) -> wasmtime::Result<(
            StreamReader<u8>,
            FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
        )> {
            let getter = store.getter();
            let Response { body, .. } = store.get().0.http(id).table.delete(res)?;
            body.consume(store, fut, move |data: &mut U| getter(data).0.http(id))
        }

        fn drop(
            mut store: Access<'_, U, Self>,
            id: NamedId,
            res: Resource<Response>,
        ) -> wasmtime::Result<()> {
            let Response { body, .. } = store.get().0.http(id).table.delete(res)?;
            body.drop(store)?;
            Ok(())
        }
    }
}

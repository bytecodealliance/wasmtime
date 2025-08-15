use crate::p3::bindings::clocks::monotonic_clock::Duration;
use crate::p3::bindings::http::types::{
    ErrorCode, FieldName, FieldValue, Fields, HeaderError, Headers, Host, HostFields, HostRequest,
    HostRequestOptions, HostRequestWithStore, HostResponse, HostResponseWithStore, Method, Request,
    RequestOptions, RequestOptionsError, Response, Scheme, StatusCode, Trailers,
};
use crate::p3::{WasiHttp, WasiHttpCtxView};
use anyhow::bail;
use wasmtime::component::{Accessor, FutureReader, Resource, StreamReader};

impl HostFields for WasiHttpCtxView<'_> {
    fn new(&mut self) -> wasmtime::Result<Resource<Fields>> {
        bail!("TODO")
    }

    fn from_list(
        &mut self,
        entries: Vec<(FieldName, FieldValue)>,
    ) -> wasmtime::Result<Result<Resource<Fields>, HeaderError>> {
        bail!("TODO")
    }

    fn get(
        &mut self,
        self_: Resource<Fields>,
        name: FieldName,
    ) -> wasmtime::Result<Vec<FieldValue>> {
        bail!("TODO")
    }

    fn has(&mut self, self_: Resource<Fields>, name: FieldName) -> wasmtime::Result<bool> {
        bail!("TODO")
    }

    fn set(
        &mut self,
        self_: Resource<Fields>,
        name: FieldName,
        value: Vec<FieldValue>,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        bail!("TODO")
    }

    fn delete(
        &mut self,
        self_: Resource<Fields>,
        name: FieldName,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        bail!("TODO")
    }

    fn get_and_delete(
        &mut self,
        self_: Resource<Fields>,
        name: FieldName,
    ) -> wasmtime::Result<Result<Vec<FieldValue>, HeaderError>> {
        bail!("TODO")
    }

    fn append(
        &mut self,
        self_: Resource<Fields>,
        name: FieldName,
        value: FieldValue,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        bail!("TODO")
    }

    fn copy_all(
        &mut self,
        self_: Resource<Fields>,
    ) -> wasmtime::Result<Vec<(FieldName, FieldValue)>> {
        bail!("TODO")
    }

    fn clone(&mut self, self_: Resource<Fields>) -> wasmtime::Result<Resource<Fields>> {
        bail!("TODO")
    }

    fn drop(&mut self, rep: Resource<Fields>) -> wasmtime::Result<()> {
        bail!("TODO")
    }
}

impl HostRequestWithStore for WasiHttp {
    async fn new<U>(
        store: &Accessor<U, Self>,
        headers: Resource<Headers>,
        contents: Option<StreamReader<u8>>,
        trailers: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
        options: Option<Resource<RequestOptions>>,
    ) -> wasmtime::Result<(Resource<Request>, FutureReader<Result<(), ErrorCode>>)> {
        bail!("TODO")
    }
}

impl HostRequest for WasiHttpCtxView<'_> {
    fn get_method(&mut self, self_: Resource<Request>) -> wasmtime::Result<Method> {
        bail!("TODO")
    }

    fn set_method(
        &mut self,
        self_: Resource<Request>,
        method: Method,
    ) -> wasmtime::Result<Result<(), ()>> {
        bail!("TODO")
    }

    fn get_path_with_query(
        &mut self,
        self_: Resource<Request>,
    ) -> wasmtime::Result<Option<String>> {
        bail!("TODO")
    }

    fn set_path_with_query(
        &mut self,
        self_: Resource<Request>,
        path_with_query: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        bail!("TODO")
    }

    fn get_scheme(&mut self, self_: Resource<Request>) -> wasmtime::Result<Option<Scheme>> {
        bail!("TODO")
    }

    fn set_scheme(
        &mut self,
        self_: Resource<Request>,
        scheme: Option<Scheme>,
    ) -> wasmtime::Result<Result<(), ()>> {
        bail!("TODO")
    }

    fn get_authority(&mut self, self_: Resource<Request>) -> wasmtime::Result<Option<String>> {
        bail!("TODO")
    }

    fn set_authority(
        &mut self,
        self_: Resource<Request>,
        authority: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        bail!("TODO")
    }

    fn get_options(
        &mut self,
        self_: Resource<Request>,
    ) -> wasmtime::Result<Option<Resource<RequestOptions>>> {
        bail!("TODO")
    }

    fn get_headers(&mut self, self_: Resource<Request>) -> wasmtime::Result<Resource<Headers>> {
        bail!("TODO")
    }

    fn consume_body(
        &mut self,
        self_: Resource<Request>,
    ) -> wasmtime::Result<
        Result<
            (
                StreamReader<u8>,
                FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
            ),
            (),
        >,
    > {
        bail!("TODO")
    }

    fn drop(&mut self, rep: Resource<Request>) -> wasmtime::Result<()> {
        bail!("TODO")
    }
}

impl HostRequestOptions for WasiHttpCtxView<'_> {
    fn new(&mut self) -> wasmtime::Result<Resource<RequestOptions>> {
        bail!("TODO")
    }

    fn get_connect_timeout(
        &mut self,
        self_: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        bail!("TODO")
    }

    fn set_connect_timeout(
        &mut self,
        self_: Resource<RequestOptions>,
        duration: Option<Duration>,
    ) -> wasmtime::Result<Result<(), RequestOptionsError>> {
        bail!("TODO")
    }

    fn get_first_byte_timeout(
        &mut self,
        self_: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        bail!("TODO")
    }

    fn set_first_byte_timeout(
        &mut self,
        self_: Resource<RequestOptions>,
        duration: Option<Duration>,
    ) -> wasmtime::Result<Result<(), RequestOptionsError>> {
        bail!("TODO")
    }

    fn get_between_bytes_timeout(
        &mut self,
        self_: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        bail!("TODO")
    }

    fn set_between_bytes_timeout(
        &mut self,
        self_: Resource<RequestOptions>,
        duration: Option<Duration>,
    ) -> wasmtime::Result<Result<(), RequestOptionsError>> {
        bail!("TODO")
    }

    fn clone(
        &mut self,
        self_: Resource<RequestOptions>,
    ) -> wasmtime::Result<Resource<RequestOptions>> {
        bail!("TODO")
    }

    fn drop(&mut self, rep: Resource<RequestOptions>) -> wasmtime::Result<()> {
        bail!("TODO")
    }
}

impl HostResponseWithStore for WasiHttp {
    async fn new<U>(
        store: &Accessor<U, Self>,
        headers: Resource<Headers>,
        contents: Option<StreamReader<u8>>,
        trailers: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
    ) -> wasmtime::Result<(Resource<Response>, FutureReader<Result<(), ErrorCode>>)> {
        bail!("TODO")
    }
}

impl HostResponse for WasiHttpCtxView<'_> {
    fn get_status_code(&mut self, self_: Resource<Response>) -> wasmtime::Result<StatusCode> {
        bail!("TODO")
    }

    fn set_status_code(
        &mut self,
        self_: Resource<Response>,
        status_code: StatusCode,
    ) -> wasmtime::Result<Result<(), ()>> {
        bail!("TODO")
    }

    fn get_headers(&mut self, self_: Resource<Response>) -> wasmtime::Result<Resource<Headers>> {
        bail!("TODO")
    }

    fn consume_body(
        &mut self,
        self_: Resource<Response>,
    ) -> wasmtime::Result<
        Result<
            (
                StreamReader<u8>,
                FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
            ),
            (),
        >,
    > {
        bail!("TODO")
    }

    fn drop(&mut self, rep: Resource<Response>) -> wasmtime::Result<()> {
        bail!("TODO")
    }
}

impl Host for WasiHttpCtxView<'_> {}

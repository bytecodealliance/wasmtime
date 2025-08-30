use crate::p3::bindings::http::handler::{Host, HostWithStore};
use crate::p3::bindings::http::types::{ErrorCode, Request, Response};
use crate::p3::{WasiHttp, WasiHttpCtxView};
use anyhow::bail;
use wasmtime::component::{Accessor, Resource};

impl HostWithStore for WasiHttp {
    #[expect(unused, reason = "work in progress")] // TODO: implement
    async fn handle<T>(
        store: &Accessor<T, Self>,
        request: Resource<Request>,
    ) -> wasmtime::Result<Result<Resource<Response>, ErrorCode>> {
        bail!("TODO")
    }
}

impl Host for WasiHttpCtxView<'_> {}

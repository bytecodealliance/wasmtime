use crate::p3::bindings::http::handler::{Host, HostWithStore};
use crate::p3::bindings::http::types::{Request, Response};
use crate::p3::{HttpResult, WasiHttp, WasiHttpCtxView};
use wasmtime::component::{Accessor, Resource};

impl HostWithStore for WasiHttp {
    #[expect(unused, reason = "work in progress")] // TODO: implement
    async fn handle<T>(
        store: &Accessor<T, Self>,
        req: Resource<Request>,
    ) -> HttpResult<Resource<Response>> {
        todo!()
    }
}

impl Host for WasiHttpCtxView<'_> {}

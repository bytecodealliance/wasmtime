use crate::p3::WasiHttpView;
use crate::p3::bindings::Proxy;
use crate::p3::bindings::http::types::{ErrorCode, Request, Response};
use anyhow::Context as _;
use wasmtime::component::Accessor;

impl Proxy {
    /// Call `wasi:http/handler#handle` on [Proxy] getting a [Response] back.
    pub async fn handle(
        &self,
        store: &Accessor<impl WasiHttpView>,
        req: impl Into<Request>,
    ) -> wasmtime::Result<Result<Response, ErrorCode>> {
        let req = store.with(|mut store| {
            store
                .data_mut()
                .http()
                .table
                .push(req.into())
                .context("failed to push request to table")
        })?;
        match self.wasi_http_handler().call_handle(store, req).await? {
            Ok(res) => {
                let res = store.with(|mut store| {
                    store
                        .data_mut()
                        .http()
                        .table
                        .delete(res)
                        .context("failed to delete response from table")
                })?;
                Ok(Ok(res))
            }
            Err(err) => Ok(Err(err)),
        }
    }
}

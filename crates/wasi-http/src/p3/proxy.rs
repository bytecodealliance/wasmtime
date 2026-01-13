use crate::p3::WasiHttpView;
use crate::p3::bindings::Service;
use crate::p3::bindings::http::types::{ErrorCode, Request, Response};
use anyhow::Context as _;
use wasmtime::component::{Accessor, TaskExit};

impl Service {
    /// Call `wasi:http/handler#handle` on [Service] getting a [Response] back.
    pub async fn handle(
        &self,
        store: &Accessor<impl WasiHttpView>,
        req: impl Into<Request>,
    ) -> wasmtime::Result<Result<(Response, TaskExit), ErrorCode>> {
        let req = store.with(|mut store| {
            store
                .data_mut()
                .http()
                .table
                .push(req.into())
                .context("failed to push request to table")
        })?;
        match self.wasi_http_handler().call_handle(store, req).await? {
            (Ok(res), task) => {
                let res = store.with(|mut store| {
                    store
                        .data_mut()
                        .http()
                        .table
                        .delete(res)
                        .context("failed to delete response from table")
                })?;
                Ok(Ok((res, task)))
            }
            (Err(err), _) => Ok(Err(err)),
        }
    }
}

use crate::p3::bindings::http::client::{Host, HostWithStore};
use crate::p3::bindings::http::types::{ErrorCode, Request, Response};
use crate::p3::body::{Body, BodyExt as _};
use crate::p3::{HttpError, HttpResult, WasiHttp, WasiHttpCtxView};
use anyhow::Context as _;
use core::task::{Context, Poll, Waker};
use http_body_util::BodyExt as _;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::task::{self, JoinHandle};
use tracing::debug;
use wasmtime::component::{Accessor, Resource};

/// A wrapper around [`JoinHandle`], which will [`JoinHandle::abort`] the task
/// when dropped
struct AbortOnDropJoinHandle(JoinHandle<()>);

impl Drop for AbortOnDropJoinHandle {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn io_task_result(
    rx: oneshot::Receiver<(
        Arc<AbortOnDropJoinHandle>,
        oneshot::Receiver<Result<(), ErrorCode>>,
    )>,
) -> Result<(), ErrorCode> {
    let Ok((_io, io_result_rx)) = rx.await else {
        return Ok(());
    };
    io_result_rx.await.unwrap_or(Ok(()))
}

impl HostWithStore for WasiHttp {
    async fn send<T>(
        store: &Accessor<T, Self>,
        req: Resource<Request>,
    ) -> HttpResult<Resource<Response>> {
        // A handle to the I/O task, if spawned, will be sent on this channel
        // and kept as part of request body state
        let (io_task_tx, io_task_rx) = oneshot::channel();

        // A handle to the I/O task, if spawned, will be sent on this channel
        // along with the result receiver
        let (io_result_tx, io_result_rx) = oneshot::channel();

        // Response processing result will be sent on this channel
        let (res_result_tx, res_result_rx) = oneshot::channel();

        let getter = store.getter();
        let fut = store.with(|mut store| {
            let WasiHttpCtxView { table, .. } = store.get();
            let req = table
                .delete(req)
                .context("failed to delete request from table")
                .map_err(HttpError::trap)?;
            let (req, options) =
                req.into_http_with_getter(&mut store, io_task_result(io_result_rx), getter)?;
            HttpResult::Ok(store.get().ctx.send_request(
                req.map(|body| body.with_state(io_task_rx).boxed_unsync()),
                options.as_deref().copied(),
                Box::new(async {
                    // Forward the response processing result to `WasiHttpCtx` implementation
                    let Ok(fut) = res_result_rx.await else {
                        return Ok(());
                    };
                    Box::into_pin(fut).await
                }),
            ))
        })?;
        let (res, io) = Box::into_pin(fut).await?;
        let (
            http::response::Parts {
                status, headers, ..
            },
            body,
        ) = res.into_parts();

        let mut io = Box::into_pin(io);
        let body = match io.as_mut().poll(&mut Context::from_waker(Waker::noop()))? {
            Poll::Ready(()) => body,
            Poll::Pending => {
                // I/O driver still needs to be polled, spawn a task and send handles to it
                let (tx, rx) = oneshot::channel();
                let io = task::spawn(async move {
                    let res = io.await;
                    debug!(?res, "`send_request` I/O future finished");
                    _ = tx.send(res);
                });
                let io = Arc::new(AbortOnDropJoinHandle(io));
                _ = io_result_tx.send((Arc::clone(&io), rx));
                _ = io_task_tx.send(Arc::clone(&io));
                body.with_state(io).boxed_unsync()
            }
        };
        let res = Response {
            status,
            headers: Arc::new(headers),
            body: Body::Host {
                body,
                result_tx: res_result_tx,
            },
        };
        store.with(|mut store| {
            store
                .get()
                .table
                .push(res)
                .context("failed to push response to table")
                .map_err(HttpError::trap)
        })
    }
}

impl Host for WasiHttpCtxView<'_> {}

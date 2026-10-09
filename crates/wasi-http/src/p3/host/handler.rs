use crate::FieldMap;
use crate::p3::bindings::http::client::{Host, HostWithStore};
use crate::p3::bindings::http::types::{Request, Response};
use crate::p3::body::{Body, BodyExt as _};
use crate::p3::{HttpError, HttpResult};
use crate::{Error, WasiHttp, WasiHttpCtxView};
use core::task::{Context, Poll, Waker};
use http_body_util::BodyExt as _;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::task::AbortHandle;
use tracing::debug;
use wasmtime::AsContextMut as _;
use wasmtime::component::{Accessor, HasData, Resource};
use wasmtime::error::Context as _;
use wasmtime_wasi::runtime::AbortOnDropJoinHandle;

/// A wrapper around [`AbortHandle`], which will [`AbortHandle::abort`] the task
/// when dropped
struct AbortOnDropHandle(AbortHandle);

impl Drop for AbortOnDropHandle {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Own an I/O task and allow it to finish during the Store's shutdown grace period.
struct DelayedAbortOnDropHandle {
    inner: Option<AbortOnDropJoinHandle<()>>,
    tx: Option<oneshot::Sender<AbortOnDropJoinHandle<()>>>,
}

impl DelayedAbortOnDropHandle {
    fn new(
        handle: wasmtime_wasi::runtime::AbortOnDropJoinHandle<()>,
        timeout: std::time::Duration,
    ) -> Self {
        let tx = if timeout.is_zero() {
            None
        } else {
            let (tx, rx) = oneshot::channel::<AbortOnDropJoinHandle<()>>();
            wasmtime_wasi::runtime::with_ambient_tokio_runtime(|| {
                tokio::spawn(async move {
                    let Ok(handle) = rx.await else { return };
                    if !handle.is_finished() {
                        // We don't care if the task completes in the deadline
                        // or not.
                        let _ = tokio::time::timeout(timeout, handle).await;
                    }
                });
            });
            Some(tx)
        };
        Self {
            inner: Some(handle),
            tx,
        }
    }
}

impl Future for DelayedAbortOnDropHandle {
    type Output = Result<(), tokio::task::JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(task) = self.inner.as_mut() else {
            return Poll::Ready(Ok(()));
        };
        let result = Pin::new(&mut **task).poll(cx);
        if result.is_ready() {
            // The task has finished, so dropping this wrapper needs no timer.
            drop(self.inner.take());
        }
        result
    }
}

impl Drop for DelayedAbortOnDropHandle {
    fn drop(&mut self) {
        let Some(inner) = self.inner.take() else {
            return;
        };
        // Try sending the handle down the channel to be timed out.
        // We took ownership of inner so it will be dropped (and cancelled) if
        // the channel doesn't exist or sending fails.
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(inner);
        };
    }
}

const DROPPED_FUTURE_ERROR: &str =
    "Future indicating transmission result dropped without being resolved.";

async fn io_task_result(
    rx: oneshot::Receiver<(
        Option<Arc<AbortOnDropHandle>>,
        oneshot::Receiver<Result<(), Error>>,
    )>,
) -> Result<(), Error> {
    let Ok((_io, io_result_rx)) = rx.await else {
        return Err(Error::InternalError(Some(DROPPED_FUTURE_ERROR.to_string())));
    };
    io_result_rx
        .await
        .unwrap_or_else(|_| Err(Error::InternalError(Some(DROPPED_FUTURE_ERROR.to_string()))))
}

fn send_dummy_io(
    result: Result<(), Error>,
    io_result_tx: oneshot::Sender<(
        Option<Arc<AbortOnDropHandle>>,
        oneshot::Receiver<Result<(), Error>>,
    )>,
) {
    let (tx, rx) = oneshot::channel();
    let _ = tx.send(result);
    let _ = io_result_tx.send((None, rx));
}

fn send_dummy_io_err<T, D>(
    store: &Accessor<T, D>,
    mut getter: impl FnMut(&mut T) -> WasiHttpCtxView<'_>,
    e: Error,
    io_result_tx: oneshot::Sender<(
        Option<Arc<AbortOnDropHandle>>,
        oneshot::Receiver<Result<(), Error>>,
    )>,
) -> HttpError
where
    D: HasData,
{
    let err_code =
        store.with(|mut store| getter(store.as_context_mut().data_mut()).error_to_p3(&e));
    send_dummy_io(Err(e), io_result_tx);
    err_code.into()
}

impl<T> HostWithStore<T> for WasiHttp {
    async fn send(
        store: &Accessor<T, Self>,
        req: Resource<Request>,
    ) -> HttpResult<Resource<Response>> {
        let getter = store.getter();
        send(store, getter, req).await
    }
}

async fn send<T, D>(
    store: &Accessor<T, D>,
    mut getter: impl FnMut(&mut T) -> WasiHttpCtxView<'_> + Copy + Unpin + Send + 'static,
    req: Resource<Request>,
) -> HttpResult<Resource<Response>>
where
    D: HasData,
    T: 'static,
{
    // An abort handle to the I/O task, if spawned, will be sent on this channel
    // and kept as part of request body state
    let (request_body_io_tx, request_body_io_rx) = oneshot::channel();

    // An abort handle to the I/O task, if spawned, will be sent on this channel
    // along with the result receiver
    let (transmission_fut_io_tx, transmission_fut_io_rx) = oneshot::channel();

    // Response processing result will be sent on this channel
    let (res_result_tx, res_result_rx) = oneshot::channel();

    let fut = store.with(|mut store| {
        let WasiHttpCtxView { table, .. } = getter(store.data_mut());
        let req = table
            .delete(req)
            .context("failed to delete request from table")
            .map_err(HttpError::trap)?;
        let (req, options) =
            req.into_http_with_getter(&mut store, io_task_result(transmission_fut_io_rx), getter)?;
        HttpResult::Ok(getter(store.data_mut()).hooks.send_request(
            // Attach a reference to the io task to the body so that the task
            // can be canceled if the body is dropped and all other references
            // are dropped.
            req.map(|body| body.with_state(request_body_io_rx).boxed_unsync()),
            options.as_deref().copied(),
            Box::new(async {
                // Forward the response processing result to `WasiHttpCtx` implementation
                let Ok(fut) = res_result_rx.await else {
                    return Ok(());
                };
                Box::into_pin(fut).await
            }),
        ))
    });
    let fut = match fut {
        Ok(fut) => fut,
        Err(e) => match e.downcast() {
            Ok(err_code) => {
                send_dummy_io(Err(err_code.clone().into()), transmission_fut_io_tx);
                return Err(err_code.into());
            }
            Err(e) => {
                let e = Error::InternalError(Some(format!("{e}")));
                return Err(send_dummy_io_err(store, getter, e, transmission_fut_io_tx));
            }
        },
    };
    let (res, io) = match Box::into_pin(fut).await {
        Ok(r) => r,
        Err(e) => {
            return Err(send_dummy_io_err(store, getter, e, transmission_fut_io_tx));
        }
    };
    let (
        http::response::Parts {
            status, headers, ..
        },
        body,
    ) = res.into_parts();

    let mut io = Box::into_pin(io);
    let body = match io.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(Ok(())) => {
            send_dummy_io(Ok(()), transmission_fut_io_tx);
            body
        }
        Poll::Ready(Err(e)) => {
            return Err(send_dummy_io_err(store, getter, e, transmission_fut_io_tx));
        }
        Poll::Pending => {
            // I/O driver still needs to be polled, spawn a task and send handles to it
            let (tx, rx) = oneshot::channel();
            let shutdown_timeout = store.with(|mut store| {
                getter(store.data_mut())
                    .ctx
                    .spawned_task_shutdown_grace_period
            });
            // `task` is a tokio task which will be owned by the `Store` so
            // that the task is cancelled if the `Store` is dropped. The
            // outgoing body, transmission future, and incoming response body
            // will all hold reference references to an abort handle on the task
            // so that it is aborted when all three are dropped. But they will
            // not retain owneship of the task, so they cannot keep it alive
            // after the `Store` has dropped.
            let task = wasmtime_wasi::runtime::spawn(async move {
                let res = io.await;
                debug!(?res, "`send_request` I/O future finished");
                _ = tx.send(res);
            });
            // `task` will be aborted when there are no more references to `io`.
            let io = Arc::new(AbortOnDropHandle(task.abort_handle()));
            let task = DelayedAbortOnDropHandle::new(task, shutdown_timeout);
            // Pass ownership of `task` to `store`.
            store
                .spawn(async move |_| {
                    match task.await {
                        Ok(()) => {}
                        Err(e) if e.is_cancelled() => {}
                        Err(e) => std::panic::resume_unwind(e.into_panic()),
                    }
                    Ok(())
                })
                .map_err(HttpError::trap)?;
            // Send one copy of `io` to the transmission future.
            _ = transmission_fut_io_tx.send((Some(Arc::clone(&io)), rx));
            // Send one copy of `io` to the request body.
            _ = request_body_io_tx.send(Arc::clone(&io));
            // Attach a reference to the io task to the response body so that
            // the `task` can be cancelled if the body is dropped and no other
            // references remain.
            body.with_state(io).boxed_unsync()
        }
    };
    store.with(|mut store| {
        let view = getter(store.data_mut());
        let res = Response {
            status,
            headers: FieldMap::new_immutable(view.hooks, headers),
            body: Body::Host {
                body,
                result_tx: res_result_tx,
            },
        };
        view.table
            .push(res)
            .context("failed to push response to table")
            .map_err(HttpError::trap)
    })
}

impl Host for WasiHttpCtxView<'_> {}

mod named {
    use crate::p3::bindings::http::types::{ErrorCode, Request, Response};
    use crate::p3::bindings::named_imports::wasi::http::client::{Host, HostWithStore};
    use crate::p3::{HttpError, HttpResult};
    use crate::{WasiHttpNamed, WasiHttpNamedView};
    use wasmtime::component::{Accessor, Resource};
    use wasmtime_wasi::{NamedId, WasiCtxNamedView};

    impl<T, U> HostWithStore<U> for WasiHttpNamed<T>
    where
        T: WasiHttpNamedView,
        U: 'static,
    {
        async fn send(
            store: &Accessor<U, Self>,
            id: NamedId,
            req: Resource<Request>,
        ) -> HttpResult<Resource<Response>> {
            let getter = store.getter();
            super::send(store, move |data| getter(data).0.http(id), req).await
        }
    }

    impl<T> Host for WasiCtxNamedView<'_, T>
    where
        T: WasiHttpNamedView,
    {
        fn convert_error_code(&mut self, error: HttpError) -> wasmtime::Result<ErrorCode> {
            error.downcast()
        }
    }
}

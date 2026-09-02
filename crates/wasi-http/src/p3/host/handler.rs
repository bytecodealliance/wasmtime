use crate::FieldMap;
use crate::p3::bindings::http::client::{Host, HostWithStore};
use crate::p3::bindings::http::types::{Request, Response};
use crate::p3::body::Body;
use crate::p3::{HttpError, HttpResult};
use crate::{Error, WasiHttp, WasiHttpCtxView};
use core::task::{Context, Poll, Waker};
use tokio::sync::oneshot;
use tokio::task::{self, JoinHandle};
use tracing::debug;
use wasmtime::AsContextMut as _;
use wasmtime::component::{Accessor, HasData, Resource};
use wasmtime::error::Context as _;

/// A wrapper around [`JoinHandle`], which will [`JoinHandle::abort`] the task
/// when dropped
struct AbortOnDropJoinHandle(JoinHandle<()>);

impl Drop for AbortOnDropJoinHandle {
    fn drop(&mut self) {
        self.0.abort();
    }
}

const DROPPED_FUTURE_ERROR: &str =
    "Future indicating transmission result dropped without being resolved.";

async fn io_task_result(
    rx: oneshot::Receiver<(
        Option<AbortOnDropJoinHandle>,
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
        Option<AbortOnDropJoinHandle>,
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
        Option<AbortOnDropJoinHandle>,
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
        send(store, store.getter(), req).await
    }
}

async fn send<T, D>(
    store: &Accessor<T, D>,
    mut getter: impl FnMut(&mut T) -> WasiHttpCtxView<'_> + Clone + Unpin + Send + 'static,
    req: Resource<Request>,
) -> HttpResult<Resource<Response>>
where
    D: HasData,
    T: 'static,
{
    // A handle to the I/O task, if spawned, will be sent on this channel
    // along with the result receiver
    let (io_result_tx, io_result_rx) = oneshot::channel();

    // Response processing result will be sent on this channel
    let (res_result_tx, res_result_rx) = oneshot::channel();

    let fut = store.with(|mut store| {
        let req = {
            let mut ctx = store.as_context_mut();
            let WasiHttpCtxView { table, .. } = getter(ctx.data_mut());
            table
                .delete(req)
                .context("failed to delete request from table")
                .map_err(HttpError::trap)?
        };
        let (req, options) =
            req.into_http_with_getter(&mut store, io_task_result(io_result_rx), getter.clone())?;
        let mut ctx = store.as_context_mut();
        HttpResult::Ok(getter(ctx.data_mut()).hooks.send_request(
            req,
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
                send_dummy_io(Err(err_code.clone().into()), io_result_tx);
                return Err(err_code.into());
            }
            Err(e) => {
                let e = Error::InternalError(Some(format!("{e}")));
                return Err(send_dummy_io_err(store, &mut getter, e, io_result_tx));
            }
        },
    };
    let (res, io) = match Box::into_pin(fut).await {
        Ok(r) => r,
        Err(e) => {
            return Err(send_dummy_io_err(store, &mut getter, e, io_result_tx));
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
            send_dummy_io(Ok(()), io_result_tx);
            body
        }
        Poll::Ready(Err(e)) => {
            return Err(send_dummy_io_err(store, &mut getter, e, io_result_tx));
        }
        Poll::Pending => {
            // I/O driver still needs to be polled, spawn a task and send handles to it
            let (tx, rx) = oneshot::channel();
            let io = AbortOnDropJoinHandle(task::spawn(async move {
                let res = io.await;
                debug!(?res, "`send_request` I/O future finished");
                _ = tx.send(res);
            }));
            _ = io_result_tx.send((Some(io), rx));
            body
        }
    };
    store.with(|mut store| {
        let mut ctx = store.as_context_mut();
        let view = getter(ctx.data_mut());
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

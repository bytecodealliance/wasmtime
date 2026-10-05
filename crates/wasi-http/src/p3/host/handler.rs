use crate::FieldMap;
use crate::p3::bindings::http::client::{Host, HostWithStore};
use crate::p3::bindings::http::types::{Request, Response};
use crate::p3::body::{Body, BodyExt as _};
use crate::p3::{HttpError, HttpResult};
use crate::{Error, WasiHttp, WasiHttpCtxView};
use core::task::{Context, Poll, Waker};
use http_body_util::BodyExt as _;
use std::sync::Arc;
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

async fn transmission_result(rx: oneshot::Receiver<Result<(), Error>>) -> Result<(), Error> {
    let Ok(transmission_result) = rx.await else {
        return Err(Error::InternalError(Some(DROPPED_FUTURE_ERROR.to_string())));
    };
    transmission_result
}

fn send_transmission_err<T, D>(
    store: &Accessor<T, D>,
    mut getter: impl FnMut(&mut T) -> WasiHttpCtxView<'_>,
    e: Error,
    transmission_result_tx: oneshot::Sender<Result<(), Error>>,
) -> HttpError
where
    D: HasData,
{
    let err_code =
        store.with(|mut store| getter(store.as_context_mut().data_mut()).error_to_p3(&e));
    let _ = transmission_result_tx.send(Err(e));
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
    // A handle to the I/O task, if spawned, will be sent on this channel
    // and kept as part of request body state
    let (io_task_tx, io_task_rx) = oneshot::channel();

    // The result of sending the request will be sent on this channel.
    let (transmission_result_tx, transmission_result_rx) = oneshot::channel();

    // Response processing result will be sent on this channel
    let (res_result_tx, res_result_rx) = oneshot::channel();

    let fut = store.with(|mut store| {
        let WasiHttpCtxView { table, .. } = getter(store.data_mut());
        let req = table
            .delete(req)
            .context("failed to delete request from table")
            .map_err(HttpError::trap)?;
        let (req, options) = req.into_http_with_getter(
            &mut store,
            transmission_result(transmission_result_rx),
            getter,
        )?;
        HttpResult::Ok(getter(store.data_mut()).hooks.send_request(
            // Attach a reference to the io task to the body so that it
            // isn't cancelled if the body is dropped.
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
    });
    let fut = match fut {
        Ok(fut) => fut,
        Err(e) => match e.downcast() {
            Ok(err_code) => {
                let _ = transmission_result_tx.send(Err(err_code.clone().into()));
                return Err(err_code.into());
            }
            Err(e) => {
                let e = Error::InternalError(Some(format!("{e}")));
                return Err(send_transmission_err(
                    store,
                    getter,
                    e,
                    transmission_result_tx,
                ));
            }
        },
    };
    let (res, io) = match Box::into_pin(fut).await {
        Ok(r) => {
            // Receiving response headers means the request was successfully
            // transmitted.
            let _ = transmission_result_tx.send(Ok(()));
            r
        }
        Err(e) => {
            return Err(send_transmission_err(
                store,
                getter,
                e,
                transmission_result_tx,
            ));
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
        Poll::Ready(Ok(())) => body,
        Poll::Ready(Err(e)) => {
            return Err(store
                .with(|mut store| getter(store.as_context_mut().data_mut()).error_to_p3(&e))
                .into());
        }
        Poll::Pending => {
            // I/O driver still needs to be polled, spawn a task and send handles to it
            let io = Arc::new(AbortOnDropJoinHandle(task::spawn(async move {
                let res = io.await;
                debug!(?res, "`send_request` I/O future finished");
            })));
            _ = io_task_tx.send(Arc::clone(&io));
            // Attach a reference to the io task to the body so that it
            // isn't cancelled if the body is dropped.
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

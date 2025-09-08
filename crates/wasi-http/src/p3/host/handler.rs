use crate::p3::bindings::http::handler::{Host, HostWithStore};
use crate::p3::bindings::http::types::{ErrorCode, Request, Response};
use crate::p3::body::{Body, ConsumedBody, GuestBody, GuestBodyKind};
use crate::p3::{HttpError, HttpResult, WasiHttp, WasiHttpCtxView, get_content_length};
use anyhow::Context as _;
use core::pin::Pin;
use http::header::HOST;
use http::{HeaderValue, Uri};
use http_body_util::BodyExt as _;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::debug;
use wasmtime::component::{Accessor, AccessorTask, Resource};

struct SendRequestTask {
    io: Pin<Box<dyn Future<Output = Result<(), ErrorCode>> + Send>>,
    result_tx: oneshot::Sender<Result<(), ErrorCode>>,
}

impl<T> AccessorTask<T, WasiHttp, wasmtime::Result<()>> for SendRequestTask {
    async fn run(self, _: &Accessor<T, WasiHttp>) -> wasmtime::Result<()> {
        let res = self.io.await;
        debug!(?res, "`send_request` I/O future finished");
        _ = self.result_tx.send(res);
        Ok(())
    }
}

impl HostWithStore for WasiHttp {
    async fn handle<T>(
        store: &Accessor<T, Self>,
        req: Resource<Request>,
    ) -> HttpResult<Resource<Response>> {
        let getter = store.getter();
        let (io_result_tx, io_result_rx) = oneshot::channel();
        let (res_result_tx, res_result_rx) = oneshot::channel();
        let fut = store.with(|mut store| {
            let WasiHttpCtxView { table, .. } = store.get();
            let Request {
                method,
                scheme,
                authority,
                path_with_query,
                headers,
                options,
                body,
            } = table
                .delete(req)
                .context("failed to delete request from table")
                .map_err(HttpError::trap)?;
            let mut headers = Arc::unwrap_or_clone(headers);
            let body = match body {
                Body::Guest {
                    contents_rx,
                    trailers_rx,
                    result_tx,
                } => {
                    let (http_result_tx, http_result_rx) = oneshot::channel();
                    let content_length = get_content_length(&headers)
                        .map_err(|err| ErrorCode::InternalError(Some(format!("{err:#}"))))?;
                    _ = result_tx.send(Box::new(async move {
                        if let Ok(Err(err)) = http_result_rx.await {
                            return Err(err);
                        };
                        io_result_rx.await.unwrap_or(Ok(()))
                    }));
                    GuestBody::new(
                        &mut store,
                        contents_rx,
                        trailers_rx,
                        http_result_tx,
                        content_length,
                        GuestBodyKind::Request,
                        getter,
                    )
                    .boxed()
                }
                Body::Host { body, result_tx } => {
                    _ = result_tx.send(Box::new(
                        async move { io_result_rx.await.unwrap_or(Ok(())) },
                    ));
                    body
                }
                Body::Consumed => ConsumedBody.boxed(),
            };

            let WasiHttpCtxView { ctx, .. } = store.get();
            if ctx.set_host_header() {
                let host = if let Some(authority) = authority.as_ref() {
                    HeaderValue::try_from(authority.as_str())
                        .map_err(|err| ErrorCode::InternalError(Some(err.to_string())))?
                } else {
                    HeaderValue::from_static("")
                };
                headers.insert(HOST, host);
            }
            let scheme = match scheme {
                None => ctx.default_scheme().ok_or(ErrorCode::HttpProtocolError)?,
                Some(scheme) if ctx.is_supported_scheme(&scheme) => scheme,
                Some(..) => return Err(ErrorCode::HttpProtocolError.into()),
            };
            let mut uri = Uri::builder().scheme(scheme);
            if let Some(authority) = authority {
                uri = uri.authority(authority)
            };
            if let Some(path_with_query) = path_with_query {
                uri = uri.path_and_query(path_with_query)
            };
            let uri = uri.build().map_err(|err| {
                debug!(?err, "failed to build request URI");
                ErrorCode::HttpRequestUriInvalid
            })?;
            let mut req = http::Request::builder();
            *req.headers_mut().unwrap() = headers;
            let req = req
                .method(method)
                .uri(uri)
                .body(body)
                .map_err(|err| ErrorCode::InternalError(Some(err.to_string())))?;
            HttpResult::Ok(store.get().ctx.send_request(
                req,
                options.as_deref().copied(),
                Box::new(async {
                    let Ok(fut) = res_result_rx.await else {
                        return Ok(());
                    };
                    Box::into_pin(fut).await
                }),
            ))
        })?;
        let (res, io) = Box::into_pin(fut).await?;
        store.spawn(SendRequestTask {
            io: Box::into_pin(io),
            result_tx: io_result_tx,
        });
        let (
            http::response::Parts {
                status, headers, ..
            },
            body,
        ) = res.into_parts();
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

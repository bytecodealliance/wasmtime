use crate::p3::bindings::http::handler::{Host, HostWithStore};
use crate::p3::bindings::http::types::{ErrorCode, Request, Response};
use crate::p3::body::{Body, ConsumedBody, GuestBody};
use crate::p3::host::{delete_request, push_response};
use crate::p3::{HttpError, HttpResult, WasiHttp, WasiHttpCtxView};
use http::header::HOST;
use http::{HeaderValue, Uri};
use http_body_util::BodyExt as _;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::debug;
use wasmtime::component::{Accessor, Resource};

impl HostWithStore for WasiHttp {
    async fn handle<T>(
        store: &Accessor<T, Self>,
        req: Resource<Request>,
    ) -> HttpResult<Resource<Response>> {
        let getter = store.getter();
        let (res_result_tx, res_result_rx) = oneshot::channel();
        let (fut, req_result_tx) = store.with(|mut store| {
            let WasiHttpCtxView { ctx, table } = store.get();
            let Request {
                method,
                scheme,
                authority,
                path_with_query,
                headers,
                options,
                body,
            } = delete_request(table, req).map_err(HttpError::trap)?;
            let mut headers = Arc::unwrap_or_clone(headers);
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
            let (body, result_tx) = match body {
                Body::Guest {
                    contents_rx,
                    trailers_rx,
                    result_tx,
                } => (
                    GuestBody::new(&mut store, contents_rx, trailers_rx, getter).boxed(),
                    Some(result_tx),
                ),
                Body::Host { body, result_tx } => (body, Some(result_tx)),
                Body::Consumed => (ConsumedBody.boxed(), None),
            };
            let req = req
                .method(method)
                .uri(uri)
                .body(body)
                .map_err(|err| ErrorCode::InternalError(Some(err.to_string())))?;
            HttpResult::Ok((
                store.get().ctx.send_request(
                    req,
                    options.as_deref().copied(),
                    Box::new(async {
                        let Ok(fut) = res_result_rx.await else {
                            return Ok(());
                        };
                        Box::into_pin(fut).await
                    }),
                ),
                result_tx,
            ))
        })?;
        let (res, io) = Box::into_pin(fut).await?;
        if let Some(req_result_tx) = req_result_tx {
            if let Err(io) = req_result_tx.send(io) {
                Box::into_pin(io).await?;
            }
        } else {
            Box::into_pin(io).await?;
        }
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
        store.with(|mut store| push_response(store.get().table, res).map_err(HttpError::trap))
    }
}

impl Host for WasiHttpCtxView<'_> {}

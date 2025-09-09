use crate::p3::bindings::http::types::ErrorCode;
use crate::p3::body::{Body, BodyKind, ConsumedBody, GuestBody};
use crate::p3::{WasiHttpView, get_content_length};
use anyhow::Context as _;
use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use http_body_util::BodyExt as _;
use http_body_util::combinators::BoxBody;
use std::sync::Arc;
use tokio::sync::oneshot;
use wasmtime::AsContextMut;

/// The concrete type behind a `wasi:http/types/response` resource.
pub struct Response {
    /// The status of the response.
    pub status: StatusCode,
    /// The headers of the response.
    pub headers: Arc<HeaderMap>,
    /// Response body.
    pub(crate) body: Body,
}

impl TryFrom<Response> for http::Response<Body> {
    type Error = http::Error;

    fn try_from(
        Response {
            status,
            headers,
            body,
        }: Response,
    ) -> Result<Self, Self::Error> {
        let mut res = http::Response::builder().status(status);
        *res.headers_mut().unwrap() = Arc::unwrap_or_clone(headers);
        res.body(body)
    }
}

impl Response {
    /// Convert [Response] into [http::Response].
    ///
    /// The specified [Future] `fut` can be used to communicate
    /// a response processing error, if any, to the constructor of the response.
    /// For example, if the response was constructed via `wasi:http/types.response#new`,
    /// a result sent on `fut` will be forwarded to the guest on the future handle returned.
    pub fn into_http<T: WasiHttpView + 'static>(
        self,
        store: impl AsContextMut<Data = T>,
        fut: impl Future<Output = Result<(), ErrorCode>> + Send + 'static,
    ) -> wasmtime::Result<http::Response<BoxBody<Bytes, ErrorCode>>> {
        let res = http::Response::try_from(self)?;
        let (res, body) = res.into_parts();
        let body = match body {
            Body::Guest {
                contents_rx,
                trailers_rx,
                result_tx,
            } => {
                let (http_result_tx, http_result_rx) = oneshot::channel();
                // `Content-Length` header value is validated in `fields` implementation
                let content_length =
                    get_content_length(&res.headers).context("failed to parse `content-length`")?;
                _ = result_tx.send(Box::new(async move {
                    if let Ok(Err(err)) = http_result_rx.await {
                        return Err(err);
                    };
                    fut.await
                }));
                GuestBody::new(
                    store,
                    contents_rx,
                    trailers_rx,
                    http_result_tx,
                    content_length,
                    BodyKind::Response,
                    T::http,
                )
                .boxed()
            }
            Body::Host { body, result_tx } => {
                _ = result_tx.send(Box::new(fut));
                body
            }
            Body::Consumed => ConsumedBody.boxed(),
        };
        Ok(http::Response::from_parts(res, body))
    }
}

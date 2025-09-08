use crate::p3::bindings::http::types::ErrorCode;
use crate::p3::body::{Body, ConsumedBody, GuestBody, GuestBodyKind};
use crate::p3::{WasiHttpView, get_content_length};
use anyhow::Context as _;
use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use http_body_util::BodyExt as _;
use http_body_util::combinators::BoxBody;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
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
    /// Construct a new [Response]
    pub fn new(
        status: StatusCode,
        headers: impl Into<Arc<HeaderMap>>,
        body: impl Into<BoxBody<Bytes, ErrorCode>>,
    ) -> (
        Self,
        impl Future<Output = Result<(), ErrorCode>> + Send + 'static,
    ) {
        let (tx, rx) = oneshot::channel();
        (
            Self {
                status,
                headers: headers.into(),
                body: Body::Host {
                    body: body.into(),
                    result_tx: tx,
                },
            },
            async {
                let Ok(fut) = rx.await else { return Ok(()) };
                Box::into_pin(fut).await
            },
        )
    }

    /// Convert [Response] into [http::Response].
    ///
    /// This returns an [mpsc::Sender] that can be used to communicate
    /// a response processing error, if any.
    pub fn into_http<T: WasiHttpView + 'static>(
        self,
        store: impl AsContextMut<Data = T>,
    ) -> wasmtime::Result<(
        http::Response<BoxBody<Bytes, ErrorCode>>,
        mpsc::Sender<Result<(), ErrorCode>>,
    )> {
        let res = http::Response::try_from(self)?;
        let (res, body) = res.into_parts();
        let (tx, mut rx) = mpsc::channel(1);
        let body = match body {
            Body::Guest {
                contents_rx,
                trailers_rx,
                result_tx,
            } => {
                let content_length =
                    get_content_length(&res.headers).context("failed to parse `content-length`")?;
                _ = result_tx.send(Box::new(async move { rx.recv().await.unwrap_or(Ok(())) }));
                GuestBody::new(
                    store,
                    contents_rx,
                    trailers_rx,
                    tx.clone(),
                    content_length,
                    GuestBodyKind::Response,
                    T::http,
                )
                .boxed()
            }
            Body::Host { body, result_tx } => {
                _ = result_tx.send(Box::new(async move { rx.recv().await.unwrap_or(Ok(())) }));
                body
            }
            Body::Consumed => ConsumedBody.boxed(),
        };
        Ok((http::Response::from_parts(res, body), tx))
    }
}

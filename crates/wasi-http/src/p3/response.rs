use crate::p3::bindings::http::types::ErrorCode;
use crate::p3::body::{Body, ConsumedBody, GuestBodyConsumer, GuestTrailerConsumer};
use crate::p3::{WasiHttpView, body::GuestBody};
use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use http_body_util::BodyExt as _;
use http_body_util::combinators::BoxBody;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::PollSender;
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
    ) -> Self {
        Self {
            status,
            headers: headers.into(),
            body: Body::Host(body.into()),
        }
    }

    /// Convert [Response] into [http::Response].
    pub fn into_http<T: WasiHttpView + 'static>(
        self,
        mut store: impl AsContextMut<Data = T>,
    ) -> http::Result<(
        http::Response<BoxBody<Bytes, ErrorCode>>,
        Option<oneshot::Sender<Result<(), ErrorCode>>>,
    )> {
        let response = http::Response::try_from(self)?;
        let (response, body) = response.into_parts();
        let (body, tx) = match body {
            Body::Guest {
                contents_rx,
                trailers_rx,
                result_tx,
            } => {
                let (trailers_http_tx, trailers_http_rx) = oneshot::channel();
                trailers_rx.pipe(
                    &mut store,
                    GuestTrailerConsumer {
                        tx: trailers_http_tx,
                        getter: T::http,
                    },
                );
                let contents_rx = contents_rx.map(|rx| {
                    let (http_tx, http_rx) = mpsc::channel(1);
                    rx.pipe(
                        store,
                        GuestBodyConsumer {
                            tx: PollSender::new(http_tx),
                        },
                    );
                    http_rx
                });
                (
                    GuestBody {
                        trailers_rx: Some(trailers_http_rx),
                        contents_rx,
                    }
                    .boxed(),
                    Some(result_tx),
                )
            }
            Body::Host(body) => (body, None),
            Body::Consumed => (ConsumedBody.boxed(), None),
        };
        Ok((http::Response::from_parts(response, body), tx))
    }
}

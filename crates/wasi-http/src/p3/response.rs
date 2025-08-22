use crate::p3::bindings::http::types::ErrorCode;
use crate::p3::body::{Body, ConsumedBody, GuestBody, GuestBodyTaskContext};
use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use http_body_util::BodyExt as _;
use http_body_util::combinators::BoxBody;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

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
    pub fn into_http(
        self,
    ) -> http::Result<(
        http::Response<BoxBody<Bytes, ErrorCode>>,
        Option<GuestBodyTaskContext>,
    )> {
        let response = http::Response::try_from(self)?;
        let (response, body) = response.into_parts();
        let (body, cx) = match body {
            Body::Guest(cx) => {
                let (contents_tx, contents_rx) = mpsc::channel(1);
                let (trailers_tx, trailers_rx) = oneshot::channel();
                let body = GuestBody {
                    contents_rx: Some(contents_rx),
                    trailers_rx: Some(trailers_rx),
                };
                let cx = GuestBodyTaskContext {
                    cx,
                    contents_tx,
                    trailers_tx,
                };
                (body.boxed(), Some(cx))
            }
            Body::Host(body) => (body, None),
            Body::Consumed => (ConsumedBody.boxed(), None),
        };
        Ok((http::Response::from_parts(response, body), cx))
    }
}

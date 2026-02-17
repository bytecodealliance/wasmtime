use crate::get_content_length;
use crate::p3::bindings::http::types::ErrorCode;
use crate::p3::body::{Body, GuestBody};
use crate::p3::{WasiHttpCtxView, WasiHttpView};
use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use http_body_util::BodyExt as _;
use http_body_util::combinators::UnsyncBoxBody;
use std::sync::Arc;
use wasmtime::AsContextMut;
use wasmtime::error::Context as _;

/// The concrete type behind a `wasi:http/types.response` resource.
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
    ) -> wasmtime::Result<http::Response<UnsyncBoxBody<Bytes, ErrorCode>>> {
        self.into_http_with_getter(store, fut, T::http)
    }

    /// Like [`Self::into_http`], but with a custom function for converting `T`
    /// to a [`WasiHttpCtxView`].
    pub fn into_http_with_getter<T: 'static>(
        self,
        store: impl AsContextMut<Data = T>,
        fut: impl Future<Output = Result<(), ErrorCode>> + Send + 'static,
        getter: fn(&mut T) -> WasiHttpCtxView<'_>,
    ) -> wasmtime::Result<http::Response<UnsyncBoxBody<Bytes, ErrorCode>>> {
        let res = http::Response::try_from(self)?;
        let (res, body) = res.into_parts();
        let body = match body {
            Body::Guest {
                contents_rx,
                trailers_rx,
                result_tx,
            } => {
                // `Content-Length` header value is validated in `fields` implementation
                let content_length =
                    get_content_length(&res.headers).context("failed to parse `content-length`")?;
                GuestBody::new(
                    store,
                    contents_rx,
                    trailers_rx,
                    result_tx,
                    fut,
                    content_length,
                    ErrorCode::HttpResponseBodySize,
                    getter,
                )
                .boxed_unsync()
            }
            Body::Host { body, result_tx } => {
                _ = result_tx.send(Box::new(fut));
                body
            }
        };
        Ok(http::Response::from_parts(res, body))
    }

    /// Convert [http::Response] into [Response].
    pub fn from_http<T>(
        res: http::Response<T>,
    ) -> (
        Self,
        impl Future<Output = Result<(), ErrorCode>> + Send + 'static,
    )
    where
        T: http_body::Body<Data = Bytes> + Send + 'static,
        T::Error: Into<ErrorCode>,
    {
        let (parts, body) = res.into_parts();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();

        let wasi_response = Response {
            status: parts.status,
            headers: Arc::new(parts.headers),
            body: Body::Host {
                body: body.map_err(Into::into).boxed_unsync(),
                result_tx,
            },
        };

        let io_future = async {
            let Ok(fut) = result_rx.await else {
                return Ok(());
            };
            Box::into_pin(fut).await
        };

        (wasi_response, io_future)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::future::Future;
    use core::pin::pin;
    use core::task::{Context, Poll, Waker};
    use http_body_util::Full;

    #[tokio::test]
    async fn test_response_from_http() {
        let http_response = http::Response::builder()
            .status(StatusCode::OK)
            .header("x-custom-header", "value123")
            .body(Full::new(Bytes::from_static(b"hello wasm")))
            .unwrap();

        let (wasi_resp, io_future) = Response::from_http(http_response);
        assert_eq!(wasi_resp.status, StatusCode::OK);
        assert_eq!(
            wasi_resp.headers.get("x-custom-header").unwrap(),
            "value123"
        );
        match wasi_resp.body {
            Body::Host { body, result_tx } => {
                let collected = body.collect().await;
                assert!(collected.is_ok(), "Body stream failed unexpectedly");
                let chunks = collected.unwrap().to_bytes();
                assert_eq!(chunks, &b"hello wasm"[..]);
                _ = result_tx.send(Box::new(async { Ok(()) }));
            }
            _ => panic!("Response body should be of type Host"),
        }

        let mut cx = Context::from_waker(Waker::noop());
        let result = pin!(io_future).poll(&mut cx);
        assert!(matches!(result, Poll::Ready(Ok(_))));
    }
}

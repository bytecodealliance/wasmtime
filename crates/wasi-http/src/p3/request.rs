use crate::p3::bindings::http::types::ErrorCode;
use crate::p3::body::{Body, BodyExt as _, GuestBody};
use crate::p3::{HttpError, HttpResult};
use crate::{
    Error, FieldMap, RequestOptions, WasiHttpCtxView, WasiHttpHooks, WasiHttpView,
    get_content_length,
};
use bytes::Bytes;
use http::header::HOST;
use http::uri::{Authority, PathAndQuery, Scheme};
use http::{HeaderValue, Method, Uri};
use http_body_util::BodyExt as _;
use http_body_util::combinators::UnsyncBoxBody;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::debug;
use wasmtime::AsContextMut;

/// The concrete type behind a `wasi:http/types.request` resource.
pub struct Request {
    /// The method of the request.
    pub method: Method,
    /// The scheme of the request.
    pub scheme: Option<Scheme>,
    /// The authority of the request.
    pub authority: Option<Authority>,
    /// The path and query of the request.
    pub path_with_query: Option<PathAndQuery>,
    /// The request headers.
    pub headers: FieldMap,
    /// Request options.
    pub options: Option<Arc<RequestOptions>>,
    /// Request body.
    pub(crate) body: Body,
}

impl Request {
    /// Construct a new [Request]
    ///
    /// This returns a [Future] that the will be used to communicate
    /// a request processing error, if any.
    ///
    /// Requests constructed this way will not perform any `Content-Length` validation.
    pub fn new<B>(
        method: Method,
        scheme: Option<Scheme>,
        authority: Option<Authority>,
        path_with_query: Option<PathAndQuery>,
        headers: impl Into<FieldMap>,
        options: Option<Arc<RequestOptions>>,
        body: B,
    ) -> (
        Self,
        impl Future<Output = Result<(), Error>> + Send + 'static,
    )
    where
        B: http_body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<Error>,
    {
        let (tx, rx) = oneshot::channel();
        (
            Self {
                method,
                scheme,
                authority,
                path_with_query,
                headers: headers.into(),
                options,
                body: Body::Host {
                    body: body.map_err(Into::into).boxed_unsync(),
                    result_tx: tx,
                },
            },
            async {
                let Ok(fut) = rx.await else { return Ok(()) };
                Box::into_pin(fut).await
            },
        )
    }

    /// Construct a new [Request] from [http::Request].
    ///
    /// This returns a [Future] that will be used to communicate
    /// a request processing error, if any.
    ///
    /// Requests constructed this way will not perform any `Content-Length` validation.
    pub fn from_http<T>(
        hooks: &mut dyn WasiHttpHooks,
        req: http::Request<T>,
    ) -> (
        Self,
        impl Future<Output = Result<(), Error>> + Send + 'static,
    )
    where
        T: http_body::Body<Data = Bytes> + Send + 'static,
        T::Error: Into<Error>,
    {
        let (
            http::request::Parts {
                method,
                uri,
                headers,
                ..
            },
            body,
        ) = req.into_parts();
        let http::uri::Parts {
            scheme,
            authority,
            path_and_query,
            ..
        } = uri.into_parts();
        Self::new(
            method,
            scheme,
            authority,
            path_and_query,
            FieldMap::new_immutable(hooks, headers),
            None,
            body,
        )
    }

    /// Convert this [`Request`] into an [`http::Request<UnsyncBoxBody<Bytes, ErrorCode>>`].
    ///
    /// The specified future `fut` can be used to communicate a request processing
    /// error, if any, back to the caller (e.g., if this request was constructed
    /// through `wasi:http/types.request#new`).
    pub fn into_http<T: WasiHttpView + 'static>(
        self,
        store: impl AsContextMut<Data = T>,
        fut: impl Future<Output = Result<(), Error>> + Send + 'static,
    ) -> HttpResult<(
        http::Request<UnsyncBoxBody<Bytes, Error>>,
        Option<Arc<RequestOptions>>,
    )> {
        self.into_http_with_getter(store, fut, T::http)
    }

    /// Like [`Self::into_http`], but uses a custom getter for obtaining the [`WasiHttpCtxView`].
    pub fn into_http_with_getter<T: 'static>(
        self,
        mut store: impl AsContextMut<Data = T>,
        fut: impl Future<Output = Result<(), Error>> + Send + 'static,
        getter: fn(&mut T) -> WasiHttpCtxView<'_>,
    ) -> HttpResult<(
        http::Request<UnsyncBoxBody<Bytes, Error>>,
        Option<Arc<RequestOptions>>,
    )> {
        let Request {
            method,
            scheme,
            authority,
            path_with_query,
            mut headers,
            options,
            body,
        } = self;
        // `Content-Length` header value is validated in `fields` implementation
        let content_length = match get_content_length(&headers) {
            Ok(content_length) => content_length,
            Err(err) => {
                body.drop(&mut store).map_err(HttpError::trap)?;
                return Err(ErrorCode::InternalError(Some(format!("{err:#}"))).into());
            }
        };
        // This match must appear before any potential errors handled with '?'
        // (or errors have to explicitly be addressed and drop the body, as above),
        // as otherwise the Body::Guest resources will not be cleaned up when dropped.
        // see: https://github.com/bytecodealliance/wasmtime/pull/11440#discussion_r2326139381
        // for additional context.
        let body = match body {
            Body::Guest {
                contents_rx,
                trailers_rx,
                result_tx,
            } => GuestBody::new(
                &mut store,
                contents_rx,
                trailers_rx,
                result_tx,
                fut,
                content_length,
                ErrorCode::HttpRequestBodySize,
                getter,
            )
            .map_err(HttpError::trap)?
            .boxed_unsync(),
            Body::Host { body, result_tx } => {
                if let Some(limit) = content_length {
                    let (http_result_tx, http_result_rx) = oneshot::channel();
                    _ = result_tx.send(Box::new(async move {
                        if let Ok(err) = http_result_rx.await {
                            return Err(err);
                        };
                        fut.await
                    }));
                    body.with_content_length(limit, http_result_tx, Error::HttpRequestBodySize)
                        .boxed_unsync()
                } else {
                    _ = result_tx.send(Box::new(fut));
                    body
                }
            }
        };
        let mut store = store.as_context_mut();
        let WasiHttpCtxView { hooks, ctx, .. } = getter(store.data_mut());
        headers.set_mutable(ctx.field_size_limit);
        if hooks.set_host_header() {
            let host = if let Some(authority) = authority.as_ref() {
                HeaderValue::try_from(authority.as_str())
                    .map_err(|err| ErrorCode::InternalError(Some(err.to_string())))?
            } else {
                HeaderValue::from_static("")
            };
            headers.append_raw(HOST, host).map_err(HttpError::trap)?;
        }
        let scheme = match scheme {
            None => hooks.default_scheme().ok_or(ErrorCode::HttpProtocolError)?,
            Some(scheme) if hooks.is_supported_scheme(&scheme) => scheme,
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
        *req.headers_mut().unwrap() = headers.into();
        let req = req
            .method(method)
            .uri(uri)
            .body(body)
            .map_err(|err| ErrorCode::InternalError(Some(err.to_string())))?;
        Ok((req, options))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WasiHttpCtx;
    use core::future::Future;
    use core::pin::pin;
    use core::str::FromStr;
    use core::task::{Context, Poll, Waker};
    use http_body_util::{BodyExt, Empty, Full};
    use wasmtime::Result;
    use wasmtime::{Engine, Store};
    use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

    struct TestCtx {
        table: ResourceTable,
        wasi: WasiCtx,
        http: WasiHttpCtx,
    }

    impl TestCtx {
        fn new() -> Self {
            Self {
                table: ResourceTable::default(),
                wasi: WasiCtxBuilder::new().build(),
                http: Default::default(),
            }
        }
    }

    impl WasiView for TestCtx {
        fn ctx(&mut self) -> WasiCtxView<'_> {
            WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.table,
            }
        }
    }

    impl WasiHttpView for TestCtx {
        fn http(&mut self) -> WasiHttpCtxView<'_> {
            WasiHttpCtxView {
                ctx: &mut self.http,
                table: &mut self.table,
                hooks: crate::default_hooks(),
            }
        }
    }

    #[tokio::test]
    async fn test_request_into_http_schemes() -> Result<()> {
        let schemes = vec![Some(Scheme::HTTP), Some(Scheme::HTTPS), None];
        let engine = Engine::default();

        for scheme in schemes {
            let (req, fut) = Request::new(
                Method::POST,
                scheme.clone(),
                Some(Authority::from_static("example.com")),
                Some(PathAndQuery::from_static("/path?query=1")),
                FieldMap::default(),
                None,
                Full::new(Bytes::from_static(b"body")).boxed_unsync(),
            );
            let mut store = Store::new(&engine, TestCtx::new());
            let (http_req, options) = req.into_http(&mut store, async { Ok(()) }).unwrap();
            assert_eq!(options, None);
            assert_eq!(http_req.method(), Method::POST);
            let expected_scheme = scheme.unwrap_or(Scheme::HTTPS); // default scheme
            assert_eq!(
                http_req.uri(),
                &http::Uri::from_str(&format!(
                    "{}://example.com/path?query=1",
                    expected_scheme.as_str()
                ))
                .unwrap()
            );
            let body_bytes = http_req.into_body().collect().await?;
            assert_eq!(body_bytes.to_bytes(), b"body".as_slice());
            let mut cx = Context::from_waker(Waker::noop());
            let result = pin!(fut).poll(&mut cx);
            assert!(matches!(result, Poll::Ready(Ok(()))));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_request_into_http_uri_error() -> Result<()> {
        let (req, fut) = Request::new(
            Method::GET,
            Some(Scheme::HTTP),
            Some(Authority::from_static("example.com")),
            None, // <-- should fail, must be Some(_) when authority is set
            FieldMap::default(),
            None,
            Empty::new().boxed_unsync(),
        );
        let mut store = Store::new(&Engine::default(), TestCtx::new());
        let result = req
            .into_http(&mut store, async {
                Err(Error::InternalError(Some("uh oh".to_string())))
            })
            .unwrap_err();
        assert!(matches!(
            result.downcast()?,
            ErrorCode::HttpRequestUriInvalid,
        ));
        let mut cx = Context::from_waker(Waker::noop());
        let result = pin!(fut).poll(&mut cx);
        assert!(matches!(
            result,
            Poll::Ready(Err(Error::InternalError(Some(_))))
        ));

        Ok(())
    }
}

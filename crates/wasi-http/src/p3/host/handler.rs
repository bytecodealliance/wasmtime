use crate::get_content_length;
use crate::p3::bindings::http::handler::{Host, HostWithStore};
use crate::p3::bindings::http::types::{ErrorCode, Request, Response};
use crate::p3::body::{Body, ConsumedBody, GuestBody};
use crate::p3::{HttpError, HttpResult, WasiHttp, WasiHttpCtxView};
use anyhow::Context as _;
use bytes::Bytes;
use core::pin::Pin;
use core::task::{Context, Poll, Waker, ready};
use http::header::HOST;
use http::{HeaderValue, Uri};
use http_body_util::BodyExt as _;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::debug;
use wasmtime::component::{Accessor, AccessorTask, JoinHandle, Resource};

/// A wrapper around [`JoinHandle`], which will [`JoinHandle::abort`] the task
/// when dropped
struct AbortOnDropJoinHandle(JoinHandle);

impl Drop for AbortOnDropJoinHandle {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// A wrapper around [http_body::Body], which allows attaching arbitrary state to it
struct BodyWithState<T, U> {
    body: T,
    _state: U,
}

impl<T, U> http_body::Body for BodyWithState<T, U>
where
    T: http_body::Body + Unpin,
    U: Unpin,
{
    type Data = T::Data;
    type Error = T::Error;

    #[inline]
    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.get_mut().body).poll_frame(cx)
    }

    #[inline]
    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    #[inline]
    fn size_hint(&self) -> http_body::SizeHint {
        self.body.size_hint()
    }
}

/// A wrapper around [http_body::Body], which validates `Content-Length`
struct BodyWithContentLength<T, E> {
    body: T,
    error_tx: Option<oneshot::Sender<E>>,
    make_error: fn(Option<u64>) -> E,
    /// Limit of bytes to be sent
    limit: u64,
    /// Number of bytes sent
    sent: u64,
}

impl<T, E> BodyWithContentLength<T, E> {
    /// Sends the error constructed by [Self::make_error] on [Self::error_tx].
    /// Does nothing if an error has already been sent on [Self::error_tx].
    fn send_error<V>(&mut self, sent: Option<u64>) -> Poll<Option<Result<V, E>>> {
        if let Some(error_tx) = self.error_tx.take() {
            _ = error_tx.send((self.make_error)(sent));
        }
        Poll::Ready(Some(Err((self.make_error)(sent))))
    }
}

impl<T, E> http_body::Body for BodyWithContentLength<T, E>
where
    T: http_body::Body<Data = Bytes, Error = E> + Unpin,
{
    type Data = T::Data;
    type Error = T::Error;

    #[inline]
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        match ready!(Pin::new(&mut self.as_mut().body).poll_frame(cx)) {
            Some(Ok(frame)) => {
                let Some(data) = frame.data_ref() else {
                    return Poll::Ready(Some(Ok(frame)));
                };
                let Ok(sent) = data.len().try_into() else {
                    return self.send_error(None);
                };
                let Some(sent) = self.sent.checked_add(sent) else {
                    return self.send_error(None);
                };
                if sent > self.limit {
                    return self.send_error(Some(sent));
                }
                self.sent = sent;
                Poll::Ready(Some(Ok(frame)))
            }
            Some(Err(err)) => Poll::Ready(Some(Err(err))),
            None if self.limit != self.sent => {
                // short write
                let sent = self.sent;
                self.send_error(Some(sent))
            }
            None => Poll::Ready(None),
        }
    }

    #[inline]
    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    #[inline]
    fn size_hint(&self) -> http_body::SizeHint {
        let n = self.limit.saturating_sub(self.sent);
        let mut hint = self.body.size_hint();
        if hint.lower() >= n {
            hint.set_exact(n)
        } else if let Some(max) = hint.upper() {
            hint.set_upper(n.min(max))
        } else {
            hint.set_upper(n)
        }
        hint
    }
}

trait BodyExt {
    fn with_state<T>(self, state: T) -> BodyWithState<Self, T>
    where
        Self: Sized,
    {
        BodyWithState {
            body: self,
            _state: state,
        }
    }

    fn with_content_length<E>(
        self,
        limit: u64,
        error_tx: oneshot::Sender<E>,
        make_error: fn(Option<u64>) -> E,
    ) -> BodyWithContentLength<Self, E>
    where
        Self: Sized,
    {
        BodyWithContentLength {
            body: self,
            error_tx: Some(error_tx),
            make_error,
            limit,
            sent: 0,
        }
    }
}

impl<T> BodyExt for T {}

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

async fn io_task_result(
    rx: oneshot::Receiver<(
        Arc<AbortOnDropJoinHandle>,
        oneshot::Receiver<Result<(), ErrorCode>>,
    )>,
) -> Result<(), ErrorCode> {
    let Ok((_io, io_result_rx)) = rx.await else {
        return Ok(());
    };
    io_result_rx.await.unwrap_or(Ok(()))
}

impl HostWithStore for WasiHttp {
    async fn handle<T>(
        store: &Accessor<T, Self>,
        req: Resource<Request>,
    ) -> HttpResult<Resource<Response>> {
        // A handle to the I/O task, if spawned, will be sent on this channel
        // and kept as part of request body state
        let (io_task_tx, io_task_rx) = oneshot::channel();

        // A handle to the I/O task, if spawned, will be sent on this channel
        // along with the result receiver
        let (io_result_tx, io_result_rx) = oneshot::channel();

        // Response processing result will be sent on this channel
        let (res_result_tx, res_result_rx) = oneshot::channel();

        let getter = store.getter();
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
            // `Content-Length` header value is validated in `fields` implementation
            let content_length = match get_content_length(&headers) {
                Ok(content_length) => content_length,
                Err(err) => {
                    body.drop(&mut store);
                    return Err(ErrorCode::InternalError(Some(format!("{err:#}"))).into());
                }
            };
            let mut headers = Arc::unwrap_or_clone(headers);
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
                    io_task_result(io_result_rx),
                    content_length,
                    ErrorCode::HttpRequestBodySize,
                    getter,
                )
                .with_state(io_task_rx)
                .boxed(),
                Body::Host { body, result_tx } => {
                    if let Some(limit) = content_length {
                        let (http_result_tx, http_result_rx) = oneshot::channel();
                        _ = result_tx.send(Box::new(async move {
                            if let Ok(err) = http_result_rx.await {
                                return Err(err);
                            };
                            io_task_result(io_result_rx).await
                        }));
                        body.with_content_length(
                            limit,
                            http_result_tx,
                            ErrorCode::HttpRequestBodySize,
                        )
                        .with_state(io_task_rx)
                        .boxed()
                    } else {
                        _ = result_tx.send(Box::new(io_task_result(io_result_rx)));
                        body.with_state(io_task_rx).boxed()
                    }
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
                    // Forward the response processing result to `WasiHttpCtx` implementation
                    let Ok(fut) = res_result_rx.await else {
                        return Ok(());
                    };
                    Box::into_pin(fut).await
                }),
            ))
        })?;
        let (res, io) = Box::into_pin(fut).await?;
        let (
            http::response::Parts {
                status, headers, ..
            },
            body,
        ) = res.into_parts();

        let mut io = Box::into_pin(io);
        let body = match io.as_mut().poll(&mut Context::from_waker(Waker::noop()))? {
            Poll::Ready(()) => body,
            Poll::Pending => {
                // I/O driver still needs to be polled, spawn a task and send handles to it
                let (tx, rx) = oneshot::channel();
                let io = store.spawn(SendRequestTask { io, result_tx: tx });
                let io = Arc::new(AbortOnDropJoinHandle(io));
                _ = io_result_tx.send((Arc::clone(&io), rx));
                _ = io_task_tx.send(Arc::clone(&io));
                body.with_state(io).boxed()
            }
        };
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

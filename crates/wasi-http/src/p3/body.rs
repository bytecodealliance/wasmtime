use crate::p3::bindings::http::types::{ErrorCode, Fields, Trailers};
use crate::p3::{WasiHttp, WasiHttpCtxView};
use bytes::Bytes;
use core::iter;
use core::num::NonZeroUsize;
use core::pin::Pin;
use core::task::{Context, Poll, ready};
use http::HeaderMap;
use http_body::Body as _;
use http_body_util::combinators::UnsyncBoxBody;
use std::any::{Any, TypeId};
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::PollSender;
use wasmtime::component::{
    Access, Destination, FutureConsumer, FutureReader, Resource, Source, StreamConsumer,
    StreamProducer, StreamReader, StreamResult,
};
use wasmtime::error::Context as _;
use wasmtime::{AsContextMut, StoreContextMut};

/// The concrete type behind a `wasi:http/types.body` resource.
pub(crate) enum Body {
    /// Body constructed by the guest
    Guest {
        /// The body stream
        contents_rx: Option<StreamReader<u8>>,
        /// Future, on which guest will write result and optional trailers
        trailers_rx: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
        /// Channel, on which transmission result will be written
        result_tx: oneshot::Sender<Box<dyn Future<Output = Result<(), ErrorCode>> + Send>>,
    },
    /// Body constructed by the host.
    Host {
        /// The [`http_body::Body`]
        body: UnsyncBoxBody<Bytes, ErrorCode>,
        /// Channel, on which transmission result will be written
        result_tx: oneshot::Sender<Box<dyn Future<Output = Result<(), ErrorCode>> + Send>>,
    },
}

/// [FutureConsumer] implementation for future passed to `consume-body`.
struct BodyResultConsumer(
    Option<oneshot::Sender<Box<dyn Future<Output = Result<(), ErrorCode>> + Send>>>,
);

impl<D> FutureConsumer<D> for BodyResultConsumer
where
    D: 'static,
{
    type Item = Result<(), ErrorCode>;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        store: StoreContextMut<D>,
        mut src: Source<'_, Self::Item>,
        _: bool,
    ) -> Poll<wasmtime::Result<()>> {
        let mut res = None;
        src.read(store, &mut res).context("failed to read result")?;
        let res = res.context("result value missing")?;
        let tx = self.0.take().context("polled after returning `Ready`")?;
        _ = tx.send(Box::new(async { res }));
        Poll::Ready(Ok(()))
    }
}

impl Body {
    /// Implementation of `consume-body` shared between requests and responses
    pub(crate) fn consume<T>(
        self,
        mut store: Access<'_, T, WasiHttp>,
        fut: FutureReader<Result<(), ErrorCode>>,
        getter: fn(&mut T) -> WasiHttpCtxView<'_>,
    ) -> (
        StreamReader<u8>,
        FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
    ) {
        match self {
            Body::Guest {
                contents_rx: Some(contents_rx),
                trailers_rx,
                result_tx,
            } => {
                fut.pipe(&mut store, BodyResultConsumer(Some(result_tx)));
                (contents_rx, trailers_rx)
            }
            Body::Guest {
                contents_rx: None,
                trailers_rx,
                result_tx,
            } => {
                fut.pipe(&mut store, BodyResultConsumer(Some(result_tx)));
                (StreamReader::new(&mut store, iter::empty()), trailers_rx)
            }
            Body::Host { body, result_tx } => {
                fut.pipe(&mut store, BodyResultConsumer(Some(result_tx)));
                let (trailers_tx, trailers_rx) = oneshot::channel();
                (
                    StreamReader::new(
                        &mut store,
                        HostBodyStreamProducer {
                            body,
                            trailers: Some(trailers_tx),
                            getter,
                        },
                    ),
                    FutureReader::new(&mut store, trailers_rx),
                )
            }
        }
    }

    /// Implementation of `drop` shared between requests and responses
    pub(crate) fn drop(self, mut store: impl AsContextMut) {
        if let Body::Guest {
            contents_rx,
            mut trailers_rx,
            ..
        } = self
        {
            if let Some(mut contents_rx) = contents_rx {
                contents_rx.close(&mut store);
            }
            trailers_rx.close(store);
        }
    }
}

/// [StreamConsumer] implementation for bodies originating in the guest with `Content-Length`
/// header set.
struct LimitedGuestBodyConsumer {
    contents_tx: PollSender<Result<Bytes, ErrorCode>>,
    error_tx: Option<oneshot::Sender<ErrorCode>>,
    make_error: fn(Option<u64>) -> ErrorCode,
    /// Limit of bytes to be sent
    limit: u64,
    /// Number of bytes sent
    sent: u64,
    // `true` when the other side of `contents_tx` was unexpectedly closed
    closed: bool,
}

impl LimitedGuestBodyConsumer {
    /// Sends the error constructed by [Self::make_error] on both error channels.
    /// Does nothing if an error has already been sent on [Self::error_tx].
    fn send_error(&mut self, sent: Option<u64>) {
        if let Some(error_tx) = self.error_tx.take() {
            _ = error_tx.send((self.make_error)(sent));
            self.contents_tx.abort_send();
            if let Some(tx) = self.contents_tx.get_ref() {
                _ = tx.try_send(Err((self.make_error)(sent)))
            }
            self.contents_tx.close();
        }
    }
}

impl Drop for LimitedGuestBodyConsumer {
    fn drop(&mut self) {
        if !self.closed && self.limit != self.sent {
            self.send_error(Some(self.sent))
        }
    }
}

impl<D> StreamConsumer<D> for LimitedGuestBodyConsumer {
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        debug_assert!(!self.closed);
        let mut src = src.as_direct(store);
        let buf = src.remaining();
        let n = buf.len();

        // Perform `content-length` check early and precompute the next value
        let Ok(sent) = n.try_into() else {
            self.send_error(None);
            return Poll::Ready(Ok(StreamResult::Dropped));
        };
        let Some(sent) = self.sent.checked_add(sent) else {
            self.send_error(None);
            return Poll::Ready(Ok(StreamResult::Dropped));
        };
        if sent > self.limit {
            self.send_error(Some(sent));
            return Poll::Ready(Ok(StreamResult::Dropped));
        }
        match self.contents_tx.poll_reserve(cx) {
            Poll::Ready(Ok(())) => {
                let buf = Bytes::copy_from_slice(buf);
                match self.contents_tx.send_item(Ok(buf)) {
                    Ok(()) => {
                        src.mark_read(n);
                        // Record new `content-length` only on successful send
                        self.sent = sent;
                        Poll::Ready(Ok(StreamResult::Completed))
                    }
                    Err(..) => {
                        self.closed = true;
                        Poll::Ready(Ok(StreamResult::Dropped))
                    }
                }
            }
            Poll::Ready(Err(..)) => {
                self.closed = true;
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// [StreamConsumer] implementation for bodies originating in the guest without `Content-Length`
/// header set.
struct UnlimitedGuestBodyConsumer(PollSender<Result<Bytes, ErrorCode>>);

impl<D> StreamConsumer<D> for UnlimitedGuestBodyConsumer {
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        match self.0.poll_reserve(cx) {
            Poll::Ready(Ok(())) => {
                let mut src = src.as_direct(store);
                let buf = src.remaining();
                let n = buf.len();
                let buf = Bytes::copy_from_slice(buf);
                match self.0.send_item(Ok(buf)) {
                    Ok(()) => {
                        src.mark_read(n);
                        Poll::Ready(Ok(StreamResult::Completed))
                    }
                    Err(..) => Poll::Ready(Ok(StreamResult::Dropped)),
                }
            }
            Poll::Ready(Err(..)) => Poll::Ready(Ok(StreamResult::Dropped)),
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// [http_body::Body] implementation for bodies originating in the guest.
pub(crate) struct GuestBody {
    contents_rx: Option<mpsc::Receiver<Result<Bytes, ErrorCode>>>,
    trailers_rx: Option<oneshot::Receiver<Result<Option<Arc<http::HeaderMap>>, ErrorCode>>>,
    content_length: Option<u64>,
}

impl GuestBody {
    /// Construct a new [GuestBody]
    pub(crate) fn new<T: 'static>(
        mut store: impl AsContextMut<Data = T>,
        contents_rx: Option<StreamReader<u8>>,
        trailers_rx: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
        result_tx: oneshot::Sender<Box<dyn Future<Output = Result<(), ErrorCode>> + Send>>,
        result_fut: impl Future<Output = Result<(), ErrorCode>> + Send + 'static,
        content_length: Option<u64>,
        make_error: fn(Option<u64>) -> ErrorCode,
        getter: fn(&mut T) -> WasiHttpCtxView<'_>,
    ) -> Self {
        let (trailers_http_tx, trailers_http_rx) = oneshot::channel();
        trailers_rx.pipe(
            &mut store,
            GuestTrailerConsumer {
                tx: Some(trailers_http_tx),
                getter,
            },
        );

        let contents_rx = if let Some(rx) = contents_rx {
            let (http_tx, http_rx) = mpsc::channel(1);
            let contents_tx = PollSender::new(http_tx);
            if let Some(limit) = content_length {
                let (error_tx, error_rx) = oneshot::channel();
                _ = result_tx.send(Box::new(async move {
                    if let Ok(err) = error_rx.await {
                        return Err(err);
                    };
                    result_fut.await
                }));
                rx.pipe(
                    store,
                    LimitedGuestBodyConsumer {
                        contents_tx,
                        error_tx: Some(error_tx),
                        make_error,
                        limit,
                        sent: 0,
                        closed: false,
                    },
                );
            } else {
                _ = result_tx.send(Box::new(result_fut));
                rx.pipe(store, UnlimitedGuestBodyConsumer(contents_tx));
            };
            Some(http_rx)
        } else {
            _ = result_tx.send(Box::new(result_fut));
            None
        };
        Self {
            trailers_rx: Some(trailers_http_rx),
            contents_rx,
            content_length,
        }
    }
}

impl http_body::Body for GuestBody {
    type Data = Bytes;
    type Error = ErrorCode;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        if let Some(contents_rx) = self.contents_rx.as_mut() {
            // `contents_rx` has not been closed yet, poll it
            while let Some(res) = ready!(contents_rx.poll_recv(cx)) {
                match res {
                    Ok(buf) => {
                        if let Some(n) = self.content_length.as_mut() {
                            // Subtract frame length from `content_length`,
                            // [LimitedGuestBodyConsumer] already performs the validation, so
                            // just keep count as optimization for
                            // `is_end_stream` and `size_hint`
                            *n = n.saturating_sub(buf.len().try_into().unwrap_or(u64::MAX));
                        }
                        return Poll::Ready(Some(Ok(http_body::Frame::data(buf))));
                    }
                    Err(err) => {
                        return Poll::Ready(Some(Err(err)));
                    }
                }
            }
            // Record that `contents_rx` is closed
            self.contents_rx = None;
        }

        let Some(trailers_rx) = self.trailers_rx.as_mut() else {
            // `trailers_rx` has already terminated - this is the end of stream
            return Poll::Ready(None);
        };

        let res = ready!(Pin::new(trailers_rx).poll(cx));
        // Record that `trailers_rx` has terminated
        self.trailers_rx = None;
        match res {
            Ok(Ok(Some(trailers))) => Poll::Ready(Some(Ok(http_body::Frame::trailers(
                Arc::unwrap_or_clone(trailers),
            )))),
            Ok(Ok(None)) => Poll::Ready(None),
            Ok(Err(err)) => Poll::Ready(Some(Err(err))),
            Err(..) => Poll::Ready(None),
        }
    }

    fn is_end_stream(&self) -> bool {
        if let Some(contents_rx) = self.contents_rx.as_ref() {
            if !contents_rx.is_empty()
                || !contents_rx.is_closed()
                || self.content_length.is_some_and(|n| n > 0)
            {
                // `contents_rx` might still produce data frames
                return false;
            }
        }
        if let Some(trailers_rx) = self.trailers_rx.as_ref() {
            if !trailers_rx.is_terminated() {
                // `trailers_rx` has not terminated yet
                return false;
            }
        }

        // no data left
        return true;
    }

    fn size_hint(&self) -> http_body::SizeHint {
        if let Some(n) = self.content_length {
            http_body::SizeHint::with_exact(n)
        } else {
            http_body::SizeHint::default()
        }
    }
}

/// [FutureConsumer] implementation for trailers originating in the guest.
struct GuestTrailerConsumer<T> {
    tx: Option<oneshot::Sender<Result<Option<Arc<HeaderMap>>, ErrorCode>>>,
    getter: fn(&mut T) -> WasiHttpCtxView<'_>,
}

impl<D> FutureConsumer<D> for GuestTrailerConsumer<D>
where
    D: 'static,
{
    type Item = Result<Option<Resource<Trailers>>, ErrorCode>;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        mut store: StoreContextMut<D>,
        mut src: Source<'_, Self::Item>,
        _: bool,
    ) -> Poll<wasmtime::Result<()>> {
        let mut res = None;
        src.read(&mut store, &mut res)
            .context("failed to read result")?;
        let res = match res.context("result value missing")? {
            Ok(Some(trailers)) => {
                let WasiHttpCtxView { table, .. } = (self.getter)(store.data_mut());
                let trailers = table
                    .delete(trailers)
                    .context("failed to delete trailers")?;
                Ok(Some(Arc::from(trailers)))
            }
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        };
        _ = self.tx.take().unwrap().send(res);
        Poll::Ready(Ok(()))
    }
}

/// [StreamProducer] implementation for bodies originating in the host.
pub(crate) struct HostBodyStreamProducer<T> {
    pub(crate) body: UnsyncBoxBody<Bytes, ErrorCode>,
    trailers: Option<oneshot::Sender<Result<Option<Resource<Trailers>>, ErrorCode>>>,
    getter: fn(&mut T) -> WasiHttpCtxView<'_>,
}

impl<T> Drop for HostBodyStreamProducer<T> {
    fn drop(&mut self) {
        self.close(Ok(None))
    }
}

impl<T> HostBodyStreamProducer<T> {
    fn close(&mut self, res: Result<Option<Resource<Trailers>>, ErrorCode>) {
        if let Some(tx) = self.trailers.take() {
            _ = tx.send(res);
        }
    }
}

impl<D> StreamProducer<D> for HostBodyStreamProducer<D>
where
    D: 'static,
{
    type Item = u8;
    type Buffer = Cursor<Bytes>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'a, D>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let res = 'result: {
            let cap = match dst.remaining(&mut store).map(NonZeroUsize::new) {
                Some(Some(cap)) => Some(cap),
                Some(None) => {
                    // On 0-length the best we can do is check that underlying stream has not
                    // reached the end yet
                    if self.body.is_end_stream() {
                        break 'result Ok(None);
                    } else {
                        return Poll::Ready(Ok(StreamResult::Completed));
                    }
                }
                None => None,
            };
            match Pin::new(&mut self.body).poll_frame(cx) {
                Poll::Ready(Some(Ok(frame))) => {
                    match frame.into_data().map_err(http_body::Frame::into_trailers) {
                        Ok(mut frame) => {
                            if let Some(cap) = cap {
                                let n = frame.len();
                                let cap = cap.into();
                                if n > cap {
                                    // data frame does not fit in destination, fill it and buffer the rest
                                    dst.set_buffer(Cursor::new(frame.split_off(cap)));
                                    let mut dst = dst.as_direct(store, cap);
                                    dst.remaining().copy_from_slice(&frame);
                                    dst.mark_written(cap);
                                } else {
                                    // copy the whole frame into the destination
                                    let mut dst = dst.as_direct(store, n);
                                    dst.remaining()[..n].copy_from_slice(&frame);
                                    dst.mark_written(n);
                                }
                            } else {
                                dst.set_buffer(Cursor::new(frame));
                            }
                            return Poll::Ready(Ok(StreamResult::Completed));
                        }
                        Err(Ok(trailers)) => {
                            let trailers = (self.getter)(store.data_mut())
                                .table
                                .push(Fields::new_mutable(trailers))
                                .context("failed to push trailers to table")?;
                            break 'result Ok(Some(trailers));
                        }
                        Err(Err(..)) => break 'result Err(ErrorCode::HttpProtocolError),
                    }
                }
                Poll::Ready(Some(Err(err))) => break 'result Err(err),
                Poll::Ready(None) => break 'result Ok(None),
                Poll::Pending if finish => return Poll::Ready(Ok(StreamResult::Cancelled)),
                Poll::Pending => return Poll::Pending,
            }
        };
        self.close(res);
        Poll::Ready(Ok(StreamResult::Dropped))
    }

    fn try_into(me: Pin<Box<Self>>, ty: TypeId) -> Result<Box<dyn Any>, Pin<Box<Self>>> {
        if ty == TypeId::of::<Self>() {
            let me = Pin::into_inner(me);
            Ok(me)
        } else {
            Err(me)
        }
    }
}

/// A wrapper around [http_body::Body], which allows attaching arbitrary state to it
pub(crate) struct BodyWithState<T, U> {
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
pub(crate) struct BodyWithContentLength<T, E> {
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

pub(crate) trait BodyExt {
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

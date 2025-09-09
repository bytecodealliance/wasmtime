use crate::p3::bindings::http::types::{ErrorCode, Fields, Trailers};
use crate::p3::{WasiHttp, WasiHttpCtxView};
use anyhow::Context as _;
use bytes::Bytes;
use core::num::NonZeroUsize;
use core::pin::Pin;
use core::task::{Context, Poll, ready};
use http::HeaderMap;
use http_body::Body as _;
use http_body_util::combinators::BoxBody;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::PollSender;
use wasmtime::component::{
    Access, Destination, FutureConsumer, FutureReader, Resource, Source, StreamConsumer,
    StreamProducer, StreamReader, StreamResult,
};
use wasmtime::{AsContextMut, StoreContextMut};
use wasmtime_wasi::p3::{FutureOneshotProducer, StreamEmptyProducer};

/// The concrete type behind a `wasi:http/types/body` resource.
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
        body: BoxBody<Bytes, ErrorCode>,
        /// Channel, on which transmission result will be written
        result_tx: oneshot::Sender<Box<dyn Future<Output = Result<(), ErrorCode>> + Send>>,
    },
    /// Body is consumed.
    Consumed,
}

impl Body {
    pub(crate) fn consume<T>(
        self,
        mut store: Access<'_, T, WasiHttp>,
        getter: fn(&mut T) -> WasiHttpCtxView<'_>,
    ) -> Result<
        (
            StreamReader<u8>,
            FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
        ),
        (),
    > {
        match self {
            Body::Guest {
                contents_rx: Some(contents_rx),
                trailers_rx,
                result_tx,
            } => {
                // TODO: Use a result specified by the caller
                // https://github.com/WebAssembly/wasi-http/issues/176
                _ = result_tx.send(Box::new(async { Ok(()) }));
                Ok((contents_rx, trailers_rx))
            }
            Body::Guest {
                contents_rx: None,
                trailers_rx,
                result_tx,
            } => {
                let instance = store.instance();
                // TODO: Use a result specified by the caller
                // https://github.com/WebAssembly/wasi-http/issues/176
                _ = result_tx.send(Box::new(async { Ok(()) }));
                Ok((
                    StreamReader::new(instance, &mut store, StreamEmptyProducer::default()),
                    trailers_rx,
                ))
            }
            Body::Host { body, result_tx } => {
                let instance = store.instance();
                // TODO: Use a result specified by the caller
                // https://github.com/WebAssembly/wasi-http/issues/176
                _ = result_tx.send(Box::new(async { Ok(()) }));
                let (trailers_tx, trailers_rx) = oneshot::channel();
                Ok((
                    StreamReader::new(
                        instance,
                        &mut store,
                        HostBodyStreamProducer {
                            body,
                            trailers: Some(trailers_tx),
                            getter,
                        },
                    ),
                    FutureReader::new(
                        instance,
                        &mut store,
                        FutureOneshotProducer::from(trailers_rx),
                    ),
                ))
            }
            Body::Consumed => Err(()),
        }
    }

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

pub(crate) enum GuestBodyKind {
    Request,
    Response,
}

/// Represents `Content-Length` limit and state
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct ContentLength {
    /// Limit of bytes to be sent
    limit: u64,
    /// Number of bytes sent
    sent: u64,
}

impl ContentLength {
    /// Constructs new [ContentLength]
    fn new(limit: u64) -> Self {
        Self { limit, sent: 0 }
    }
}

struct GuestBodyConsumer {
    contents_tx: PollSender<Result<Bytes, ErrorCode>>,
    result_tx: Option<oneshot::Sender<Result<(), ErrorCode>>>,
    content_length: Option<ContentLength>,
    kind: GuestBodyKind,
    // `true` when the other side of `contents_tx` was unexpectedly closed
    closed: bool,
}

impl GuestBodyConsumer {
    fn body_size_error(&self, n: Option<u64>) -> ErrorCode {
        match self.kind {
            GuestBodyKind::Request => ErrorCode::HttpRequestBodySize(n),
            GuestBodyKind::Response => ErrorCode::HttpResponseBodySize(n),
        }
    }

    // Sends the corresponding error constructed by [Self::body_size_error] on both
    // error channels.
    // [`PollSender::poll_reserve`] on `contents_tx` must have succeeed prior to this being called.
    fn send_body_size_error(&mut self, n: Option<u64>) {
        if let Some(result_tx) = self.result_tx.take() {
            _ = result_tx.send(Err(self.body_size_error(n)));
            _ = self.contents_tx.send_item(Err(self.body_size_error(n)));
        }
    }
}

impl Drop for GuestBodyConsumer {
    fn drop(&mut self) {
        if let Some(result_tx) = self.result_tx.take() {
            if let Some(ContentLength { limit, sent }) = self.content_length {
                if !self.closed && limit != sent {
                    _ = result_tx.send(Err(self.body_size_error(Some(sent))));
                    self.contents_tx.abort_send();
                    if let Some(tx) = self.contents_tx.get_ref() {
                        _ = tx.try_send(Err(self.body_size_error(Some(sent))))
                    }
                }
            }
        }
    }
}

impl<D> StreamConsumer<D> for GuestBodyConsumer {
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        debug_assert!(!self.closed);
        match self.contents_tx.poll_reserve(cx) {
            Poll::Ready(Ok(())) => {
                let mut src = src.as_direct(store);
                let buf = src.remaining();
                if let Some(ContentLength { limit, sent }) = self.content_length.as_mut() {
                    let Some(n) = buf.len().try_into().ok().and_then(|n| sent.checked_add(n))
                    else {
                        self.send_body_size_error(None);
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    };
                    if n > *limit {
                        self.send_body_size_error(Some(n));
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    }
                    *sent = n;
                }
                let buf = Bytes::copy_from_slice(buf);
                let n = buf.len();
                match self.contents_tx.send_item(Ok(buf)) {
                    Ok(()) => {
                        src.mark_read(n);
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

pub(crate) struct GuestBody {
    contents_rx: Option<mpsc::Receiver<Result<Bytes, ErrorCode>>>,
    trailers_rx: Option<oneshot::Receiver<Result<Option<Arc<http::HeaderMap>>, ErrorCode>>>,
    content_length: Option<u64>,
}

impl GuestBody {
    pub(crate) fn new<T: 'static>(
        mut store: impl AsContextMut<Data = T>,
        contents_rx: Option<StreamReader<u8>>,
        trailers_rx: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
        result_tx: oneshot::Sender<Result<(), ErrorCode>>,
        content_length: Option<u64>,
        kind: GuestBodyKind,
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
        let contents_rx = contents_rx.map(|rx| {
            let (http_tx, http_rx) = mpsc::channel(1);
            rx.pipe(
                store,
                GuestBodyConsumer {
                    contents_tx: PollSender::new(http_tx),
                    result_tx: Some(result_tx),
                    content_length: content_length.map(ContentLength::new),
                    kind,
                    closed: false,
                },
            );
            http_rx
        });
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
            while let Some(res) = ready!(contents_rx.poll_recv(cx)) {
                match res {
                    Ok(buf) => {
                        if let Some(n) = self.content_length.as_mut() {
                            *n = n.saturating_sub(buf.len().try_into().unwrap_or(u64::MAX));
                        }
                        return Poll::Ready(Some(Ok(http_body::Frame::data(buf))));
                    }
                    Err(err) => {
                        return Poll::Ready(Some(Err(err)));
                    }
                }
            }
            self.contents_rx = None;
        }

        let Some(trailers_rx) = self.trailers_rx.as_mut() else {
            return Poll::Ready(None);
        };

        let res = ready!(Pin::new(trailers_rx).poll(cx));
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
                return false;
            }
        }
        if let Some(trailers_rx) = self.trailers_rx.as_ref() {
            if !trailers_rx.is_terminated() {
                return false;
            }
        }
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

pub(crate) struct ConsumedBody;

impl http_body::Body for ConsumedBody {
    type Data = Bytes;
    type Error = ErrorCode;

    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        Poll::Ready(Some(Err(ErrorCode::InternalError(Some(
            "body consumed".into(),
        )))))
    }

    fn is_end_stream(&self) -> bool {
        true
    }

    fn size_hint(&self) -> http_body::SizeHint {
        http_body::SizeHint::with_exact(0)
    }
}

pub(crate) struct GuestTrailerConsumer<T> {
    pub(crate) tx: Option<oneshot::Sender<Result<Option<Arc<HeaderMap>>, ErrorCode>>>,
    pub(crate) getter: fn(&mut T) -> WasiHttpCtxView<'_>,
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
        mut source: Source<'_, Self::Item>,
        _: bool,
    ) -> Poll<wasmtime::Result<()>> {
        let value = &mut None;
        source.read(store.as_context_mut(), value)?;
        let res = match value.take().unwrap() {
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

struct HostBodyStreamProducer<T> {
    body: BoxBody<Bytes, ErrorCode>,
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
                                    dst.set_buffer(Cursor::new(frame.split_off(cap)));
                                    let mut dst = dst.as_direct(store, cap);
                                    dst.remaining().copy_from_slice(&frame);
                                    dst.mark_written(cap);
                                } else {
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
}

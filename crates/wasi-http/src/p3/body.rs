use crate::p3::WasiHttpCtxView;
use crate::p3::bindings::http::types::{ErrorCode, Trailers};
use anyhow::Context as _;
use bytes::Bytes;
use core::pin::Pin;
use core::task::{Context, Poll, ready};
use http::HeaderMap;
use http_body_util::combinators::BoxBody;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::PollSender;
use wasmtime::component::{
    FutureConsumer, FutureReader, Resource, Source, StreamConsumer, StreamReader, StreamResult,
};
use wasmtime::{AsContextMut, StoreContextMut};

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
        body: BoxBody<Bytes, ErrorCode>,
        /// Channel, on which transmission result will be written
        result_tx: oneshot::Sender<Box<dyn Future<Output = Result<(), ErrorCode>> + Send>>,
    },
    /// Body is consumed.
    Consumed,
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
    result_tx: mpsc::Sender<Result<(), ErrorCode>>,
    content_length: Option<ContentLength>,
    kind: GuestBodyKind,
}

impl GuestBodyConsumer {
    fn body_size_error(&self, n: Option<u64>) -> ErrorCode {
        match self.kind {
            GuestBodyKind::Request => ErrorCode::HttpRequestBodySize(n),
            GuestBodyKind::Response => ErrorCode::HttpResponseBodySize(n),
        }
    }
}

impl Drop for GuestBodyConsumer {
    fn drop(&mut self) {
        if let Some(ContentLength { limit, sent }) = self.content_length {
            if limit != sent {
                _ = self
                    .result_tx
                    .try_send(Err(self.body_size_error(Some(sent))));
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
        match self.contents_tx.poll_reserve(cx) {
            Poll::Ready(Ok(())) => {
                let mut src = src.as_direct(store);
                let buf = src.remaining();
                if let Some(ContentLength { limit, sent }) = self.content_length.as_mut() {
                    let Ok(n) = buf.len().try_into() else {
                        _ = self.result_tx.try_send(Err(self.body_size_error(None)));
                        let err = self.body_size_error(None);
                        _ = self.contents_tx.send_item(Err(err));
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    };
                    let Some(n) = sent.checked_add(n) else {
                        _ = self.result_tx.try_send(Err(self.body_size_error(None)));
                        let err = self.body_size_error(None);
                        _ = self.contents_tx.send_item(Err(err));
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    };
                    if n > *limit {
                        _ = self.result_tx.try_send(Err(self.body_size_error(Some(n))));
                        let err = self.body_size_error(Some(n));
                        _ = self.contents_tx.send_item(Err(err));
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
                    Err(..) => Poll::Ready(Ok(StreamResult::Dropped)),
                }
            }
            Poll::Ready(Err(..)) => Poll::Ready(Ok(StreamResult::Dropped)),
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
    pub fn new<T: 'static>(
        mut store: impl AsContextMut<Data = T>,
        contents_rx: Option<StreamReader<u8>>,
        trailers_rx: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
        result_tx: mpsc::Sender<Result<(), ErrorCode>>,
        content_length: Option<u64>,
        kind: GuestBodyKind,
        getter: for<'a> fn(&'a mut T) -> WasiHttpCtxView<'a>,
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
                    result_tx,
                    content_length: content_length.map(ContentLength::new),
                    kind,
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
    pub(crate) getter: for<'a> fn(&'a mut T) -> WasiHttpCtxView<'a>,
}

impl<D> FutureConsumer<D> for GuestTrailerConsumer<D>
where
    D: 'static,
{
    type Item = Result<Option<Resource<Trailers>>, ErrorCode>;

    fn poll_consume(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        mut store: StoreContextMut<D>,
        mut source: Source<'_, Self::Item>,
        _: bool,
    ) -> Poll<wasmtime::Result<()>> {
        let value = &mut None;
        source.read(store.as_context_mut(), value)?;
        let res = value.take().unwrap();
        let me = self.get_mut();
        match res {
            Ok(Some(trailers)) => {
                let WasiHttpCtxView { table, .. } = (me.getter)(store.data_mut());
                let trailers = table
                    .delete(trailers)
                    .context("failed to delete trailers")?;
                _ = me.tx.take().unwrap().send(Ok(Some(Arc::from(trailers))));
            }
            Ok(None) => {
                _ = me.tx.take().unwrap().send(Ok(None));
            }
            Err(err) => {
                _ = me.tx.take().unwrap().send(Err(err));
            }
        }
        Poll::Ready(Ok(()))
    }
}

pub(crate) struct IncomingResponseBody {
    pub incoming: hyper::body::Incoming,
    pub timeout: tokio::time::Interval,
}

impl http_body::Body for IncomingResponseBody {
    type Data = <hyper::body::Incoming as http_body::Body>::Data;
    type Error = ErrorCode;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        match Pin::new(&mut self.as_mut().incoming).poll_frame(cx) {
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Err(err))) => {
                Poll::Ready(Some(Err(ErrorCode::from_hyper_response_error(err))))
            }
            Poll::Ready(Some(Ok(frame))) => {
                self.timeout.reset();
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Pending => {
                ready!(self.timeout.poll_tick(cx));
                Poll::Ready(Some(Err(ErrorCode::ConnectionReadTimeout)))
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.incoming.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.incoming.size_hint()
    }
}

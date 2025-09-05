use crate::p3::bindings::http::types::{ErrorCode, Trailers};
use crate::p3::{WasiHttp, WasiHttpCtxView};
use anyhow::Context as _;
use bytes::Bytes;
use core::pin::Pin;
use core::task::{Context, Poll, ready};
use http::HeaderMap;
use http_body_util::combinators::BoxBody;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::PollSender;
use wasmtime::StoreContextMut;
use wasmtime::component::{
    Accessor, FutureConsumer, FutureReader, Resource, Source, StreamConsumer, StreamReader,
    StreamResult,
};

/// The concrete type behind a `wasi:http/types/body` resource.
pub(crate) enum Body {
    /// Body constructed by the guest
    Guest {
        /// The body stream
        contents_rx: Option<StreamReader<u8>>,
        /// Future, on which guest will write result and optional trailers
        trailers_rx: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
        /// Channel, on which transmission result will be written
        result_tx: oneshot::Sender<Result<(), ErrorCode>>,
    },
    /// Body constructed by the host.
    Host(BoxBody<Bytes, ErrorCode>),
    /// Body is consumed.
    Consumed,
}

pub(crate) struct GuestBodyConsumer {
    pub(crate) tx: PollSender<Bytes>,
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
        match self.tx.poll_reserve(cx) {
            Poll::Ready(Ok(())) => {
                let mut src = src.as_direct(store);
                let buf = Bytes::copy_from_slice(src.remaining());
                let n = buf.len();
                match self.tx.send_item(buf) {
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
    pub(crate) contents_rx: Option<mpsc::Receiver<Bytes>>,
    pub(crate) trailers_rx:
        Option<oneshot::Receiver<Result<Option<Arc<http::HeaderMap>>, ErrorCode>>>,
}

impl http_body::Body for GuestBody {
    type Data = Bytes;
    type Error = ErrorCode;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        if let Some(contents_rx) = self.contents_rx.as_mut() {
            while let Some(buf) = ready!(contents_rx.poll_recv(cx)) {
                return Poll::Ready(Some(Ok(http_body::Frame::data(buf))));
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
            if !contents_rx.is_empty() || !contents_rx.is_closed() {
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
        // TODO: use content-length
        http_body::SizeHint::default()
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
    pub(crate) tx: oneshot::Sender<Result<Option<Arc<HeaderMap>>, ErrorCode>>,
    pub(crate) getter: for<'a> fn(&'a mut T) -> WasiHttpCtxView<'a>,
}

impl<D> FutureConsumer<D> for GuestTrailerConsumer<D>
where
    D: 'static,
{
    type Item = Result<Option<Resource<Trailers>>, ErrorCode>;

    async fn consume(self, store: &Accessor<D>, res: Self::Item) -> wasmtime::Result<()> {
        match res {
            Ok(Some(trailers)) => store
                .with_getter::<WasiHttp>(self.getter)
                .with(|mut store| {
                    let WasiHttpCtxView { table, .. } = store.get();
                    let trailers = table
                        .delete(trailers)
                        .context("failed to delete trailers")?;
                    _ = self.tx.send(Ok(Some(Arc::from(trailers))));
                    Ok(())
                }),
            Ok(None) => {
                _ = self.tx.send(Ok(None));
                Ok(())
            }
            Err(err) => {
                _ = self.tx.send(Err(err));
                Ok(())
            }
        }
    }
}

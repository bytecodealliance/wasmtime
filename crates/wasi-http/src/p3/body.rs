use crate::p3::WasiHttpView;
use crate::p3::bindings::http::types::{ErrorCode, Trailers};
use anyhow::Context as _;
use bytes::{Bytes, BytesMut};
use core::future::poll_fn;
use core::pin::{Pin, pin};
use core::task::{Context, Poll, ready};
use http_body_util::combinators::BoxBody;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use wasmtime::component::{
    Accessor, AccessorTask, FutureReader, FutureWriter, GuardedFutureReader, GuardedFutureWriter,
    GuardedStreamReader, HasData, Resource, StreamReader,
};

/// The concrete type behind a `wasi:http/types/body` resource.
pub(crate) enum Body {
    /// Body constructed by the guest
    Guest(GuestBodyContext),
    /// Body constructed by the host.
    Host(BoxBody<Bytes, ErrorCode>),
    /// Body is consumed.
    Consumed,
}

/// Context of a body constructed by the guest
pub struct GuestBodyContext {
    /// The body stream
    pub(crate) contents_rx: Option<StreamReader<u8>>,
    /// Future, on which guest will write result and optional trailers
    pub(crate) trailers_rx: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
    /// Future, on which transmission result will be written
    pub(crate) result_tx: FutureWriter<Result<(), ErrorCode>>,
}

pub struct GuestBodyTaskContext {
    pub(crate) cx: GuestBodyContext,
    pub(crate) contents_tx: mpsc::Sender<Bytes>,
    pub(crate) trailers_tx: oneshot::Sender<Result<Option<Arc<http::HeaderMap>>, ErrorCode>>,
}

impl GuestBodyTaskContext {
    /// Consume the body given an I/O operation `io`.
    ///
    /// This function returns a [GuestBodyTask], which implements a [AccessorTask] and
    /// must be run using the engine's event loop.
    pub fn consume<Fut>(self, io: Fut) -> GuestBodyTask<Fut>
    where
        Fut: Future<Output = Result<(), ErrorCode>>,
    {
        GuestBodyTask { cx: self, io }
    }
}

pub struct GuestBodyTask<T> {
    cx: GuestBodyTaskContext,
    io: T,
}

impl<T, U, Fut> AccessorTask<T, U, wasmtime::Result<()>> for GuestBodyTask<Fut>
where
    T: WasiHttpView,
    U: HasData,
    Fut: Future<Output = Result<(), ErrorCode>> + Send + 'static,
{
    async fn run(self, store: &Accessor<T, U>) -> wasmtime::Result<()> {
        let Self {
            cx:
                GuestBodyTaskContext {
                    cx:
                        GuestBodyContext {
                            contents_rx,
                            trailers_rx,
                            result_tx,
                        },
                    contents_tx,
                    mut trailers_tx,
                },
            io,
        } = self;
        let trailers_rx = GuardedFutureReader::new(store, trailers_rx);
        let mut result_tx = GuardedFutureWriter::new(store, result_tx);
        if let Some(contents_rx) = contents_rx {
            let mut contents_rx = GuardedStreamReader::new(store, contents_rx);
            // TODO: use content-length
            let mut buf = BytesMut::with_capacity(8192);
            while !contents_rx.is_closed() {
                let mut tx = pin!(contents_tx.reserve());
                let Some(Ok(tx)) = ({
                    let mut contents_tx_dropped = pin!(contents_rx.watch_writer());
                    poll_fn(|cx| match contents_tx_dropped.as_mut().poll(cx) {
                        Poll::Ready(()) => return Poll::Ready(None),
                        Poll::Pending => tx.as_mut().poll(cx).map(Some),
                    })
                    .await
                }) else {
                    // Either:
                    // - body receiver has been closed
                    // - guest writer has been closed
                    break;
                };
                buf = contents_rx.read(buf).await;
                if !buf.is_empty() {
                    tx.send(buf.split().freeze());
                }
            }
        }
        drop(contents_tx);

        let mut rx = pin!(trailers_rx.read());
        match poll_fn(|cx| match trailers_tx.poll_closed(cx) {
            Poll::Ready(()) => return Poll::Ready(None),
            Poll::Pending => rx.as_mut().poll(cx).map(Some),
        })
        .await
        {
            Some(Some(Ok(Some(trailers)))) => {
                let trailers = store.with(|mut store| {
                    store
                        .data_mut()
                        .http()
                        .table
                        .delete(trailers)
                        .context("failed to delete trailers")
                })?;
                _ = trailers_tx.send(Ok(Some(trailers.into())));
            }
            Some(Some(Ok(None))) => {
                _ = trailers_tx.send(Ok(None));
            }
            Some(Some(Err(err))) => {
                _ = trailers_tx.send(Err(err));
            }
            Some(None) | None => {
                // Either:
                // - trailer receiver has been closed
                // - guest writer has been closed
                drop(trailers_tx);
            }
        }

        let mut io = pin!(io);
        if let Some(res) = {
            let mut result_rx_dropped = pin!(result_tx.watch_reader());
            poll_fn(|cx| match result_rx_dropped.as_mut().poll(cx) {
                Poll::Ready(()) => return Poll::Ready(None),
                Poll::Pending => io.as_mut().poll(cx).map(Some),
            })
            .await
        } {
            result_tx.write(res).await;
        }
        Ok(())
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

use bytes::BytesMut;
use std::io::Cursor;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::oneshot;
use wasmtime::StoreContextMut;
use wasmtime::component::{Destination, Source, StreamConsumer, StreamProducer, StreamResult};

use crate::p3::util::pipe;

/// A [StreamProducer] for an [AsyncRead] stream.
///
/// Once the inner `AsyncRead` returns EOF or an error, the producer will be
/// dropped and the final result is submitted to the `dropped` channel.
pub(crate) struct AsyncReadProducer<IO>(Option<(IO, oneshot::Sender<std::io::Result<IO>>)>);
impl<IO> AsyncReadProducer<IO> {
    pub(crate) fn new(io: IO, dropped: oneshot::Sender<std::io::Result<IO>>) -> Self {
        Self(Some((io, dropped)))
    }
    fn io(&mut self) -> Option<&mut IO> {
        self.0.as_mut().map(|(io, _)| io)
    }
    fn notify_dropped(&mut self, result: std::io::Result<()>) {
        if let Some((io, dropped)) = self.0.take() {
            let _ = dropped.send(result.map(|()| io));
        }
    }
}
impl<IO> Drop for AsyncReadProducer<IO> {
    fn drop(&mut self) {
        self.notify_dropped(Ok(()));
    }
}
impl<D, IO> StreamProducer<D> for AsyncReadProducer<IO>
where
    IO: AsyncRead + Send + Unpin + 'static,
{
    type Item = u8;
    type Buffer = Cursor<BytesMut>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let Some(io) = self.io() else {
            return Poll::Ready(Ok(StreamResult::Dropped));
        };

        let mut dst = dst.as_direct(store, pipe::CAPACITY);
        let remaining = dst.remaining();

        // A zero-remaining destination means the guest is waiting for
        // readiness rather than requesting data. AsyncRead has no
        // separate readiness API, so we lie and say "ready" here; the
        // next actual poll will read data.
        //
        // See WebAssembly/component-model#561 for background.
        if remaining.is_empty() {
            return Poll::Ready(Ok(StreamResult::Completed));
        }

        let mut buf = ReadBuf::new(remaining);
        match Pin::new(io).poll_read(cx, &mut buf) {
            Poll::Ready(Ok(())) if buf.filled().is_empty() => {
                // EOF — signal clean completion.
                self.notify_dropped(Ok(()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Ready(Ok(())) => {
                let n = buf.filled().len();
                dst.mark_written(n);
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Poll::Ready(Err(e)) => {
                self.notify_dropped(Err(e));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A [StreamConsumer] for an [AsyncWrite] stream.
///
/// Once the inner `AsyncWrite` returns an error, the consumer will be dropped
/// and the final result is submitted to the `dropped` channel.
pub(crate) struct AsyncWriteConsumer<IO>(Option<(IO, oneshot::Sender<std::io::Result<IO>>)>);
impl<IO> AsyncWriteConsumer<IO> {
    pub(crate) fn new(io: IO, dropped: oneshot::Sender<std::io::Result<IO>>) -> Self {
        Self(Some((io, dropped)))
    }
    fn io(&mut self) -> Option<&mut IO> {
        self.0.as_mut().map(|(io, _)| io)
    }
    fn notify_dropped(&mut self, result: std::io::Result<()>) {
        if let Some((io, dropped)) = self.0.take() {
            let _ = dropped.send(result.map(|()| io));
        }
    }
}
impl<IO> Drop for AsyncWriteConsumer<IO> {
    fn drop(&mut self) {
        self.notify_dropped(Ok(()));
    }
}
impl<D, IO> StreamConsumer<D> for AsyncWriteConsumer<IO>
where
    IO: AsyncWrite + Send + Unpin + 'static,
{
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: Source<'_, Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let Some(io) = self.io() else {
            return Poll::Ready(Ok(StreamResult::Dropped));
        };

        let mut src = src.as_direct(store);
        let remaining = src.remaining();

        if remaining.is_empty() {
            // Zero-length consume = readiness check. AsyncWrite has no
            // dedicated readiness API so we report ready; the next poll will
            // perform the actual write.
            //
            // See WebAssembly/component-model#561 for background.
            return Poll::Ready(Ok(if finish {
                StreamResult::Cancelled
            } else {
                StreamResult::Completed
            }));
        }

        match Pin::new(io).poll_write(cx, remaining) {
            Poll::Ready(Ok(0)) => {
                self.notify_dropped(Err(std::io::ErrorKind::WriteZero.into()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Ready(Ok(n)) => {
                src.mark_read(n);
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Poll::Ready(Err(e)) => {
                self.notify_dropped(Err(e));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

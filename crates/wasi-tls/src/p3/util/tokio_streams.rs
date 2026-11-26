use bytes::BytesMut;
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
    type Buffer = BytesMut;

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
        let remaining = dst.remaining()?;

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
/// Every consume is followed by a flush, since the inner `AsyncWrite` (e.g. a
/// TLS stream) may hold on to written data until it is flushed.
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
        let remaining = src.remaining()?;

        // A zero-length consume is a readiness check rather than a write. There
        // is nothing to write in that case, but the flush below still runs, so
        // anything a previous write left buffered gets sent out.
        //
        // See WebAssembly/component-model#561 for background.
        if !remaining.is_empty() {
            match Pin::new(&mut *io).poll_write(cx, remaining) {
                Poll::Ready(Ok(0)) => {
                    self.notify_dropped(Err(std::io::ErrorKind::WriteZero.into()));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                }
                Poll::Ready(Ok(n)) => src.mark_read(n),
                Poll::Ready(Err(e)) => {
                    self.notify_dropped(Err(e));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                }
                Poll::Pending if finish => return Poll::Ready(Ok(StreamResult::Cancelled)),
                Poll::Pending => return Poll::Pending,
            }
        }

        match Pin::new(io).poll_flush(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(StreamResult::Completed)),
            Poll::Ready(Err(e)) => {
                self.notify_dropped(Err(e));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::BufWriter;
    use wasmtime::component::StreamReader;
    use wasmtime::{Config, Engine, Store};

    /// `BufWriter` holds written bytes back until it is flushed, like a TLS
    /// stream does, so the bytes only reach the inner `Vec` if the consumer
    /// flushed before reporting the write complete.
    #[tokio::test]
    async fn write_is_flushed_before_completing() -> wasmtime::Result<()> {
        let mut config = Config::default();
        config.wasm_component_model_async(true);
        let engine = Engine::new(&config)?;
        let mut store = Store::new(&engine, ());

        let (tx, rx) = oneshot::channel();
        let consumer = AsyncWriteConsumer::new(BufWriter::new(Vec::default()), tx);
        StreamReader::new(&mut store, b"hello".to_vec())?.pipe(&mut store, consumer)?;
        let io = store.run_concurrent(async |_| rx.await).await??;
        let io = io?;

        assert_eq!(io.get_ref(), b"hello");
        Ok(())
    }
}

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
/// Every write is followed by a flush, since the inner `AsyncWrite` (e.g. a
/// TLS stream) may hold on to written data until it is flushed.
///
/// Once the inner `AsyncWrite` returns an error, the consumer will be dropped
/// and the final result is submitted to the `dropped` channel.
pub(crate) struct AsyncWriteConsumer<IO> {
    inner: Option<(IO, oneshot::Sender<std::io::Result<IO>>)>,
    flush_pending: bool,
}
impl<IO> AsyncWriteConsumer<IO> {
    pub(crate) fn new(io: IO, dropped: oneshot::Sender<std::io::Result<IO>>) -> Self {
        Self {
            inner: Some((io, dropped)),
            flush_pending: false,
        }
    }
    fn io(&mut self) -> Option<&mut IO> {
        self.inner.as_mut().map(|(io, _)| io)
    }
    fn notify_dropped(&mut self, result: std::io::Result<()>) {
        if let Some((io, dropped)) = self.inner.take() {
            let _ = dropped.send(result.map(|()| io));
        }
    }
    fn poll_flush(
        &mut self,
        cx: &mut Context<'_>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>>
    where
        IO: AsyncWrite + Unpin,
    {
        let Some(io) = self.io() else {
            return Poll::Ready(Ok(StreamResult::Dropped));
        };
        match Pin::new(io).poll_flush(cx) {
            Poll::Ready(Ok(())) => {
                self.flush_pending = false;
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Poll::Ready(Err(e)) => {
                self.flush_pending = false;
                self.notify_dropped(Err(e));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Pending if finish => {
                self.flush_pending = false;
                Poll::Ready(Ok(StreamResult::Cancelled))
            }
            Poll::Pending => {
                self.flush_pending = true;
                Poll::Pending
            }
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
        if self.flush_pending {
            return self.poll_flush(cx, finish);
        }

        let Some(io) = self.io() else {
            return Poll::Ready(Ok(StreamResult::Dropped));
        };

        let mut src = src.as_direct(store);
        let remaining = src.remaining()?;

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
                self.poll_flush(cx, finish)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use wasmtime::component::StreamReader;
    use wasmtime::{Config, Engine, Store};

    /// An `AsyncWrite` that, like a TLS stream, holds on to written bytes until
    /// it is flushed. The first flush returns `Pending`.
    struct SendOnFlush {
        buffered: Vec<u8>,
        sent: Arc<Mutex<Vec<u8>>>,
        flush_was_pending: bool,
    }

    impl AsyncWrite for SendOnFlush {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            self.buffered.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            if !self.flush_was_pending {
                self.flush_was_pending = true;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let buffered = std::mem::take(&mut self.buffered);
            self.sent.lock().unwrap().extend(buffered);
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            self.poll_flush(cx)
        }
    }

    /// Writes `data` once, signals `written` once that write has completed,
    /// and then keeps the stream open.
    struct WriteOnce {
        data: Option<Vec<u8>>,
        written: Option<oneshot::Sender<()>>,
    }

    impl<D> StreamProducer<D> for WriteOnce {
        type Item = u8;
        type Buffer = BytesMut;

        fn poll_produce<'a>(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _store: StoreContextMut<'a, D>,
            mut dst: Destination<'a, Self::Item, Self::Buffer>,
            finish: bool,
        ) -> Poll<wasmtime::Result<StreamResult>> {
            if let Some(data) = self.data.take() {
                dst.set_buffer(BytesMut::from(&data[..]));
                return Poll::Ready(Ok(StreamResult::Completed));
            }
            if let Some(written) = self.written.take() {
                let _ = written.send(());
            }
            if finish {
                Poll::Ready(Ok(StreamResult::Cancelled))
            } else {
                Poll::Pending
            }
        }
    }

    #[tokio::test]
    async fn write_is_flushed_before_completing() -> wasmtime::Result<()> {
        let mut config = Config::new();
        config.wasm_component_model_async(true);
        let engine = Engine::new(&config)?;
        let mut store = Store::new(&engine, ());

        let sent = Arc::new(Mutex::new(Vec::new()));
        let io = SendOnFlush {
            buffered: Vec::new(),
            sent: sent.clone(),
            flush_was_pending: false,
        };
        let (written_tx, written_rx) = oneshot::channel();
        let producer = WriteOnce {
            data: Some(b"hello".to_vec()),
            written: Some(written_tx),
        };
        let (dropped_tx, _dropped_rx) = oneshot::channel();

        StreamReader::new(&mut store, producer)?
            .pipe(&mut store, AsyncWriteConsumer::new(io, dropped_tx))?;
        store
            .run_concurrent(async move |_| written_rx.await)
            .await??;

        assert_eq!(*sent.lock().unwrap(), b"hello");
        Ok(())
    }
}

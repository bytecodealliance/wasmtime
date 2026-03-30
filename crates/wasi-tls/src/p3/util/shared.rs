use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// An `Arc<Mutex<IO>>` wrapper.
///
/// Implements `AsyncRead` and `AsyncWrite` if the inner `IO` does so too.
pub(crate) struct Shared<IO>(Arc<Mutex<IO>>);
impl<IO> Shared<IO> {
    pub(crate) fn new(io: IO) -> Self {
        Self(Arc::new(Mutex::new(io)))
    }
    pub(crate) fn lock(&self) -> MutexGuard<'_, IO> {
        self.0.lock().expect("other thread panicked")
    }
}
impl<IO> Clone for Shared<IO> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl<IO: AsyncRead + Unpin> AsyncRead for Shared<IO> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.lock()).poll_read(cx, buf)
    }
}
impl<IO: AsyncWrite + Unpin> AsyncWrite for Shared<IO> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut *self.lock()).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.lock()).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.lock()).poll_shutdown(cx)
    }
}

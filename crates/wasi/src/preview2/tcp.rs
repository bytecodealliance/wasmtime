use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

/// We can't just use `tokio::net::tcp::OwnedReadHalf` because we need to keep
/// access to the original TcpStream.
pub(crate) struct SystemTcpReader {
    inner: Arc<tokio::net::TcpStream>,
}

impl SystemTcpReader {
    pub fn new(inner: Arc<tokio::net::TcpStream>) -> Self {
        Self { inner }
    }
}

impl tokio::io::AsyncRead for SystemTcpReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        while self.inner.poll_read_ready(cx).is_ready() {
            match self.inner.try_read_buf(buf) {
                Ok(_) => return Poll::Ready(Ok(())),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Poll::Ready(Err(e)),
            }
        }

        Poll::Pending
    }
}

/// We can't just use `tokio::net::tcp::OwnedWriteHalf` because we need to keep
/// access to the original TcpStream. Also, `OwnedWriteHalf` calls `shutdown` on
/// the underlying socket, which is not what we want.
pub(crate) struct SystemTcpWriter {
    pub(crate) inner: Arc<tokio::net::TcpStream>,
}

impl SystemTcpWriter {
    pub fn new(inner: Arc<tokio::net::TcpStream>) -> Self {
        Self { inner }
    }
}

impl tokio::io::AsyncWrite for SystemTcpWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        while self.inner.poll_write_ready(cx).is_ready() {
            match self.inner.try_write(buf) {
                Ok(n) => return Poll::Ready(Ok(n)),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Poll::Ready(Err(e)),
            }
        }

        Poll::Pending
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        // We're not managing any internal buffer so we have nothing to flush.
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        // This method is never called by the WASI wrappers.
        // And even if it was, we wouldn't want to call `shutdown` because we don't own the socket.
        Poll::Ready(Ok(()))
    }
}

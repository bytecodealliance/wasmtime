//! Bridge a tokio byte stream to hyper's runtime I/O traits.

use std::pin::Pin;
use std::task::{ready, Context, Poll};

use hyper::rt::{Read, ReadBufCursor, Write};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

// Equivalent to `hyper_util::rt::TokioIo` and the similar version in
// `wasmtime-wasi-http`'s `io.rs`. Kept here to avoid the dependency and
// simplified to avoid `unsafe` (one buffer copy per read).

pub struct TokioIo<T>(pub T);

impl<T: AsyncRead + Unpin> Read for TokioIo<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> Poll<std::io::Result<()>> {
        let mut tmp = [0u8; 8192];
        let n = buf.remaining().min(tmp.len());
        let mut rb = ReadBuf::new(&mut tmp[..n]);
        ready!(Pin::new(&mut self.0).poll_read(cx, &mut rb))?;
        buf.put_slice(rb.filled());
        Poll::Ready(Ok(()))
    }
}

impl<T: AsyncWrite + Unpin> Write for TokioIo<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

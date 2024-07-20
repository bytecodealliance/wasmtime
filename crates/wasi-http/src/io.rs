//! I/O utility for bridging between `tokio::io` and `hyper::rt`.

use hyper::rt::{Read, ReadBufCursor, Write};
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A type that wraps any type implementing [`tokio::io::AsyncRead`] and [`tokio::io::AsyncWrite`]
/// and itself implements [`hyper::rt::Read`] and [`hyper::rt::Write`].
#[derive(Debug)]
pub struct TokioIo<T> {
    inner: T,
}

impl<T> TokioIo<T> {
    /// Create a new `TokioIo` wrapping the given inner type.
    pub fn new(inner: T) -> TokioIo<T> {
        TokioIo { inner }
    }
}

impl<T: AsyncRead + Unpin> Read for TokioIo<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> Poll<Result<(), Error>> {
        unsafe {
            let mut dst = ReadBuf::uninit(buf.as_mut());
            let res = Pin::new(&mut self.inner).poll_read(cx, &mut dst);
            let amt = dst.filled().len();
            buf.advance(amt);
            res
        }
    }
}

impl<T: AsyncWrite + Unpin> Write for TokioIo<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

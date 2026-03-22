use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

pub(crate) const CAPACITY: usize = 16 * 1024;

/// A unidirectional in-memory pipe.
///
/// Data written to the `Writer` can be read from the `Reader`.
/// Closing the `Writer` will cause the `Reader` to see EOF, and
/// closing the `Reader` will cause the `Writer` to see a broken pipe error.
///
/// Naively one would reach for `tokio::io::simplex`, but that does not
/// communicate EOF or broken pipe when the other side is dropped.
pub(crate) fn pipe() -> (Reader, Writer) {
    let (r, w) = tokio::io::duplex(CAPACITY);
    (Reader(r), Writer(w))
}
pub(crate) struct Reader(DuplexStream);
pub(crate) struct Writer(DuplexStream);

impl AsyncRead for Reader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl AsyncWrite for Writer {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn basic_functionality() {
        let (mut r, mut w) = pipe();
        w.write_all(b"hello, world").await.unwrap();
        let mut buf = vec![0u8; 12];
        r.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"hello, world");
    }

    #[tokio::test]
    async fn reader_eof_on_writer_drop() {
        let (mut r, w) = pipe();
        drop(w);
        let n = r.read(&mut vec![0u8; 16]).await.unwrap();
        assert_eq!(n, 0, "expected EOF after writer drop");
    }

    #[tokio::test]
    async fn reader_eof_on_writer_shutdown() {
        let (mut r, mut w) = pipe();
        w.shutdown().await.unwrap();
        let n = r.read(&mut vec![0u8; 16]).await.unwrap();
        assert_eq!(n, 0, "expected EOF after writer shutdown");
    }

    #[tokio::test]
    async fn writer_broken_pipe_on_reader_drop() {
        let (r, mut w) = pipe();
        drop(r);
        let err = w.write_all(b"hello").await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
    }
}

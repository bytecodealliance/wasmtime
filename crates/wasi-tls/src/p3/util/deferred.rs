use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A stream whose underlying IO implementation will come later.
///
/// The stream starts out idle. All IO operations during this time will return
/// `Poll::Pending` and store the waker. Once the underlying IO is installed via
/// `resolve()`, the wakers will be fired and all IO operations will be
/// delegated to the inner IO.
pub(crate) enum Deferred<IO> {
    Pending {
        read_waker: Waker,
        write_waker: Waker,
    },
    Ready(IO),
}
impl<IO> Deferred<IO> {
    /// Creates an idle stream.
    pub(crate) fn pending() -> Self {
        let noop = Waker::noop();
        Deferred::Pending {
            read_waker: noop.clone(),
            write_waker: noop.clone(),
        }
    }

    /// Install the inner IO and wake any stored wakers.
    pub(crate) fn resolve(&mut self, io: IO) {
        match std::mem::replace(self, Deferred::Ready(io)) {
            Deferred::Pending {
                read_waker,
                write_waker,
            } => {
                read_waker.wake();
                write_waker.wake();
            }
            Deferred::Ready(_) => unreachable!("Deferred should only be made ready once"),
        }
    }

    fn poll_read_io<T>(
        &mut self,
        cx: &mut Context<'_>,
        f: impl FnOnce(Pin<&mut IO>, &mut Context<'_>) -> Poll<T>,
    ) -> Poll<T>
    where
        IO: Unpin,
    {
        match self {
            Deferred::Pending { read_waker, .. } => {
                *read_waker = cx.waker().clone();
                Poll::Pending
            }
            Deferred::Ready(io) => f(Pin::new(io), cx),
        }
    }

    fn poll_write_io<T>(
        &mut self,
        cx: &mut Context<'_>,
        f: impl FnOnce(Pin<&mut IO>, &mut Context<'_>) -> Poll<T>,
    ) -> Poll<T>
    where
        IO: Unpin,
    {
        match self {
            Deferred::Pending { write_waker, .. } => {
                *write_waker = cx.waker().clone();
                Poll::Pending
            }
            Deferred::Ready(io) => f(Pin::new(io), cx),
        }
    }
}

impl<IO: AsyncRead + Unpin> AsyncRead for Deferred<IO> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.poll_read_io(cx, |io, cx| io.poll_read(cx, buf))
    }
}

impl<IO: AsyncWrite + Unpin> AsyncWrite for Deferred<IO> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.poll_write_io(cx, |io, cx| io.poll_write(cx, buf))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.poll_write_io(cx, |io, cx| io.poll_flush(cx))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.poll_write_io(cx, |io, cx| io.poll_shutdown(cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p3::util::pipe;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::task::Wake;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

    struct BoolWaker(AtomicBool);
    impl BoolWaker {
        fn new() -> Arc<Self> {
            Arc::new(Self(AtomicBool::new(false)))
        }
        fn waker(self: &Arc<Self>) -> Waker {
            Waker::from(self.clone())
        }
        fn is_set(&self) -> bool {
            self.0.load(Ordering::SeqCst)
        }
    }
    impl Wake for BoolWaker {
        fn wake(self: Arc<Self>) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    /// Core lifecycle: pending → Poll::Pending (read waker stored) → resolve
    /// wakes it → IO is then delegated to the inner reader.
    #[tokio::test]
    async fn read_lifecycle() {
        let read_flag = BoolWaker::new();
        let read_waker = read_flag.waker();
        let mut cx = Context::from_waker(&read_waker);

        let (pipe_reader, mut pipe_writer) = pipe::pipe();
        pipe_writer.write_all(b"hello").await.unwrap();
        drop(pipe_writer);
        let mut deferred: Deferred<pipe::Reader> = Deferred::pending();

        let mut buf = [0u8; 8];
        assert!(
            Pin::new(&mut deferred)
                .poll_read(&mut cx, &mut ReadBuf::new(&mut buf))
                .is_pending()
        );
        assert!(
            !read_flag.is_set(),
            "read waker must not fire before resolve"
        );

        deferred.resolve(pipe_reader);
        assert!(
            read_flag.is_set(),
            "resolve must wake the stored read waker"
        );

        let mut out = Vec::new();
        deferred.read_to_end(&mut out).await.unwrap();
        assert_eq!(&out, b"hello");
    }

    /// Write-side lifecycle: pending → Poll::Pending (write waker stored) →
    /// resolve wakes it → IO is then delegated to the inner writer.
    #[tokio::test]
    async fn write_lifecycle() {
        let write_flag = BoolWaker::new();
        let write_waker = write_flag.waker();
        let mut cx = Context::from_waker(&write_waker);

        let (mut pipe_reader, pipe_writer) = pipe::pipe();
        let mut deferred: Deferred<pipe::Writer> = Deferred::pending();

        // Poll write while still pending — write waker must be stored.
        assert!(
            Pin::new(&mut deferred)
                .poll_write(&mut cx, b"world")
                .is_pending()
        );
        assert!(
            !write_flag.is_set(),
            "write waker must not fire before resolve"
        );

        deferred.resolve(pipe_writer);
        assert!(
            write_flag.is_set(),
            "resolve must wake the stored write waker"
        );

        deferred.write_all(b"world").await.unwrap();
        deferred.shutdown().await.unwrap();

        let mut out = Vec::new();
        pipe_reader.read_to_end(&mut out).await.unwrap();
        assert_eq!(&out, b"world");
    }

    /// resolve() wakes both the read and write wakers on the *same* Deferred.
    /// Uses DuplexStream which implements both AsyncRead and AsyncWrite, so a
    /// single Deferred<DuplexStream> can have distinct wakers registered on
    /// each side before the IO is available.
    #[tokio::test]
    async fn resolve_wakes_both() {
        let read_flag = BoolWaker::new();
        let write_flag = BoolWaker::new();

        let (duplex, _other) = tokio::io::duplex(64);
        let mut deferred: Deferred<DuplexStream> = Deferred::pending();

        let read_waker = read_flag.waker();
        let write_waker = write_flag.waker();
        let mut read_cx = Context::from_waker(&read_waker);
        let mut write_cx = Context::from_waker(&write_waker);

        // Register different wakers on each side of the same Deferred.
        let mut buf = [0u8; 4];
        assert!(
            Pin::new(&mut deferred)
                .poll_read(&mut read_cx, &mut ReadBuf::new(&mut buf))
                .is_pending()
        );
        assert!(
            Pin::new(&mut deferred)
                .poll_write(&mut write_cx, b"bye")
                .is_pending()
        );
        assert!(
            !read_flag.is_set(),
            "read waker must not fire before resolve"
        );
        assert!(
            !write_flag.is_set(),
            "write waker must not fire before resolve"
        );

        deferred.resolve(duplex);

        assert!(read_flag.is_set(), "resolve must wake the read waker");
        assert!(write_flag.is_set(), "resolve must wake the write waker");
    }

    /// resolve() before any poll is fine; the noop waker is woken harmlessly.
    #[tokio::test]
    async fn resolve_before_poll() {
        let (pipe_reader, mut pipe_writer) = pipe::pipe();
        pipe_writer.write_all(b"eager").await.unwrap();
        drop(pipe_writer);

        let mut deferred: Deferred<pipe::Reader> = Deferred::pending();
        deferred.resolve(pipe_reader);

        let mut out = Vec::new();
        deferred.read_to_end(&mut out).await.unwrap();
        assert_eq!(&out, b"eager");
    }
}

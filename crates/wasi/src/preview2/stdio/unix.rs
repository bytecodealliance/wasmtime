use crate::preview2::{pipe::AsyncReadStream, HostInputStream, StreamState};
use anyhow::Error;
use bytes::Bytes;
use futures::ready;
use std::future::Future;
use std::io::{self, Read};
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll};
use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, ReadBuf};

// We need a single global instance of the AsyncFd<Stdin> because creating
// this instance registers the process's stdin fd with epoll, which will
// return an error if an fd is registered more than once.
static STDIN: OnceLock<Stdin> = OnceLock::new();

#[derive(Clone)]
pub struct Stdin(Arc<Mutex<AsyncReadStream>>);

pub fn stdin() -> Stdin {
    fn init_stdin() -> AsyncReadStream {
        use crate::preview2::RUNTIME;
        match tokio::runtime::Handle::try_current() {
            Ok(_) => AsyncReadStream::new(InnerStdin::new().unwrap()),
            Err(_) => {
                let _enter = RUNTIME.enter();
                RUNTIME.block_on(async { AsyncReadStream::new(InnerStdin::new().unwrap()) })
            }
        }
    }

    let handle = STDIN
        .get_or_init(|| Stdin(Arc::new(Mutex::new(init_stdin()))))
        .clone();

    {
        let mut guard = handle.0.lock().unwrap();

        // The backing task exited. This can happen in two cases:
        //
        // 1. the task crashed
        // 2. the runtime has exited and been restarted in the same process
        //
        // As we can't tell the difference between these two, we assume the latter and restart the
        // task.
        if guard.join_handle.is_finished() {
            *guard = init_stdin();
        }
    }

    handle
}

#[async_trait::async_trait]
impl crate::preview2::HostInputStream for Stdin {
    fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), Error> {
        HostInputStream::read(&mut *self.0.lock().unwrap(), size)
    }

    async fn ready(&mut self) -> Result<(), Error> {
        // Custom Future impl takes the std mutex in each invocation of poll.
        // Required so we don't have to use a tokio mutex, which we can't take from
        // inside a sync context in Self::read.
        //
        // Taking the lock, creating a fresh ready() future, polling it once, and
        // then releasing the lock is acceptable here because the ready() future
        // is only ever going to await on a single channel recv, plus some management
        // of a state machine (for buffering).
        struct Ready<'a> {
            handle: &'a Stdin,
        }
        impl<'a> Future for Ready<'a> {
            type Output = Result<(), Error>;
            fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                let mut locked = self.handle.0.lock().unwrap();
                let fut = locked.ready();
                tokio::pin!(fut);
                fut.poll(cx)
            }
        }
        Ready { handle: self }.await
    }
}

struct InnerStdin {
    inner: AsyncFd<std::io::Stdin>,
}

impl InnerStdin {
    pub fn new() -> anyhow::Result<Self> {
        use rustix::fs::OFlags;
        use std::os::fd::AsRawFd;

        let stdin = std::io::stdin();

        let borrowed_fd = unsafe { rustix::fd::BorrowedFd::borrow_raw(stdin.as_raw_fd()) };
        let flags = rustix::fs::fcntl_getfl(borrowed_fd)?;
        if !flags.contains(OFlags::NONBLOCK) {
            rustix::fs::fcntl_setfl(borrowed_fd, flags.union(OFlags::NONBLOCK))?;
        }

        Ok(Self {
            inner: AsyncFd::new(stdin)?,
        })
    }
}

impl AsyncRead for InnerStdin {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            let mut guard = ready!(self.inner.poll_read_ready_mut(cx))?;

            let unfilled = buf.initialize_unfilled();
            match guard.try_io(|inner| inner.get_mut().read(unfilled)) {
                Ok(Ok(len)) => {
                    buf.advance(len);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(err)) => {
                    return Poll::Ready(Err(err));
                }
                Err(_would_block) => {
                    continue;
                }
            }
        }
    }
}

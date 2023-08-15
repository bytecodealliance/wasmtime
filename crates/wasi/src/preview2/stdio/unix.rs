use super::worker_thread_stdin;
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
pub enum Stdin {
    // The process's standard input can be successfully registered with `epoll`,
    // so it's tracked by a native async stream.
    Async(Arc<Mutex<AsyncReadStream>>),

    // The process's stdin can't be registered with epoll, for example it's a
    // file on Linux or `/dev/null` on macOS. The fallback implementation of a
    // worker thread is used in these situations.
    Blocking(worker_thread_stdin::Stdin),
}

pub fn stdin() -> Stdin {
    fn init_stdin() -> anyhow::Result<AsyncReadStream> {
        use crate::preview2::RUNTIME;
        match tokio::runtime::Handle::try_current() {
            Ok(_) => Ok(AsyncReadStream::new(InnerStdin::new()?)),
            Err(_) => {
                let _enter = RUNTIME.enter();
                RUNTIME.block_on(async { Ok(AsyncReadStream::new(InnerStdin::new()?)) })
            }
        }
    }

    let handle = STDIN
        .get_or_init(|| match init_stdin() {
            Ok(stream) => Stdin::Async(Arc::new(Mutex::new(stream))),
            Err(_) => Stdin::Blocking(worker_thread_stdin::stdin()),
        })
        .clone();

    if let Stdin::Async(stream) = &handle {
        let mut guard = stream.lock().unwrap();

        // The backing task exited. This can happen in two cases:
        //
        // 1. the task crashed
        // 2. the runtime has exited and been restarted in the same process
        //
        // As we can't tell the difference between these two, we assume the latter and restart the
        // task.
        if guard.join_handle.is_finished() {
            *guard = init_stdin().unwrap();
        }
    }

    handle
}

impl is_terminal::IsTerminal for Stdin {
    fn is_terminal(&self) -> bool {
        std::io::stdin().is_terminal()
    }
}

#[async_trait::async_trait]
impl crate::preview2::HostInputStream for Stdin {
    fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), Error> {
        match self {
            Stdin::Async(s) => HostInputStream::read(&mut *s.lock().unwrap(), size),
            Stdin::Blocking(s) => s.read(size),
        }
    }

    async fn ready(&mut self) -> Result<(), Error> {
        match self {
            Stdin::Async(handle) => {
                // Custom Future impl takes the std mutex in each invocation of poll.
                // Required so we don't have to use a tokio mutex, which we can't take from
                // inside a sync context in Self::read.
                //
                // Taking the lock, creating a fresh ready() future, polling it once, and
                // then releasing the lock is acceptable here because the ready() future
                // is only ever going to await on a single channel recv, plus some management
                // of a state machine (for buffering).
                struct Ready<'a> {
                    handle: &'a Arc<Mutex<AsyncReadStream>>,
                }
                impl<'a> Future for Ready<'a> {
                    type Output = Result<(), Error>;
                    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                        let mut locked = self.handle.lock().unwrap();
                        let fut = locked.ready();
                        tokio::pin!(fut);
                        fut.poll(cx)
                    }
                }
                Ready { handle }.await
            }
            Stdin::Blocking(s) => s.ready().await,
        }
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

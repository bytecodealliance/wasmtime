use crate::preview2::{pipe::AsyncReadStream, HostInputStream, StreamState};
use anyhow::Error;
use bytes::Bytes;
use futures::ready;
use std::future::Future;
use std::io::{self, Read};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, ReadBuf};

// wasmtime cant use std::sync::OnceLock yet because of a llvm regression in
// 1.70. when 1.71 is released, we can switch to using std here.
use once_cell::sync::OnceCell as OnceLock;

use std::sync::Mutex;

// We need a single global instance of the AsyncFd<Stdin> because creating
// this instance registers the process's stdin fd with epoll, which will
// return an error if an fd is registered more than once.
struct GlobalStdin(Mutex<AsyncReadStream>);
static STDIN: OnceLock<GlobalStdin> = OnceLock::new();

impl GlobalStdin {
    fn new() -> anyhow::Result<Self> {
        Ok(Self(Mutex::new(AsyncReadStream::new(InnerStdin::new()?))))
    }
    fn read(&self, size: usize) -> Result<(Bytes, StreamState), Error> {
        HostInputStream::read(&mut *self.0.lock().unwrap(), size)
    }
    fn ready<'a>(&'a self) -> impl Future<Output = Result<(), Error>> + 'a {
        // Custom Future impl takes the std mutex in each invocation of poll.
        // Required so we don't have to use a tokio mutex, which we can't take from
        // inside a sync context in Self::read.
        //
        // Taking the lock, creating a fresh ready() future, polling it once, and
        // then releasing the lock is acceptable here because the ready() future
        // is only ever going to await on a single channel recv, plus some management
        // of a state machine (for buffering).
        struct Ready<'a>(&'a GlobalStdin);
        impl<'a> Future for Ready<'a> {
            type Output = Result<(), Error>;
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                let mut locked = self.as_mut().0 .0.lock().unwrap();
                let fut = locked.ready();
                tokio::pin!(fut);
                fut.poll(cx)
            }
        }
        Ready(self)
    }
}

pub struct Stdin;
impl Stdin {
    fn get_global() -> &'static GlobalStdin {
        // Creation must be running in a tokio context to succeed.
        match tokio::runtime::Handle::try_current() {
            Ok(_) => STDIN.get_or_init(|| {
                GlobalStdin::new().expect("creating AsyncFd for stdin in existing tokio context")
            }),
            Err(_) => STDIN.get_or_init(|| {
                crate::preview2::in_tokio(async {
                    GlobalStdin::new()
                        .expect("creating AsyncFd for stdin in internal tokio context")
                })
            }),
        }
    }
}

pub fn stdin() -> Stdin {
    Stdin
}

#[async_trait::async_trait]
impl crate::preview2::HostInputStream for Stdin {
    fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), Error> {
        Self::get_global().read(size)
    }

    async fn ready(&mut self) -> Result<(), Error> {
        Self::get_global().ready().await
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
            rustix::fs::fcntl_setfl(borrowed_fd, flags.difference(OFlags::NONBLOCK))?;
        }

        Ok(Self {
            inner: AsyncFd::new(std::io::stdin())?,
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
                Ok(Err(err)) => return Poll::Ready(Err(err)),
                Err(_would_block) => continue,
            }
        }
    }
}

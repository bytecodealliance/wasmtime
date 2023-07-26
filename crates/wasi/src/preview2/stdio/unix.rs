use crate::preview2::{pipe::AsyncReadStream, HostInputStream, StreamState};
use anyhow::{Context as _, Error};
use bytes::Bytes;
use futures::ready;
use std::future::Future;
use std::io::{self, Read};
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::task::{Context, Poll};
use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, ReadBuf};

// We need a single global instance of the AsyncFd<Stdin> because creating
// this instance registers the process's stdin fd with epoll, which will
// return an error if an fd is registered more than once.
static STDIN: OnceLock<Mutex<GlobalStdin>> = OnceLock::new();

struct GlobalStdin(Weak<Mutex<AsyncReadStream>>);

pub struct Stdin(Arc<Mutex<AsyncReadStream>>);
pub fn stdin() -> Stdin {
    GlobalStdin::get().unwrap()
}

impl GlobalStdin {
    fn upgrade(&self) -> Option<Stdin> {
        Weak::upgrade(&self.0).map(Stdin)
    }

    fn new() -> anyhow::Result<(Self, Stdin)> {
        use crate::preview2::RUNTIME;
        let inner = match tokio::runtime::Handle::try_current() {
            Ok(_) => AsyncReadStream::new(InnerStdin::new()?),
            Err(_) => {
                let _enter = RUNTIME.enter();
                RUNTIME.block_on(async {
                    Ok::<_, anyhow::Error>(AsyncReadStream::new(InnerStdin::new()?))
                })?
            }
        };
        let strong = Arc::new(Mutex::new(inner));
        let global = GlobalStdin(Arc::downgrade(&strong));
        Ok((global, Stdin(strong)))
    }

    fn get() -> anyhow::Result<Stdin> {
        match STDIN.get() {
            None => {
                let (global, strong) =
                    Self::new().context("creating global stdin resource for first time")?;
                match STDIN.set(Mutex::new(global)) {
                    Ok(_) => Ok(strong),
                    Err(_) => panic!("fixme: lost race?"),
                }
            }
            Some(g) => {
                let mut g = g.lock().unwrap();
                match g.upgrade() {
                    Some(strong) => Ok(strong),
                    None => {
                        // BUG: the Arc can go to zero but the AsyncFd hasnt finished dropping yet,
                        // so this will fail sometimes because epoll hasnt yet had the fd
                        // unregistered
                        let (global, strong) =
                            Self::new().context("re-creating global stdin resource")?;
                        *g = global;
                        Ok(strong)
                    }
                }
            }
        }
    }
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

struct MyStdin(std::os::fd::RawFd);
impl MyStdin {
    fn new() -> Self {
        MyStdin(libc::STDIN_FILENO)
    }
}
impl std::os::fd::AsRawFd for MyStdin {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.0
    }
}
impl rustix::fd::AsFd for MyStdin {
    fn as_fd(&self) -> rustix::fd::BorrowedFd<'_> {
        unsafe { rustix::fd::BorrowedFd::borrow_raw(self.0) }
    }
}

impl Read for MyStdin {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(rustix::io::read(self, buf)?)
    }
}

struct InnerStdin {
    inner: AsyncFd<MyStdin>,
}

impl InnerStdin {
    pub fn new() -> anyhow::Result<Self> {
        use rustix::fs::OFlags;
        use std::os::fd::AsRawFd;

        let stdin = MyStdin::new();

        let borrowed_fd = unsafe { rustix::fd::BorrowedFd::borrow_raw(stdin.as_raw_fd()) };
        let flags = rustix::fs::fcntl_getfl(borrowed_fd)?;
        if !flags.contains(OFlags::NONBLOCK) {
            rustix::fs::fcntl_setfl(borrowed_fd, flags.union(OFlags::NONBLOCK))?;
        }

        Ok(Self {
            inner: AsyncFd::new(MyStdin::new())?,
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

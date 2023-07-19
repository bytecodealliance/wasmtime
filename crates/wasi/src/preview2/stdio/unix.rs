use crate::preview2::{pipe::AsyncReadStream, StreamState};
use anyhow::Error;
use bytes::Bytes;
use tokio::io::unix::AsyncFd;

// wasmtime cant use std::sync::OnceLock yet because of a llvm regression in
// 1.70. when 1.71 is released, we can switch to using std here.
use once_cell::sync::OnceCell as OnceLock;

// FIXME: we might be able to eliminate this, and block_in_place as well,
// if we write ready() with an impl Future that takes and releases a std::sync::mutex
// as part of every poll() invocation. It isnt critical that ready hold the
// lock for the duration of the polling - using stdin from multiple contexts
// is already bogus in terms of application functionality, we are just trying to
// make the implementation typecheck.
// We use a tokio Mutex because, in ready(), the mutex needs to be held
// across an await.
use tokio::sync::Mutex;

// We need a single global instance of the AsyncFd<Stdin> because creating
// this instance registers the process's stdin fd with epoll, which will
// return an error if an fd is registered more than once.
type GlobalStdin = Mutex<AsyncReadStream>;
static STDIN: OnceLock<GlobalStdin> = OnceLock::new();

fn create() -> anyhow::Result<GlobalStdin> {
    Ok(Mutex::new(AsyncReadStream::new(InnerStdin::new()?)))
}

pub struct Stdin;
impl Stdin {
    fn get_global() -> &'static GlobalStdin {
        // Creation must be running in a tokio context to succeed.
        match tokio::runtime::Handle::try_current() {
            Ok(_) => STDIN.get_or_init(|| {
                create().expect("creating AsyncFd for stdin in existing tokio context")
            }),
            Err(_) => STDIN.get_or_init(|| {
                crate::preview2::block_on(async {
                    create().expect("creating AsyncFd for stdin in internal tokio context")
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
        let r = move || Self::get_global().blocking_lock().read(size);
        // If we are currently in a tokio context, blocking_lock will panic unless inside a
        // block_in_place:
        match tokio::runtime::Handle::try_current() {
            Ok(_) => tokio::task::block_in_place(r),
            Err(_) => r(),
        }
    }

    async fn ready(&mut self) -> Result<(), Error> {
        Self::get_global().lock().await.ready().await
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

use futures::ready;
use std::io::{self, Read};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};

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

use anyhow::Error;

use crate::preview2::{AsyncWriteStream, HostInputStream, HostOutputStream, StreamState};

pub use self::stdin::*;

// TODO: different cfg for windows here
#[cfg(unix)]
mod stdin {
    use crate::preview2::AsyncFdStream;

    pub type Stdin = AsyncFdStream<std::io::Stdin>;

    // FIXME this will still die if more than one is alive per process
    pub fn stdin() -> Stdin {
        // Must be running in a tokio context to succeed.
        fn create() -> anyhow::Result<Stdin> {
            AsyncFdStream::new(std::io::stdin())
        }

        match tokio::runtime::Handle::try_current() {
            Ok(_) => create().expect("already running in a tokio context"),
            Err(_) => crate::preview2::poll::sync::block_on(async {
                create().expect("created a tokio context to run in")
            }),
        }
    }
}

pub type Stdout = AsyncWriteStream<tokio::io::Stdout>;

pub fn stdout() -> Stdout {
    AsyncWriteStream::new(tokio::io::stdout())
}
pub type Stderr = AsyncWriteStream<tokio::io::Stderr>;

pub fn stderr() -> Stderr {
    AsyncWriteStream::new(tokio::io::stderr())
}

pub struct EmptyStream;

#[async_trait::async_trait]
impl HostInputStream for EmptyStream {
    fn read(&mut self, _buf: &mut [u8]) -> Result<(u64, StreamState), Error> {
        Ok((0, StreamState::Open))
    }

    async fn ready(&mut self) -> Result<(), Error> {
        struct Never;

        impl std::future::Future for Never {
            type Output = anyhow::Result<()>;
            fn poll(
                self: std::pin::Pin<&mut Self>,
                _ctx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Self::Output> {
                std::task::Poll::Pending
            }
        }

        // This stream is never ready for reading.
        Never.await
    }
}

#[async_trait::async_trait]
impl HostOutputStream for EmptyStream {
    fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
        Ok(buf.len() as u64)
    }

    async fn ready(&mut self) -> Result<(), Error> {
        // This stream is always ready for writing.
        Ok(())
    }
}

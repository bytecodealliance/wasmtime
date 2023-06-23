use anyhow::Error;

use crate::preview2::pipe::{AsyncReadStream, AsyncWriteStream};
use crate::preview2::{HostInputStream, HostOutputStream, StreamState};

pub use self::stdin::*;

// TODO: different cfg for windows here
#[cfg(unix)]
mod stdin {
    use crate::preview2::pipe::{AsyncReadStream, AsyncWriteStream, AsyncFdStream};

    pub type Stdin = AsyncFdStream<std::io::Stdin>;

    pub fn stdin() -> Stdin {
        AsyncFdStream::new(std::io::stdin())
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

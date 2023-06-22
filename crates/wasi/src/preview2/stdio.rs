use anyhow::Error;

use crate::preview2::{HostInputStream, HostOutputStream, HostPollable, StreamState};
use crate::preview2::pipe::{WrappedRead, WrappedWrite};

// TODO: different cfg for windows here
pub type Stdin = WrappedRead<tokio::io::Stdin>;

pub fn stdin() -> Stdin {
    WrappedRead::new(tokio::io::stdin())
}

pub type Stdout = WrappedWrite<tokio::io::Stdout>;

pub fn stdout() -> Stdout {
    WrappedWrite::new(tokio::io::stdout())
}
pub type Stderr = WrappedWrite<tokio::io::Stderr>;

pub fn stderr() -> Stderr {
    WrappedWrite::new(tokio::io::stderr())
}

pub struct EmptyStream;

#[async_trait::async_trait]
impl HostInputStream for EmptyStream {
    async fn read(&mut self, _buf: &mut [u8]) -> Result<(u64, StreamState), Error> {
        Ok((0, StreamState::Open))
    }

    fn pollable(&self) -> HostPollable {
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
        HostPollable::new(|| Box::pin(Never))
    }
}

#[async_trait::async_trait]
impl HostOutputStream for EmptyStream {
    async fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
        Ok(buf.len() as u64)
    }

    fn pollable(&self) -> HostPollable {
        // This stream is always ready for writing.
        HostPollable::new(|| Box::pin(async { Ok(()) }))
    }
}

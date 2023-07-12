use anyhow::Error;
use bytes::Bytes;

use crate::preview2::{AsyncWriteStream, HostInputStream, HostOutputStream, StreamState};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::{stdin, Stdin};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{stdin, Stdin};

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
    fn read(&mut self) -> Result<(Bytes, StreamState), Error> {
        // Ok((0, StreamState::Open))
        todo!()
    }

    async fn ready(&mut self) -> Result<(), Error> {
        futures_util::future::pending().await
    }
}

#[async_trait::async_trait]
impl HostOutputStream for EmptyStream {
    fn write(&mut self, buf: Bytes) -> Result<u64, Error> {
        // Ok(buf.len() as u64)
        todo!()
    }

    async fn ready(&mut self) -> Result<(), Error> {
        // This stream is always ready for writing.
        Ok(())
    }
}

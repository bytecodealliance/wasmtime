use anyhow::Error;

use crate::preview2::{AsyncWriteStream, HostInputStream, HostOutputStream, StreamState};

pub use self::stdin::{stdin, Stdin};

// TODO: different cfg for windows here
#[cfg(unix)]
mod stdin {
    use crate::preview2::{AsyncFdStream, HostInputStream, StreamState};
    use anyhow::Error;
    use std::sync::OnceLock;
    use tokio::sync::Mutex;

    type GlobalStdin = Mutex<AsyncFdStream<std::io::Stdin>>;
    static STDIN: OnceLock<GlobalStdin> = OnceLock::new();

    // Must be running in a tokio context to succeed.
    fn create() -> anyhow::Result<GlobalStdin> {
        Ok(Mutex::new(AsyncFdStream::new(std::io::stdin())?))
    }

    pub struct Stdin;
    impl Stdin {
        fn get_global() -> &'static GlobalStdin {
            match tokio::runtime::Handle::try_current() {
                Ok(_) => STDIN.get_or_init(|| {
                    create().expect("creating AsyncFd for stdin in existing tokio context")
                }),
                Err(_) => STDIN.get_or_init(|| {
                    crate::preview2::poll::sync::block_on(async {
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
    impl HostInputStream for Stdin {
        fn read(&mut self, buf: &mut [u8]) -> Result<(u64, StreamState), Error> {
            let mut r = move || Self::get_global().blocking_lock().read(buf);
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
        futures_util::future::pending().await
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

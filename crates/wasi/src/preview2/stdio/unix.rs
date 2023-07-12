use crate::preview2::{AsyncFdStream, HostInputStream, StreamState};
use anyhow::Error;
use bytes::Bytes;

// wasmtime cant use std::sync::OnceLock yet because of a llvm regression in
// 1.70. when 1.71 is released, we can switch to using std here.
use once_cell::sync::OnceCell as OnceLock;

// We use a tokio Mutex because, in ready(), the mutex needs to be held
// across an await.
use tokio::sync::Mutex;

// We need a single global instance of the AsyncFd<Stdin> because creating
// this instance registers the process's stdin fd with epoll, which will
// return an error if an fd is registered more than once.
type GlobalStdin = Mutex<AsyncFdStream<std::io::Stdin>>;
static STDIN: OnceLock<GlobalStdin> = OnceLock::new();

fn create() -> anyhow::Result<GlobalStdin> {
    Ok(Mutex::new(AsyncFdStream::new(std::io::stdin())?))
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
impl HostInputStream for Stdin {
    fn read(&mut self) -> Result<(Bytes, StreamState), Error> {
        // let mut r = move || Self::get_global().blocking_lock().read(buf);
        // // If we are currently in a tokio context, blocking_lock will panic unless inside a
        // // block_in_place:
        // match tokio::runtime::Handle::try_current() {
        //     Ok(_) => tokio::task::block_in_place(r),
        //     Err(_) => r(),
        // }
        todo!()
    }

    async fn ready(&mut self) -> Result<(), Error> {
        Self::get_global().lock().await.ready().await
    }
}

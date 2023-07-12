use crate::preview2::{HostInputStream, StreamState};
use anyhow::{Context, Error};
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
struct GlobalStdin {
    tx: tokio::sync::mpsc::Sender<tokio::sync::oneshot::Sender<anyhow::Result<()>>>,
    // FIXME use a Watch to check for readiness instead of sending a oneshot sender
}
static STDIN: OnceLock<Mutex<GlobalStdin>> = OnceLock::new();

fn create() -> Mutex<GlobalStdin> {
    let (tx, mut rx) =
        tokio::sync::mpsc::channel::<tokio::sync::oneshot::Sender<anyhow::Result<()>>>(1);
    std::thread::spawn(move || {
        use std::io::BufRead;
        // A client is interested in stdin's readiness
        while let Some(msg) = rx.blocking_recv() {
            // Fill buf - can we skip this if its
            // already filled?
            // also, this could block forever and the
            // client could give up. in that case,
            // another client may want to start waiting
            let r = std::io::stdin()
                .lock()
                .fill_buf()
                .map(|_| ())
                .map_err(anyhow::Error::from);
            // tell the client stdin is ready for reading
            let _ = msg.send(r);
        }
    });

    Mutex::new(GlobalStdin { tx })
}

pub struct Stdin;
impl Stdin {
    fn get_global() -> &'static Mutex<GlobalStdin> {
        // Creation must be running in a tokio context to succeed.
        match tokio::runtime::Handle::try_current() {
            Ok(_) => STDIN.get_or_init(|| create()),
            Err(_) => STDIN.get_or_init(|| crate::preview2::block_on(async { create() })),
        }
    }
}

pub fn stdin() -> Stdin {
    Stdin
}

#[async_trait::async_trait]
impl HostInputStream for Stdin {
    fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), Error> {
        // use std::io::Read;
        // let mut r = move || {
        //     let nbytes = std::io::stdin().read(buf)?;
        //     // FIXME handle eof
        //     Ok((nbytes as u64, StreamState::Open))
        // };
        // // If we are currently in a tokio context, block:
        // match tokio::runtime::Handle::try_current() {
        //     Ok(_) => tokio::task::block_in_place(r),
        //     Err(_) => r(),
        // }
        todo!()
    }

    async fn ready(&mut self) -> Result<(), Error> {
        let (result_tx, rx) = tokio::sync::oneshot::channel::<anyhow::Result<()>>();
        Self::get_global()
            .lock()
            .await
            .tx
            .send(result_tx)
            .await // Could hang here if we another wait on this was canceled??
            .context("sending message to worker thread")?;
        rx.await.expect("channel is always alive")
    }
}

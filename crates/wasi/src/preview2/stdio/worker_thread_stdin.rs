use crate::preview2::{HostInputStream, StreamState};
use anyhow::{Context, Error};
use bytes::{Bytes, BytesMut};
use std::io::Read;
use std::sync::Arc;
use tokio::sync::watch;

// wasmtime cant use std::sync::OnceLock yet because of a llvm regression in
// 1.70. when 1.71 is released, we can switch to using std here.
use once_cell::sync::OnceCell as OnceLock;

use std::sync::Mutex;

// We need a single global instance of the AsyncFd<Stdin> because creating
// this instance registers the process's stdin fd with epoll, which will
// return an error if an fd is registered more than once.
struct GlobalStdin {
    rx: watch::Receiver<()>,
    state: Arc<Mutex<StdinState>>,
}
struct StdinState {
    buffer: BytesMut,
    error: Option<std::io::Error>,
    closed: bool,
}

static STDIN: OnceLock<GlobalStdin> = OnceLock::new();

fn create() -> GlobalStdin {
    let (tx, rx) = watch::channel(());

    let state = Arc::new(Mutex::new(StdinState {
        buffer: BytesMut::new(),
        error: None,
        closed: false,
    }));

    let ret = GlobalStdin {
        state: state.clone(),
        rx,
    };

    std::thread::spawn(move || loop {
        let mut bytes = BytesMut::zeroed(1024);
        match std::io::stdin().lock().read(&mut bytes) {
            Ok(nbytes) => {
                bytes.truncate(nbytes);
                {
                    let mut locked = state.lock().unwrap();
                    locked.buffer.extend_from_slice(&bytes);
                }
                tx.send(()).expect("at least one rx exists");
            }
            Err(e) => {
                {
                    let mut locked = state.lock().unwrap();
                    if locked.error.is_none() {
                        locked.error = Some(e)
                    }
                    locked.closed = true;
                }
                tx.send(()).expect("at least one rx exists");
            }
        }
    });
    ret
}

pub struct Stdin;
impl Stdin {
    fn get_global() -> &'static GlobalStdin {
        STDIN.get_or_init(|| create())
    }
}

pub fn stdin() -> Stdin {
    Stdin
}

#[async_trait::async_trait]
impl HostInputStream for Stdin {
    fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), Error> {
        let mut locked = Stdin::get_global().state.lock().unwrap();
        if let Some(e) = locked.error.take() {
            return Err(e.into());
        }
        let size = locked.buffer.len().min(size);
        let bytes = locked.buffer.split_to(size);
        let state = if locked.buffer.is_empty() && locked.closed {
            StreamState::Closed
        } else {
            StreamState::Open
        };
        Ok((bytes.freeze(), state))
    }

    async fn ready(&mut self) -> Result<(), Error> {
        let g = Stdin::get_global();
        // Make sure we dont hold this lock across the await:
        {
            let locked = g.state.lock().unwrap();
            if !locked.buffer.is_empty() || locked.error.is_some() || locked.closed {
                return Ok(());
            }
        };

        let mut rx = g.rx.clone();
        rx.changed().await.context("stdin sender died")
    }
}

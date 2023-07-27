use crate::preview2::{HostInputStream, StreamState};
use anyhow::Error;
use bytes::{Bytes, BytesMut};
use std::io::Read;
use std::sync::Arc;
use tokio::sync::watch;

// wasmtime cant use std::sync::OnceLock yet because of a llvm regression in
// 1.70. when 1.71 is released, we can switch to using std here.
use once_cell::sync::OnceCell as OnceLock;

use std::sync::Mutex;

struct GlobalStdin {
    // Worker thread uses this to notify of new events. Ready checks use this
    // to create a new Receiver via .subscribe(). The newly created receiver
    // will only wait for events created after the call to subscribe().
    tx: Arc<watch::Sender<()>>,
    // Worker thread and receivers share this state to get bytes read off
    // stdin, or the error/closed state.
    state: Arc<Mutex<StdinState>>,
}

#[derive(Debug)]
struct StdinState {
    // Bytes read off stdin.
    buffer: BytesMut,
    // Error read off stdin, if any.
    error: Option<std::io::Error>,
    // If an error has occured in the past, we consider the stream closed.
    closed: bool,
}

static STDIN: OnceLock<GlobalStdin> = OnceLock::new();

fn create() -> GlobalStdin {
    let (tx, _rx) = watch::channel(());
    let tx = Arc::new(tx);

    let state = Arc::new(Mutex::new(StdinState {
        buffer: BytesMut::new(),
        error: None,
        closed: false,
    }));

    let ret = GlobalStdin {
        state: state.clone(),
        tx: tx.clone(),
    };

    std::thread::spawn(move || loop {
        let mut bytes = BytesMut::zeroed(1024);
        match std::io::stdin().lock().read(&mut bytes) {
            // Reading `0` indicates that stdin has reached EOF, so we break
            // the loop to allow the thread to exit.
            Ok(0) => break,

            Ok(nbytes) => {
                // Append to the buffer:
                bytes.truncate(nbytes);
                let mut locked = state.lock().unwrap();
                locked.buffer.extend_from_slice(&bytes);
            }
            Err(e) => {
                // Set the error, and mark the stream as closed:
                let mut locked = state.lock().unwrap();
                if locked.error.is_none() {
                    locked.error = Some(e)
                }
                locked.closed = true;
            }
        }
        // Receivers may or may not exist - fine if they dont, new
        // ones will be created with subscribe()
        let _ = tx.send(());
    });
    ret
}

/// Only public interface is the [`HostInputStream`] impl.
pub struct Stdin;
impl Stdin {
    // Private! Only required internally.
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
        let g = Stdin::get_global();
        let mut locked = g.state.lock().unwrap();

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

        // Block makes sure we dont hold the mutex across the await:
        let mut rx = {
            let locked = g.state.lock().unwrap();
            // read() will only return (empty, open) when the buffer is empty,
            // AND there is no error AND the stream is still open:
            if !locked.buffer.is_empty() || locked.error.is_some() || locked.closed {
                return Ok(());
            }
            // Sender will take the mutex before updating the state of
            // subscribe, so this ensures we will only await for any stdin
            // events that are recorded after we drop the mutex:
            g.tx.subscribe()
        };

        rx.changed().await.expect("impossible for sender to drop");

        Ok(())
    }
}

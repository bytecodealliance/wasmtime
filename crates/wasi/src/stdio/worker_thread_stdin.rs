//! Handling for standard in using a worker task.
//!
//! Standard input is a global singleton resource for the entire program which
//! needs special care. Currently this implementation adheres to a few
//! constraints which make this nontrivial to implement.
//!
//! * Any number of guest wasm programs can read stdin. While this doesn't make
//!   a ton of sense semantically they shouldn't block forever. Instead it's a
//!   race to see who actually reads which parts of stdin.
//!
//! * Data from stdin isn't actually read unless requested. This is done to try
//!   to be a good neighbor to others running in the process. Under the
//!   assumption that most programs have one "thing" which reads stdin the
//!   actual consumption of bytes is delayed until the wasm guest is dynamically
//!   chosen to be that "thing". Before that data from stdin is not consumed to
//!   avoid taking it from other components in the process.
//!
//! * Tokio's documentation indicates that "interactive stdin" is best done with
//!   a helper thread to avoid blocking shutdown of the event loop. That's
//!   respected here where all stdin reading happens on a blocking helper thread
//!   that, at this time, is never shut down.
//!
//! This module is one that's likely to change over time though as new systems
//! are encountered along with preexisting bugs.

use crate::poll::Subscribe;
use crate::stdio::StdinStream;
use crate::{HostInputStream, StreamError};
use bytes::{Bytes, BytesMut};
use std::io::{IsTerminal, Read};
use std::mem;
use std::sync::{Condvar, Mutex, OnceLock};
use tokio::sync::Notify;

#[derive(Default)]
struct GlobalStdin {
    state: Mutex<StdinState>,
    read_requested: Condvar,
    read_completed: Notify,
}

#[derive(Default, Debug)]
enum StdinState {
    #[default]
    ReadNotRequested,
    ReadRequested,
    Data(BytesMut),
    Error(std::io::Error),
    Closed,
}

impl GlobalStdin {
    fn get() -> &'static GlobalStdin {
        static STDIN: OnceLock<GlobalStdin> = OnceLock::new();
        STDIN.get_or_init(|| create())
    }
}

fn create() -> GlobalStdin {
    std::thread::spawn(|| {
        let state = GlobalStdin::get();
        loop {
            // Wait for a read to be requested, but don't hold the lock across
            // the blocking read.
            let mut lock = state.state.lock().unwrap();
            lock = state
                .read_requested
                .wait_while(lock, |state| !matches!(state, StdinState::ReadRequested))
                .unwrap();
            drop(lock);

            let mut bytes = BytesMut::zeroed(1024);
            let (new_state, done) = match std::io::stdin().read(&mut bytes) {
                Ok(0) => (StdinState::Closed, true),
                Ok(nbytes) => {
                    bytes.truncate(nbytes);
                    (StdinState::Data(bytes), false)
                }
                Err(e) => (StdinState::Error(e), true),
            };

            // After the blocking read completes the state should not have been
            // tampered with.
            debug_assert!(matches!(
                *state.state.lock().unwrap(),
                StdinState::ReadRequested
            ));
            *state.state.lock().unwrap() = new_state;
            state.read_completed.notify_waiters();
            if done {
                break;
            }
        }
    });

    GlobalStdin::default()
}

/// Only public interface is the [`HostInputStream`] impl.
#[derive(Clone)]
pub struct Stdin;

/// Returns a stream that represents the host's standard input.
///
/// Suitable for passing to
/// [`WasiCtxBuilder::stdin`](crate::WasiCtxBuilder::stdin).
pub fn stdin() -> Stdin {
    Stdin
}

impl StdinStream for Stdin {
    fn stream(&self) -> Box<dyn HostInputStream> {
        Box::new(Stdin)
    }

    fn isatty(&self) -> bool {
        std::io::stdin().is_terminal()
    }
}

#[async_trait::async_trait]
impl HostInputStream for Stdin {
    fn read(&mut self, size: usize) -> Result<Bytes, StreamError> {
        let g = GlobalStdin::get();
        let mut locked = g.state.lock().unwrap();
        match mem::replace(&mut *locked, StdinState::ReadRequested) {
            StdinState::ReadNotRequested => {
                g.read_requested.notify_one();
                Ok(Bytes::new())
            }
            StdinState::ReadRequested => Ok(Bytes::new()),
            StdinState::Data(mut data) => {
                let size = data.len().min(size);
                let bytes = data.split_to(size);
                *locked = if data.is_empty() {
                    StdinState::ReadNotRequested
                } else {
                    StdinState::Data(data)
                };
                Ok(bytes.freeze())
            }
            StdinState::Error(e) => {
                *locked = StdinState::Closed;
                Err(StreamError::LastOperationFailed(e.into()))
            }
            StdinState::Closed => {
                *locked = StdinState::Closed;
                Err(StreamError::Closed)
            }
        }
    }
}

#[async_trait::async_trait]
impl Subscribe for Stdin {
    async fn ready(&mut self) {
        let g = GlobalStdin::get();

        // Scope the synchronous `state.lock()` to this block which does not
        // `.await` inside of it.
        let notified = {
            let mut locked = g.state.lock().unwrap();
            match *locked {
                // If a read isn't requested yet
                StdinState::ReadNotRequested => {
                    g.read_requested.notify_one();
                    *locked = StdinState::ReadRequested;
                    g.read_completed.notified()
                }
                StdinState::ReadRequested => g.read_completed.notified(),
                StdinState::Data(_) | StdinState::Closed | StdinState::Error(_) => return,
            }
        };

        notified.await;
    }
}

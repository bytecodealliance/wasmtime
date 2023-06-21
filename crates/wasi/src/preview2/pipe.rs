//! Virtual pipes.
//!
//! These types provide easy implementations of `WasiFile` that mimic much of the behavior of Unix
//! pipes. These are particularly helpful for redirecting WASI stdio handles to destinations other
//! than OS files.
//!
//! Some convenience constructors are included for common backing types like `Vec<u8>` and `String`,
//! but the virtual pipes can be instantiated with any `Read` or `Write` type.
//!
use crate::preview2::{HostInputStream, HostOutputStream, HostPollable};
use std::sync::{Arc, Mutex};

pub fn pipe(bound: usize) -> (InputPipe, OutputPipe) {
    let (writer, reader) = tokio::sync::mpsc::channel(bound);

    let input = InnerInputPipe {
        state: StreamState::Open,
        buffer: Vec::new(),
        channel: reader,
    };

    let output = InnerOutputPipe {
        buffer: Vec::new(),
        channel: writer,
    };

    (
        InputPipe(Arc::new(Mutex::new(input))),
        OutputPipe(Arc::new(Mutex::new(output))),
    )
}

struct InnerInputPipe {
    state: StreamState,
    buffer: Vec<u8>,
    channel: tokio::sync::mpsc::Receiver<Vec<u8>>,
}

pub struct InputPipe(Arc<Mutex<InnerInputPipe>>);

impl HostInputStream for InputPipe {
    fn read(&mut self, dest: &mut [u8]) -> Result<(u64, StreamState), Error> {
        let mut i = self.0.lock().unwrap();
        let l = i.buffer.len().min(dest.len());
        let dest = &mut dest[..l];
        dest.copy_from_slice(&i.buffer[..l]);
        i.buffer = i.buffer.split_off(l);
        Ok((l as u64, i.state))
    }

    fn pollable(&self) -> HostPollable {
        let i = Arc::clone(&self.0);
        HostPollable::new(move || {
            let i = Arc::clone(&i);
            Box::pin(async move {
                let mut i = i.lock().unwrap();
                match i.channel.recv().await {
                    None => i.state = StreamState::Closed,
                    Some(mut buf) => i.buffer.append(&mut buf),
                }
            })
        })
    }
}

impl tokio::io::AsyncRead for InputPipe {}

struct InnerOutputPipe {
    buffer: Vec<u8>,
    channel: tokio::sync::mpsc::Sender<Vec<u8>>,
}

pub struct OutputPipe(Arc<Mutex<InnerOutputPipe>>);

impl HostOutputStream for OutputPipe {}

impl tokio::io::AsyncWrite for OutputPipe {}

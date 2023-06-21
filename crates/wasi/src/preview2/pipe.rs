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
use std::sync::Arc;

pub fn pipe(bound: usize) -> (InputPipe, OutputPipe) {
    let (writer, reader) = tokio::sync::mpsc::channel(bound);

    let input = InnerInputPipe {
        state: StreamState::Open,
        buffer: Vec::new(),
        channel: reader,
    };

    let output = InnerOutputPipe {
        channel: SenderState::Channel(writer),
    };

    (
        InputPipe(Arc::new(tokio::sync::Mutex::new(input))),
        OutputPipe(Arc::new(tokio::sync::Mutex::new(output))),
    )
}

struct InnerInputPipe {
    state: StreamState,
    buffer: Vec<u8>,
    channel: tokio::sync::mpsc::Receiver<Vec<u8>>,
}

pub struct InputPipe(Arc<tokio::sync::Mutex<InnerInputPipe>>);

#[async_trait::async_trait]
impl HostInputStream for InputPipe {
    async fn read(&mut self, dest: &mut [u8]) -> Result<(u64, StreamState), Error> {
        let mut i = self.0.lock().await;
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
                let mut i = i.lock().await;
                match i.channel.recv().await {
                    None => i.state = StreamState::Closed,
                    Some(mut buf) => i.buffer.append(&mut buf),
                }
                Ok(())
            })
        })
    }
}

impl tokio::io::AsyncRead for InputPipe {}

enum SenderState {
    Writable(tokio::sync::OwnedPermit<Vec<u8>>),
    Channel(tokio::sync::mpsc::Sender<Vec<u8>>),
}

struct InnerOutputPipe {
    channel: SenderState,
}

pub struct OutputPipe(Arc<tokio::sync::Mutex<InnerOutputPipe>>);

#[async_trait::async_trait]
impl HostOutputStream for OutputPipe {
    async fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
        let mut i = self.0.lock().await;
        let bytes = Vec::from_iter(buf);
        match i.channel {
            SenderState::Writable(p) => {
                let s = p.send(bytes).await;
                i.channel = SenderState::Channel(s);
            }

            SenderState::Channel(s) => {
                s.send(bytes).await;
            }
        }

        Ok(buf.len() as u64)
    }

    fn pollable(&self) -> HostPollable {
        let i = Arc::clone(&self.0);
        HostPollable::new(move || {
            let i = Arc::clone(&i);
            Box::pin(async move {
                let mut i = i.lock().await;
                match i.channel.reserve_owned() {
                    Ok(p) => {
                        i.channel = SenderState::Writable(p);
                        Ok(())
                    }
                    Err(e) => Err(anyhow!(e)),
                }
            })
        })
    }
}

impl tokio::io::AsyncWrite for OutputPipe {}

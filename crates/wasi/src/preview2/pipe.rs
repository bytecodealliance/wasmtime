//! Virtual pipes.
//!
//! These types provide easy implementations of `WasiFile` that mimic much of the behavior of Unix
//! pipes. These are particularly helpful for redirecting WASI stdio handles to destinations other
//! than OS files.
//!
//! Some convenience constructors are included for common backing types like `Vec<u8>` and `String`,
//! but the virtual pipes can be instantiated with any `Read` or `Write` type.
//!
use crate::preview2::{HostInputStream, HostOutputStream, HostPollable, StreamState};
use anyhow::{anyhow, Error};
use std::sync::Arc;

pub fn pipe(bound: usize) -> (InputPipe, OutputPipe) {
    let (writer, reader) = tokio::sync::mpsc::channel(bound);

    (
        InputPipe(Arc::new(tokio::sync::Mutex::new(InnerInputPipe::new(
            reader,
        )))),
        OutputPipe(Arc::new(tokio::sync::Mutex::new(InnerOutputPipe::new(
            writer,
        )))),
    )
}

pub async fn empty_output() -> OutputPipe {
    let (writer, reader) = pipe(1);

    tokio::spawn(async move {
        let mut i = writer.0.lock().await;
        while let Some(_) = i.channel.recv().await {}
    });

    reader
}

pub async fn sink_input() -> InputPipe {
    let (writer, reader) = pipe(1);

    tokio::spawn(async move {
        let mut i = reader.0.lock().await;
        while !i.blocking_send(Vec::new()).await.is_err() {}
    });

    writer
}

struct InnerInputPipe {
    state: StreamState,
    buffer: Vec<u8>,
    channel: tokio::sync::mpsc::Receiver<Vec<u8>>,
}

impl InnerInputPipe {
    fn new(channel: tokio::sync::mpsc::Receiver<Vec<u8>>) -> Self {
        Self {
            state: StreamState::Open,
            buffer: Vec::new(),
            channel,
        }
    }
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

// impl tokio::io::AsyncRead for InputPipe {}

enum SenderState {
    Writable(tokio::sync::mpsc::OwnedPermit<Vec<u8>>),
    Channel(tokio::sync::mpsc::Sender<Vec<u8>>),
}

struct InnerOutputPipe {
    buffer: Vec<u8>,
    channel: Option<SenderState>,
}

impl InnerOutputPipe {
    fn new(s: tokio::sync::mpsc::Sender<Vec<u8>>) -> Self {
        Self {
            buffer: Vec::new(),
            channel: Some(SenderState::Channel(s)),
        }
    }

    async fn blocking_send(&mut self, buf: Vec<u8>) -> Result<(), Error> {
        let s = match self.take_channel() {
            SenderState::Writable(p) => {
                let s = p.send(buf);
                SenderState::Channel(s)
            }

            SenderState::Channel(s) => {
                s.send(buf).await?;
                SenderState::Channel(s)
            }
        };

        self.channel = Some(s);

        Ok(())
    }

    async fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        let bytes = core::mem::take(&mut self.buffer);

        self.blocking_send(bytes)
            .await
            .expect("fixme: handle closed write end later")
    }

    fn take_channel(&mut self) -> SenderState {
        self.channel.take().expect("Missing channel state")
    }
}

pub struct OutputPipe(Arc<tokio::sync::Mutex<InnerOutputPipe>>);

#[async_trait::async_trait]
impl HostOutputStream for OutputPipe {
    async fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
        use tokio::sync::mpsc::error::TrySendError;

        let mut i = self.0.lock().await;
        let mut bytes = core::mem::take(&mut i.buffer);
        bytes.extend(buf);
        let (s, bytes) = match i.take_channel() {
            SenderState::Writable(p) => {
                let s = p.send(bytes);
                (s, Vec::new())
            }

            SenderState::Channel(s) => match s.try_send(bytes) {
                Ok(()) => (s, Vec::new()),
                Err(TrySendError::Full(b)) => (s, b),
                Err(TrySendError::Closed(_)) => {
                    // TODO: we may need to communicate failure out in a way that doesn't result in
                    // a trap.
                    return Err(anyhow!("pipe closed"));
                }
            },
        };

        i.buffer = bytes;
        i.channel = Some(SenderState::Channel(s));

        Ok(buf.len() as u64)
    }

    fn pollable(&self) -> HostPollable {
        let i = Arc::clone(&self.0);
        HostPollable::new(move || {
            let i = Arc::clone(&i);
            Box::pin(async move {
                let mut i = i.lock().await;
                i.flush().await;
                let p = match i.channel.take().expect("Missing sender channel state") {
                    SenderState::Writable(p) => p,
                    SenderState::Channel(s) => s.reserve_owned().await?,
                };

                i.channel = Some(SenderState::Writable(p));

                Ok(())
            })
        })
    }
}

// impl tokio::io::AsyncWrite for OutputPipe {}

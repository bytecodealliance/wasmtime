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
use tokio::sync::Mutex;

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

#[derive(Clone)]
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

struct InnerWrappedRead<T> {
    state: StreamState,
    buffer: Vec<u8>,
    reader: T,
}

#[derive(Clone)]
pub struct WrappedRead<T>(Arc<Mutex<InnerWrappedRead<T>>>);

impl<T> WrappedRead<T> {
    pub fn new(reader: T) -> Self {
        Self(Arc::new(Mutex::new(InnerWrappedRead {
            state: StreamState::Open,
            buffer: Vec::new(),
            reader,
        })))
    }
}

#[async_trait::async_trait]
impl<T: tokio::io::AsyncRead + Send + Sync + Unpin + 'static> HostInputStream for WrappedRead<T> {
    async fn read(&mut self, mut dest: &mut [u8]) -> Result<(u64, StreamState), Error> {
        use std::io::Write;
        use tokio::io::AsyncReadExt;
        let mut i = self.0.lock().await;
        let l = dest.write(&i.buffer)?;

        i.buffer.drain(..l);
        if !i.buffer.is_empty() {
            return Ok((l as u64, StreamState::Open));
        }

        if i.state.is_closed() {
            return Ok((l as u64, StreamState::Closed));
        }

        let mut dest = &mut dest[l..];
        let rest = if !dest.is_empty() {
            let written = i.reader.read_buf(&mut dest).await?;
            if written == 0 {
                i.state = StreamState::Closed;
            }
            written
        } else {
            0
        };

        // TODO: figure out how we're tracking the stream state. Maybe mutate it when handling the
        // result of `read_buf`?
        Ok(((l + rest) as u64, i.state))
    }

    fn pollable(&self) -> HostPollable {
        use tokio::io::AsyncReadExt;
        let i = Arc::clone(&self.0);
        HostPollable::new(move || {
            let i = Arc::clone(&i);
            Box::pin(async move {
                let mut i = i.lock().await;

                if i.state.is_closed() {
                    return Ok(());
                }

                let mut bytes = core::mem::take(&mut i.buffer);
                let start = bytes.len();
                bytes.resize(start + 1024, 0);
                let l = i.reader.read_buf(&mut &mut bytes[start..]).await?;

                // Reading 0 bytes means either there wasn't enough space in the buffer (which we
                // know there is because we just resized) or that the stream has closed. Thus, we
                // know the stream has closed here.
                if l == 0 {
                    i.state = StreamState::Closed;
                }

                bytes.drain(start + l..);
                i.buffer = bytes;

                Ok(())
            })
        })
    }
}

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

#[derive(Clone)]
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

struct InnerWrappedWrite<T> {
    buffer: Vec<u8>,
    writer: T,
}

#[derive(Clone)]
pub struct WrappedWrite<T>(Arc<Mutex<InnerWrappedWrite<T>>>);

impl<T> WrappedWrite<T> {
    pub fn new(writer: T) -> Self {
        WrappedWrite(Arc::new(Mutex::new(InnerWrappedWrite {
            buffer: Vec::new(),
            writer,
        })))
    }
}

#[async_trait::async_trait]
impl<T: tokio::io::AsyncWrite + Send + Sync + Unpin + 'static> HostOutputStream
    for WrappedWrite<T>
{
    async fn write(&mut self, buf: &[u8]) -> Result<u64, anyhow::Error> {
        use tokio::io::AsyncWriteExt;
        let mut i = self.0.lock().await;
        let mut bytes = core::mem::take(&mut i.buffer);
        bytes.extend(buf);
        let written = i.writer.write_buf(&mut bytes.as_slice()).await?;
        bytes.drain(..written);
        i.buffer = bytes;
        Ok(written as u64)
    }

    fn pollable(&self) -> HostPollable {
        use tokio::io::AsyncWriteExt;
        let i = Arc::clone(&self.0);
        HostPollable::new(move || {
            let i = Arc::clone(&i);
            Box::pin(async move {
                let mut i = i.lock().await;
                let bytes = core::mem::take(&mut i.buffer);
                if !bytes.is_empty() {
                    i.writer.write_all(bytes.as_slice()).await?;
                }
                Ok(())
            })
        })
    }
}

#[derive(Debug)]
struct InnerMemoryOutputPipe {
    buffer: Vec<u8>,
}

#[derive(Clone)]
pub struct MemoryOutputPipe(Arc<Mutex<InnerMemoryOutputPipe>>);

impl MemoryOutputPipe {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(InnerMemoryOutputPipe {
            buffer: Vec::new(),
        })))
    }

    pub fn finalize(self) -> Vec<u8> {
        Arc::try_unwrap(self.0)
            .expect("finalizing MemoryOutputPipe")
            .into_inner()
            .buffer
    }
}

#[async_trait::async_trait]
impl HostOutputStream for MemoryOutputPipe {
    async fn write(&mut self, buf: &[u8]) -> Result<u64, anyhow::Error> {
        let mut i = self.0.lock().await;
        i.buffer.extend(buf);
        Ok(buf.len() as u64)
    }

    fn pollable(&self) -> HostPollable {
        // This stream is always ready for writing.
        HostPollable::new(|| Box::pin(async { Ok(()) }))
    }
}

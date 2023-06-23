//! Virtual pipes.
//!
//! These types provide easy implementations of `WasiFile` that mimic much of the behavior of Unix
//! pipes. These are particularly helpful for redirecting WASI stdio handles to destinations other
//! than OS files.
//!
//! Some convenience constructors are included for common backing types like `Vec<u8>` and `String`,
//! but the virtual pipes can be instantiated with any `Read` or `Write` type.
//!
use crate::preview2::{HostInputStream, HostOutputStream, StreamState};
use anyhow::{anyhow, Error};

pub fn pipe(bound: usize) -> (InputPipe, OutputPipe) {
    let (writer, reader) = tokio::sync::mpsc::channel(bound);

    (InputPipe::new(reader), OutputPipe::new(writer))
}

pub struct InputPipe {
    state: StreamState,
    buffer: Vec<u8>,
    channel: tokio::sync::mpsc::Receiver<Vec<u8>>,
}

impl InputPipe {
    fn new(channel: tokio::sync::mpsc::Receiver<Vec<u8>>) -> Self {
        Self {
            state: StreamState::Open,
            buffer: Vec::new(),
            channel,
        }
    }
}

#[async_trait::async_trait]
impl HostInputStream for InputPipe {
    fn read(&mut self, dest: &mut [u8]) -> Result<(u64, StreamState), Error> {
        let l = self.buffer.len().min(dest.len());
        let dest = &mut dest[..l];
        dest.copy_from_slice(&self.buffer[..l]);
        self.buffer = self.buffer.split_off(l);
        Ok((l as u64, self.state))
    }

    /*
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
    */
    async fn ready(&mut self) -> Result<(), Error> {
        todo!() // FIXME
    }
}

pub struct WrappedRead<T> {
    state: StreamState,
    buffer: Vec<u8>,
    reader: T,
}

impl<T> WrappedRead<T> {
    pub fn new(reader: T) -> Self {
        WrappedRead {
            state: StreamState::Open,
            buffer: Vec::new(),
            reader,
        }
    }
}

#[async_trait::async_trait]
impl<T: tokio::io::AsyncRead + Send + Sync + Unpin + 'static> HostInputStream for WrappedRead<T> {
    fn read(&mut self, mut dest: &mut [u8]) -> Result<(u64, StreamState), Error> {
        use std::io::Write;
        let l = dest.write(&self.buffer)?;

        self.buffer.drain(..l);
        if !self.buffer.is_empty() {
            return Ok((l as u64, StreamState::Open));
        }

        if self.state.is_closed() {
            return Ok((l as u64, StreamState::Closed));
        }

        let mut dest = &mut dest[l..];
        let rest = if !dest.is_empty() {
            let written = todo!() /* self.reader.read_buf(&mut dest).await? */ ; // FIXME we want to poll_read
                                                                                 // here
            if written == 0 {
                self.state = StreamState::Closed;
            }
            written
        } else {
            0
        };

        // TODO: figure out how we're tracking the stream state. Maybe mutate it when handling the
        // result of `read_buf`?
        Ok(((l + rest) as u64, self.state))
    }

    /*
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
    */
    async fn ready(&mut self) -> Result<(), Error> {
        todo!()
    }
}

enum SenderState {
    Writable(tokio::sync::mpsc::OwnedPermit<Vec<u8>>),
    Channel(tokio::sync::mpsc::Sender<Vec<u8>>),
}

pub struct OutputPipe {
    buffer: Vec<u8>,
    channel: Option<SenderState>,
}

impl OutputPipe {
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

#[async_trait::async_trait]
impl HostOutputStream for OutputPipe {
    fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
        use tokio::sync::mpsc::error::TrySendError;

        let mut bytes = core::mem::take(&mut self.buffer);
        bytes.extend(buf);
        let (s, bytes) = match self.take_channel() {
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

        self.buffer = bytes;
        self.channel = Some(SenderState::Channel(s));

        Ok(buf.len() as u64)
    }

    /*
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
    */
    async fn ready(&mut self) -> Result<(), Error> {
        todo!() // FIXME
    }
}

pub struct WrappedWrite<T> {
    buffer: Vec<u8>,
    writer: T,
}

impl<T> WrappedWrite<T> {
    pub fn new(writer: T) -> Self {
        WrappedWrite {
            buffer: Vec::new(),
            writer,
        }
    }
}

#[async_trait::async_trait]
impl<T: tokio::io::AsyncWrite + Send + Sync + Unpin + 'static> HostOutputStream
    for WrappedWrite<T>
{
    // I can get rid of the `async` here once the lock is no longer a tokio lock:
    fn write(&mut self, buf: &[u8]) -> Result<u64, anyhow::Error> {
        use std::pin::Pin;
        use std::task::{Context, Poll};
        let mut bytes = core::mem::take(&mut self.buffer);
        bytes.extend(buf);

        // FIXME: either Waker::noop is stable, or its something trivial to implement,
        // and we can use it here.
        let mut no_op_context: Context<'_> = todo!();
        // This is a true non-blocking call to the writer
        match Pin::new(&mut self.writer).poll_write(&mut no_op_context, &mut bytes.as_slice()) {
            Poll::Pending => {
                // Nothing was written: buffer all of it below.
            }
            Poll::Ready(written) => {
                // So much was written:
                bytes.drain(..written?);
            }
        }
        self.buffer = bytes;
        Ok(buf.len() as u64)
    }

    async fn ready(&mut self) -> Result<(), Error> {
        use tokio::io::AsyncWriteExt;
        let bytes = core::mem::take(&mut self.buffer);
        if !bytes.is_empty() {
            self.writer.write_all(bytes.as_slice()).await?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct MemoryOutputPipe {
    buffer: Vec<u8>,
}

impl MemoryOutputPipe {
    pub fn new() -> Self {
        MemoryOutputPipe { buffer: Vec::new() }
    }
}

#[async_trait::async_trait]
impl HostOutputStream for MemoryOutputPipe {
    fn write(&mut self, buf: &[u8]) -> Result<u64, anyhow::Error> {
        self.buffer.extend(buf);
        Ok(buf.len() as u64)
    }

    async fn ready(&mut self) -> Result<(), Error> {
        // This stream is always ready for writing.
        Ok(())
    }
}

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
use anyhow::Error;
use bytes::Bytes;

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
    fn read(&mut self) -> Result<(Bytes, StreamState), Error> {
        // use tokio::sync::mpsc::error::TryRecvError;
        // let read_from_buffer = self.buffer.len().min(dest.len());
        // let buffer_dest = &mut dest[..read_from_buffer];
        // buffer_dest.copy_from_slice(&self.buffer[..read_from_buffer]);
        // // Keep remaining contents in buffer
        // self.buffer = self.buffer.split_off(read_from_buffer);
        // if read_from_buffer < dest.len() {
        //     match self.channel.try_recv() {
        //         Ok(msg) => {
        //             let recv_dest = &mut dest[read_from_buffer..];
        //             if msg.len() < recv_dest.len() {
        //                 recv_dest[..msg.len()].copy_from_slice(&msg);
        //                 Ok(((read_from_buffer + msg.len()) as u64, self.state))
        //             } else {
        //                 recv_dest.copy_from_slice(&msg[..recv_dest.len()]);
        //                 self.buffer.extend_from_slice(&msg[recv_dest.len()..]);
        //                 Ok((dest.len() as u64, self.state))
        //             }
        //         }
        //         Err(TryRecvError::Empty) => Ok((read_from_buffer as u64, self.state)),
        //         Err(TryRecvError::Disconnected) => {
        //             self.state = StreamState::Closed;
        //             Ok((read_from_buffer as u64, self.state))
        //         }
        //     }
        // } else {
        //     Ok((read_from_buffer as u64, self.state))
        // }
        todo!()
    }

    async fn ready(&mut self) -> Result<(), Error> {
        match self.channel.recv().await {
            None => self.state = StreamState::Closed,
            Some(mut buf) => self.buffer.append(&mut buf),
        }
        Ok(())
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
    fn write(&mut self, buf: Bytes) -> Result<u64, Error> {
        // use tokio::sync::mpsc::error::TrySendError;
        //
        // let mut bytes = core::mem::take(&mut self.buffer);
        // bytes.extend(buf);
        // let (s, bytes) = match self.take_channel() {
        //     SenderState::Writable(p) => {
        //         let s = p.send(bytes);
        //         (s, Vec::new())
        //     }
        //
        //     SenderState::Channel(s) => match s.try_send(bytes) {
        //         Ok(()) => (s, Vec::new()),
        //         Err(TrySendError::Full(b)) => (s, b),
        //         Err(TrySendError::Closed(_)) => {
        //             // TODO: we may need to communicate failure out in a way that doesn't result in
        //             // a trap.
        //             return Err(anyhow::anyhow!("pipe closed"));
        //         }
        //     },
        // };
        //
        // self.buffer = bytes;
        // self.channel = Some(SenderState::Channel(s));
        //
        // Ok(buf.len() as u64)
        todo!()
    }

    async fn ready(&mut self) -> Result<(), Error> {
        self.flush().await;
        let p = match self.channel.take().expect("Missing sender channel state") {
            SenderState::Writable(p) => p,
            SenderState::Channel(s) => s.reserve_owned().await?,
        };

        self.channel = Some(SenderState::Writable(p));

        Ok(())
    }
}

#[derive(Debug)]
pub struct MemoryInputPipe {
    buffer: std::io::Cursor<Vec<u8>>,
}

impl MemoryInputPipe {
    pub fn new(bytes: impl AsRef<[u8]>) -> Self {
        Self {
            buffer: std::io::Cursor::new(Vec::from(bytes.as_ref())),
        }
    }
}

#[async_trait::async_trait]
impl HostInputStream for MemoryInputPipe {
    fn read(&mut self) -> Result<(Bytes, StreamState), Error> {
        // let nbytes = std::io::Read::read(&mut self.buffer, dest)?;
        // let state = if self.buffer.get_ref().len() as u64 == self.buffer.position() {
        //     StreamState::Closed
        // } else {
        //     StreamState::Open
        // };
        // Ok((nbytes as u64, state))
        todo!()
    }
    async fn ready(&mut self) -> Result<(), Error> {
        if self.buffer.get_ref().len() as u64 > self.buffer.position() {
            Ok(())
        } else {
            futures_util::future::pending().await
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryOutputPipe {
    buffer: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
}

impl MemoryOutputPipe {
    pub fn new() -> Self {
        MemoryOutputPipe {
            buffer: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
    pub fn contents(&self) -> Vec<u8> {
        self.buffer.lock().unwrap().clone()
    }
    pub fn try_into_inner(self) -> Result<Vec<u8>, Self> {
        match std::sync::Arc::try_unwrap(self.buffer) {
            Ok(m) => Ok(m.into_inner().map_err(|_| ()).expect("mutex poisioned")),
            Err(buffer) => Err(Self { buffer }),
        }
    }
}

#[async_trait::async_trait]
impl HostOutputStream for MemoryOutputPipe {
    fn write(&mut self, buf: Bytes) -> Result<u64, anyhow::Error> {
        // self.buffer.lock().unwrap().extend(buf);
        // Ok(buf.len() as u64)
        todo!()
    }

    async fn ready(&mut self) -> Result<(), Error> {
        // This stream is always ready for writing.
        Ok(())
    }
}

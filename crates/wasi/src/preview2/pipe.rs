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
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

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
        use tokio::sync::mpsc::error::TryRecvError;
        let read_from_buffer = self.buffer.len().min(dest.len());
        let buffer_dest = &mut dest[..read_from_buffer];
        buffer_dest.copy_from_slice(&self.buffer[..read_from_buffer]);
        // Keep remaining contents in buffer
        self.buffer = self.buffer.split_off(read_from_buffer);
        if read_from_buffer < dest.len() {
            match self.channel.try_recv() {
                Ok(msg) => {
                    let recv_dest = &mut dest[read_from_buffer..];
                    if msg.len() < recv_dest.len() {
                        recv_dest[..msg.len()].copy_from_slice(&msg);
                        Ok(((read_from_buffer + msg.len()) as u64, self.state))
                    } else {
                        recv_dest.copy_from_slice(&msg[..recv_dest.len()]);
                        self.buffer.extend_from_slice(&msg[recv_dest.len()..]);
                        Ok((dest.len() as u64, self.state))
                    }
                }
                Err(TryRecvError::Empty) => Ok((read_from_buffer as u64, self.state)),
                Err(TryRecvError::Disconnected) => {
                    self.state = StreamState::Closed;
                    Ok((read_from_buffer as u64, self.state))
                }
            }
        } else {
            Ok((read_from_buffer as u64, self.state))
        }
    }

    async fn ready(&mut self) -> Result<(), Error> {
        match self.channel.recv().await {
            None => self.state = StreamState::Closed,
            Some(mut buf) => self.buffer.append(&mut buf),
        }
        Ok(())
    }
}

#[cfg(unix)]
pub use async_fd_stream::*;

#[cfg(unix)]
mod async_fd_stream {
    use super::{HostInputStream, HostOutputStream, StreamState};
    use anyhow::Error;
    use std::io::{Read, Write};
    use std::os::fd::{AsFd, AsRawFd, FromRawFd, IntoRawFd};
    use tokio::io::unix::AsyncFd;

    pub struct AsyncFdStream<T: AsRawFd> {
        fd: AsyncFd<T>,
    }

    impl<T: AsRawFd> AsyncFdStream<T> {
        pub fn new(fd: T) -> Self {
            Self {
                fd: AsyncFd::new(fd).unwrap(),
            }
        }
    }

    #[async_trait::async_trait]
    impl<T: AsRawFd + Send + Sync + Unpin + 'static> HostInputStream for AsyncFdStream<T> {
        fn read(&mut self, mut dest: &mut [u8]) -> Result<(u64, StreamState), Error> {
            // Safety: we're the only one accessing this fd, and we turn it back into a raw fd when
            // we're done.
            let mut file = unsafe { std::fs::File::from_raw_fd(self.fd.as_raw_fd()) };

            // TODO: how sure are we that this is non-blocking?
            let read_res = file.read(dest);

            // Make sure that the file doesn't close the fd when it's dropped.
            file.into_raw_fd();

            let n = read_res?;

            // TODO: figure out when the stream should be considered closed
            // TODO: figure out how to handle the error conditions from the read call above

            Ok((n as u64, StreamState::Open))
        }

        async fn ready(&mut self) -> Result<(), Error> {
            self.fd.readable().await?;
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl<T: AsRawFd + Send + Sync + Unpin + 'static> HostOutputStream for AsyncFdStream<T> {
        fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
            // Safety: we're the only one accessing this fd, and we turn it back into a raw fd when
            // we're done.
            let mut file = unsafe { std::fs::File::from_raw_fd(self.fd.as_raw_fd()) };

            // TODO: how sure are we that this is non-blocking?
            let write_res = file.write(buf);

            // Make sure that the file doesn't close the fd when it's dropped.
            file.into_raw_fd();

            let n = write_res?;

            // TODO: figure out when the stream should be considered closed
            // TODO: figure out how to handle the error conditions from the write call above

            Ok(n as u64)
        }

        async fn ready(&mut self) -> Result<(), Error> {
            self.fd.writable().await?;
            Ok(())
        }
    }
}

pub struct AsyncReadStream<T> {
    state: StreamState,
    buffer: Vec<u8>,
    reader: T,
}

impl<T> AsyncReadStream<T> {
    pub fn new(reader: T) -> Self {
        AsyncReadStream {
            state: StreamState::Open,
            buffer: Vec::new(),
            reader,
        }
    }
}

#[async_trait::async_trait]
impl<T: tokio::io::AsyncRead + Send + Sync + Unpin + 'static> HostInputStream for AsyncReadStream<T> {
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
            let mut readbuf = tokio::io::ReadBuf::new(dest);

            let noop_waker = noop_waker();
            let mut cx: Context<'_> = Context::from_waker(&noop_waker);
            // Make a synchronous, non-blocking call attempt to read. We are not
            // going to poll this more than once, so the noop waker is appropriate.
            match Pin::new(&mut self.reader).poll_read(&mut cx, &mut readbuf) {
                Poll::Pending => {}             // Nothing was read
                Poll::Ready(result) => result?, // Maybe an error occured
            };
            let bytes_read = readbuf.filled().len();

            if bytes_read == 0 {
                self.state = StreamState::Closed;
            }
            bytes_read
        } else {
            0
        };

        Ok(((l + rest) as u64, self.state))
    }

    async fn ready(&mut self) -> Result<(), Error> {
        if self.state.is_closed() {
            return Ok(());
        }

        let mut bytes = core::mem::take(&mut self.buffer);
        let start = bytes.len();
        bytes.resize(start + 1024, 0);
        let l =
            tokio::io::AsyncReadExt::read_buf(&mut self.reader, &mut &mut bytes[start..]).await?;

        // Reading 0 bytes means either there wasn't enough space in the buffer (which we
        // know there is because we just resized) or that the stream has closed. Thus, we
        // know the stream has closed here.
        if l == 0 {
            self.state = StreamState::Closed;
        }

        bytes.drain(start + l..);
        self.buffer = bytes;

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

pub struct AsyncWriteStream<T> {
    buffer: Vec<u8>,
    writer: T,
}

impl<T> AsyncWriteStream<T> {
    pub fn new(writer: T) -> Self {
        AsyncWriteStream {
            buffer: Vec::new(),
            writer,
        }
    }
}

#[async_trait::async_trait]
impl<T: tokio::io::AsyncWrite + Send + Sync + Unpin + 'static> HostOutputStream
    for AsyncWriteStream<T>
{
    // I can get rid of the `async` here once the lock is no longer a tokio lock:
    fn write(&mut self, buf: &[u8]) -> Result<u64, anyhow::Error> {
        let mut bytes = core::mem::take(&mut self.buffer);
        bytes.extend(buf);

        let noop_waker = noop_waker();
        let mut cx: Context<'_> = Context::from_waker(&noop_waker);
        // Make a synchronous, non-blocking call attempt to write. We are not
        // going to poll this more than once, so the noop waker is appropriate.
        match Pin::new(&mut self.writer).poll_write(&mut cx, &mut bytes.as_slice()) {
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
    pub fn finalize(self) -> Vec<u8> {
        std::sync::Arc::try_unwrap(self.buffer)
            .map_err(|_| ())
            .expect("more than one outstanding reference")
            .into_inner()
            .map_err(|_| ())
            .expect("mutex poisioned")
    }
}

#[async_trait::async_trait]
impl HostOutputStream for MemoryOutputPipe {
    fn write(&mut self, buf: &[u8]) -> Result<u64, anyhow::Error> {
        self.buffer.lock().unwrap().extend(buf);
        Ok(buf.len() as u64)
    }

    async fn ready(&mut self) -> Result<(), Error> {
        // This stream is always ready for writing.
        Ok(())
    }
}

// This implementation is basically copy-pasted out of `std` because the
// implementation there has not yet stabilized. When the `noop_waker` feature
// stabilizes, replace this with std::task::Waker::noop().
fn noop_waker() -> Waker {
    use std::task::{RawWaker, RawWakerVTable};
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        // Cloning just returns a new no-op raw waker
        |_| RAW,
        // `wake` does nothing
        |_| {},
        // `wake_by_ref` does nothing
        |_| {},
        // Dropping does nothing as we don't allocate anything
        |_| {},
    );
    const RAW: RawWaker = RawWaker::new(std::ptr::null(), &VTABLE);

    unsafe { Waker::from_raw(RAW) }
}

use crate::preview2::{Table, TableError};
use anyhow::Error;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StreamState {
    Open,
    Closed,
}

impl StreamState {
    pub fn is_closed(&self) -> bool {
        *self == Self::Closed
    }
}

/// Host trait for implementing the `wasi:io/streams.input-stream` resource: A
/// bytestream which can be read from.
#[async_trait::async_trait]
pub trait HostInputStream: Send + Sync {
    /// Read bytes. On success, returns a pair holding the number of bytes
    /// read and a flag indicating whether the end of the stream was reached.
    /// Important: this read must be non-blocking!
    fn read(&mut self, buf: &mut [u8]) -> Result<(u64, StreamState), Error>;

    /// Read bytes from a stream and discard them. Important: this method must
    /// be non-blocking!
    fn skip(&mut self, nelem: u64) -> Result<(u64, StreamState), Error> {
        let mut nread = 0;
        let mut state = StreamState::Open;

        // TODO: Optimize by reading more than one byte at a time.
        for _ in 0..nelem {
            let (num, read_state) = self.read(&mut [0])?;
            nread += num;
            if read_state.is_closed() {
                state = read_state;
                break;
            }
        }

        Ok((nread, state))
    }

    /// Check for read readiness: this method blocks until the stream is ready
    /// for reading.
    async fn ready(&mut self) -> Result<(), Error>;
}

/// Host trait for implementing the `wasi:io/streams.output-stream` resource:
/// A bytestream which can be written to.
#[async_trait::async_trait]
pub trait HostOutputStream: Send + Sync {
    /// Write bytes. On success, returns the number of bytes written.
    /// Important: this write must be non-blocking!
    fn write(&mut self, _buf: &[u8]) -> Result<u64, Error>;

    /// Transfer bytes directly from an input stream to an output stream.
    /// Important: this splice must be non-blocking!
    fn splice(
        &mut self,
        src: &mut dyn HostInputStream,
        nelem: u64,
    ) -> Result<(u64, StreamState), Error> {
        let mut nspliced = 0;
        let mut state = StreamState::Open;

        // TODO: Optimize by splicing more than one byte at a time.
        for _ in 0..nelem {
            let mut buf = [0u8];
            let (num, read_state) = src.read(&mut buf)?;
            self.write(&buf)?;
            nspliced += num;
            if read_state.is_closed() {
                state = read_state;
                break;
            }
        }

        Ok((nspliced, state))
    }

    /// Repeatedly write a byte to a stream. Important: this write must be
    /// non-blocking!
    fn write_zeroes(&mut self, nelem: u64) -> Result<u64, Error> {
        let mut nwritten = 0;

        // TODO: Optimize by writing more than one byte at a time.
        for _ in 0..nelem {
            let num = self.write(&[0])?;
            if num == 0 {
                break;
            }
            nwritten += num;
        }

        Ok(nwritten)
    }

    /// Check for write readiness: this method blocks until the stream is
    /// ready for writing.
    async fn ready(&mut self) -> Result<(), Error>;
}

/// Extension trait for managing [`HostInputStream`]s and [`HostOutputStream`]s in the [`Table`].
pub trait TableStreamExt {
    /// Push a [`HostInputStream`] into a [`Table`], returning the table index.
    fn push_input_stream(&mut self, istream: Box<dyn HostInputStream>) -> Result<u32, TableError>;
    /// Get a mutable reference to a [`HostInputStream`] in a [`Table`].
    fn get_input_stream_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut Box<dyn HostInputStream>, TableError>;

    /// Push a [`HostOutputStream`] into a [`Table`], returning the table index.
    fn push_output_stream(&mut self, ostream: Box<dyn HostOutputStream>)
        -> Result<u32, TableError>;
    /// Get a mutable reference to a [`HostOutputStream`] in a [`Table`].
    fn get_output_stream_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut Box<dyn HostOutputStream>, TableError>;
}
impl TableStreamExt for Table {
    fn push_input_stream(&mut self, istream: Box<dyn HostInputStream>) -> Result<u32, TableError> {
        self.push(Box::new(istream))
    }
    fn get_input_stream_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut Box<dyn HostInputStream>, TableError> {
        self.get_mut::<Box<dyn HostInputStream>>(fd)
    }

    fn push_output_stream(
        &mut self,
        ostream: Box<dyn HostOutputStream>,
    ) -> Result<u32, TableError> {
        self.push(Box::new(ostream))
    }
    fn get_output_stream_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut Box<dyn HostOutputStream>, TableError> {
        self.get_mut::<Box<dyn HostOutputStream>>(fd)
    }
}

/// Provides a [`HostInputStream`] impl from a [`tokio::io::AsyncRead`] impl
pub struct AsyncReadStream<T> {
    state: StreamState,
    buffer: Vec<u8>,
    reader: T,
}

impl<T> AsyncReadStream<T> {
    /// Create a [`AsyncReadStream`]. In order to use the [`HostInputStream`] impl
    /// provided by this struct, the argument must impl [`tokio::io::AsyncRead`].
    pub fn new(reader: T) -> Self {
        AsyncReadStream {
            state: StreamState::Open,
            buffer: Vec::new(),
            reader,
        }
    }
}

#[async_trait::async_trait]
impl<T: tokio::io::AsyncRead + Send + Sync + Unpin + 'static> HostInputStream
    for AsyncReadStream<T>
{
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

        let dest = &mut dest[l..];
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

/// Provides a [`HostOutputStream`] impl from a [`tokio::io::AsyncWrite`] impl
pub struct AsyncWriteStream<T> {
    buffer: Vec<u8>,
    writer: T,
}

impl<T> AsyncWriteStream<T> {
    /// Create a [`AsyncWriteStream`]. In order to use the [`HostOutputStream`] impl
    /// provided by this struct, the argument must impl [`tokio::io::AsyncWrite`].
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

#[cfg(unix)]
pub use async_fd_stream::*;

#[cfg(unix)]
mod async_fd_stream {
    use super::{HostInputStream, HostOutputStream, StreamState};
    use anyhow::Error;
    use std::io::{Read, Write};
    use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
    use tokio::io::unix::AsyncFd;

    /// Provides a [`HostInputStream`] and [`HostOutputStream`] impl from an
    /// [`std::os::fd::AsRawFd`] impl, using [`tokio::io::unix::AsyncFd`]
    pub struct AsyncFdStream<T: AsRawFd> {
        fd: AsyncFd<T>,
    }

    impl<T: AsRawFd> AsyncFdStream<T> {
        /// Create a [`AsyncFdStream`] from a type which implements [`AsRawFd`].
        /// This constructor will use `fcntl(2)` to set the `O_NONBLOCK` flag
        /// if it is not already set.
        /// The implementation of this constructor creates an
        /// [`tokio::io::unix::AsyncFd`]. It will return an error unless
        /// called from inside a tokio context. Additionally, tokio (via mio)
        /// will register the fd inside with `epoll(7)` (or equiv on
        /// macos). The process may only make one registration of an fd at a
        /// time. If another registration exists, this constructor will return
        /// an error.
        pub fn new(fd: T) -> anyhow::Result<Self> {
            use rustix::fs::OFlags;
            let borrowed_fd = unsafe { rustix::fd::BorrowedFd::borrow_raw(fd.as_raw_fd()) };
            tokio::task::block_in_place(|| {
                let flags = rustix::fs::fcntl_getfl(borrowed_fd)?;
                if !flags.contains(OFlags::NONBLOCK) {
                    rustix::fs::fcntl_setfl(borrowed_fd, flags.difference(OFlags::NONBLOCK))?;
                }
                Ok::<(), anyhow::Error>(())
            })?;
            Ok(Self {
                fd: AsyncFd::new(fd)?,
            })
        }
    }

    #[async_trait::async_trait]
    impl<T: AsRawFd + Send + Sync + Unpin + 'static> HostInputStream for AsyncFdStream<T> {
        fn read(&mut self, dest: &mut [u8]) -> Result<(u64, StreamState), Error> {
            // Safety: we're the only one accessing this fd, and we turn it back into a raw fd when
            // we're done.
            let mut file = unsafe { std::fs::File::from_raw_fd(self.fd.as_raw_fd()) };

            // Ensured this is nonblocking at construction of AsyncFdStream.
            let read_res = file.read(dest);

            // Make sure that the file doesn't close the fd when it's dropped.
            file.into_raw_fd();

            let n = read_res?;

            // TODO: figure out when the stream should be considered closed
            // TODO: figure out how to handle the error conditions from the read call above

            Ok((n as u64, StreamState::Open))
        }

        async fn ready(&mut self) -> Result<(), Error> {
            let _ = self.fd.readable().await?;
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl<T: AsRawFd + Send + Sync + Unpin + 'static> HostOutputStream for AsyncFdStream<T> {
        fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
            // Safety: we're the only one accessing this fd, and we turn it back into a raw fd when
            // we're done.
            let mut file = unsafe { std::fs::File::from_raw_fd(self.fd.as_raw_fd()) };

            // Ensured this is nonblocking at construction of AsyncFdStream.
            let write_res = file.write(buf);

            // Make sure that the file doesn't close the fd when it's dropped.
            file.into_raw_fd();

            let n = write_res?;

            // TODO: figure out how to handle the error conditions from the write call above

            Ok(n as u64)
        }

        async fn ready(&mut self) -> Result<(), Error> {
            let _ = self.fd.writable().await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn input_stream_in_table() {
        struct DummyInputStream;
        #[async_trait::async_trait]
        impl HostInputStream for DummyInputStream {
            fn read(&mut self, _: &mut [u8]) -> Result<(u64, StreamState), Error> {
                unimplemented!();
            }
            async fn ready(&mut self) -> Result<(), Error> {
                unimplemented!();
            }
        }

        let dummy = DummyInputStream;
        let mut table = Table::new();
        // Show that we can put an input stream in the table, and get a mut
        // ref back out:
        let ix = table.push_input_stream(Box::new(dummy)).unwrap();
        let _ = table.get_input_stream_mut(ix).unwrap();
    }

    #[test]
    fn output_stream_in_table() {
        struct DummyOutputStream;
        #[async_trait::async_trait]
        impl HostOutputStream for DummyOutputStream {
            fn write(&mut self, _: &[u8]) -> Result<u64, Error> {
                unimplemented!();
            }
            async fn ready(&mut self) -> Result<(), Error> {
                unimplemented!();
            }
        }

        let dummy = DummyOutputStream;
        let mut table = Table::new();
        // Show that we can put an output stream in the table, and get a mut
        // ref back out:
        let ix = table.push_output_stream(Box::new(dummy)).unwrap();
        let _ = table.get_output_stream_mut(ix).unwrap();
    }
}

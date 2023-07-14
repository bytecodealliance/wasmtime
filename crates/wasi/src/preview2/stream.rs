use crate::preview2::{Table, TableError};
use anyhow::Error;
use bytes::Bytes;
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
    fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), Error>;

    /// Read bytes from a stream and discard them. Important: this method must
    /// be non-blocking!
    fn skip(&mut self, nelem: usize) -> Result<(usize, StreamState), Error> {
        let mut nread = 0;
        let mut state = StreamState::Open;

        let (bs, read_state) = self.read(nelem)?;
        // TODO: handle the case where `bs.len()` is less than `nelem`
        nread += bs.len();
        if read_state.is_closed() {
            state = read_state;
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
    fn write(&mut self, bytes: Bytes) -> Result<(usize, StreamState), Error>;

    /// Transfer bytes directly from an input stream to an output stream.
    /// Important: this splice must be non-blocking!
    fn splice(
        &mut self,
        src: &mut dyn HostInputStream,
        nelem: usize,
    ) -> Result<(usize, StreamState), Error> {
        let mut nspliced = 0;
        let mut state = StreamState::Open;

        // TODO: handle the case where `bs.len()` is less than `nelem`
        let (bs, read_state) = src.read(nelem)?;
        // TODO: handle the case where write returns less than `bs.len()`
        let (nwritten, _write_state) = self.write(bs)?;
        nspliced += nwritten;
        if read_state.is_closed() {
            state = read_state;
        }

        Ok((nspliced, state))
    }

    /// Repeatedly write a byte to a stream. Important: this write must be
    /// non-blocking!
    fn write_zeroes(&mut self, nelem: usize) -> Result<(usize, StreamState), Error> {
        // TODO: We could optimize this to not allocate one big zeroed buffer, and instead write
        // repeatedly from a 'static buffer of zeros.
        let bs = Bytes::from_iter(core::iter::repeat(0 as u8).take(nelem));
        let r = self.write(bs)?;
        Ok(r)
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
pub struct AsyncReadStream {
    state: StreamState,
    buffer: Option<Result<Bytes, std::io::Error>>,
    receiver: tokio::sync::mpsc::Receiver<Result<(Bytes, StreamState), std::io::Error>>,
}

impl AsyncReadStream {
    /// Create a [`AsyncReadStream`]. In order to use the [`HostInputStream`] impl
    /// provided by this struct, the argument must impl [`tokio::io::AsyncRead`].
    pub fn new<T: tokio::io::AsyncRead + Send + Sync + Unpin + 'static>(mut reader: T) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            loop {
                use tokio::io::AsyncReadExt;
                let mut buf = bytes::BytesMut::with_capacity(4096);
                let sent = match reader.read_buf(&mut buf).await {
                    Ok(nbytes) if nbytes == 0 => {
                        sender.send(Ok((Bytes::new(), StreamState::Closed))).await
                    }
                    Ok(_) => sender.send(Ok((buf.freeze(), StreamState::Open))).await,
                    Err(e) => sender.send(Err(e)).await,
                };
                if sent.is_err() {
                    // no more receiver - stop trying to read
                    break;
                }
            }
        });
        AsyncReadStream {
            state: StreamState::Open,
            buffer: None,
            receiver,
        }
    }
}

#[async_trait::async_trait]
impl HostInputStream for AsyncReadStream {
    fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), Error> {
        use tokio::sync::mpsc::error::TryRecvError;

        match self.buffer.take() {
            Some(Ok(mut bytes)) => {
                // TODO: de-duplicate the buffer management with the case below
                let len = bytes.len().min(size);
                let rest = bytes.split_off(len);
                let return_state = if !rest.is_empty() {
                    self.buffer = Some(Ok(rest));
                    StreamState::Open
                } else {
                    self.state
                };
                return Ok((bytes, return_state));
            }
            Some(Err(e)) => return Err(e.into()),
            None => {}
        }

        match dbg!(self.receiver.try_recv()) {
            Ok(Ok((mut bytes, state))) => {
                self.state = state;

                let len = bytes.len().min(size);
                let rest = bytes.split_off(len);
                let return_state = if !rest.is_empty() {
                    self.buffer = Some(Ok(rest));
                    StreamState::Open
                } else {
                    self.state
                };

                Ok((bytes, return_state))
            }
            Ok(Err(e)) => Err(e.into()),
            Err(TryRecvError::Empty) => Ok((Bytes::new(), self.state)),
            Err(TryRecvError::Disconnected) => Err(anyhow::anyhow!(
                "AsyncReadStream sender died - should be impossible"
            )),
        }
    }

    async fn ready(&mut self) -> Result<(), Error> {
        if self.buffer.is_some() || self.state == StreamState::Closed {
            return Ok(());
        }
        match self.receiver.recv().await {
            Some(Ok((bytes, state))) => {
                if state == StreamState::Closed {
                    self.state = state;
                }
                self.buffer = Some(Ok(bytes));
            }
            Some(Err(e)) => self.buffer = Some(Err(e)),
            None => {
                return Err(anyhow::anyhow!(
                    "no more sender for an open AsyncReadStream - should be impossible"
                ))
            }
        }
        Ok(())
    }
}

enum WriteState {
    Ready,
    Pending,
    Err(std::io::Error),
}

/// Provides a [`HostOutputStream`] impl from a [`tokio::io::AsyncWrite`] impl
pub struct AsyncWriteStream {
    state: Option<WriteState>,
    sender: tokio::sync::mpsc::Sender<Bytes>,
    result_receiver: tokio::sync::mpsc::Receiver<Result<StreamState, std::io::Error>>,
}

impl AsyncWriteStream {
    /// Create a [`AsyncWriteStream`]. In order to use the [`HostOutputStream`] impl
    /// provided by this struct, the argument must impl [`tokio::io::AsyncWrite`].
    pub fn new<T: tokio::io::AsyncWrite + Send + Sync + Unpin + 'static>(mut writer: T) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Bytes>(1);
        let (result_sender, result_receiver) = tokio::sync::mpsc::channel(1);

        tokio::spawn(async move {
            'outer: loop {
                use tokio::io::AsyncWriteExt;
                match receiver.recv().await {
                    Some(mut bytes) => {
                        while !bytes.is_empty() {
                            match writer.write_buf(&mut bytes).await {
                                Ok(0) => {
                                    let _ = result_sender.send(Ok(StreamState::Closed)).await;
                                    break 'outer;
                                }

                                Ok(_) => {
                                    if bytes.is_empty() {
                                        match result_sender.send(Ok(StreamState::Open)).await {
                                            Ok(_) => break,
                                            Err(_) => break 'outer,
                                        }
                                    }
                                    continue;
                                }

                                Err(e) => match result_sender.send(Err(e)).await {
                                    Ok(_) => break,
                                    Err(_) => break 'outer,
                                },
                            }
                        }
                    }

                    // The other side of the channel hung up, the task can exit now
                    None => break 'outer,
                }
            }
        });

        AsyncWriteStream {
            state: Some(WriteState::Ready),
            sender,
            result_receiver,
        }
    }
}

#[async_trait::async_trait]
impl HostOutputStream for AsyncWriteStream {
    fn write(&mut self, bytes: Bytes) -> Result<(usize, StreamState), anyhow::Error> {
        use tokio::sync::mpsc::error::{TryRecvError, TrySendError};

        match self.state.take() {
            Some(WriteState::Ready) => {}
            Some(WriteState::Pending) => match self.result_receiver.try_recv() {
                Ok(Ok(StreamState::Open)) => {
                    self.state = Some(WriteState::Ready);
                }

                Ok(Ok(StreamState::Closed)) => {
                    self.state = None;
                    return Ok((0, StreamState::Closed));
                }

                Ok(Err(e)) => {
                    self.state = Some(WriteState::Ready);
                    return Err(e.into());
                }

                Err(TryRecvError::Empty) => {
                    self.state = Some(WriteState::Pending);
                    return Ok((0, StreamState::Open));
                }

                Err(TryRecvError::Disconnected) => {
                    unreachable!("task shouldn't die while pending")
                }
            },
            Some(WriteState::Err(e)) => {
                self.state = Some(WriteState::Ready);
                return Err(e.into());
            }

            None => {
                return Ok((0, StreamState::Closed));
            }
        }

        let len = bytes.len();
        match self.sender.try_send(bytes) {
            Ok(_) => Ok((len, StreamState::Open)),
            Err(TrySendError::Full(_)) => Ok((0, StreamState::Open)),
            Err(TrySendError::Closed(_)) => unreachable!("task shouldn't die while not closed"),
        }
    }

    async fn ready(&mut self) -> Result<(), Error> {
        match &self.state {
            Some(WriteState::Pending) => match self.result_receiver.recv().await {
                Some(Ok(StreamState::Open)) => {
                    self.state = Some(WriteState::Ready);
                }

                Some(Ok(StreamState::Closed)) => {
                    self.state = None;
                }

                Some(Err(e)) => {
                    self.state = Some(WriteState::Err(e));
                }

                None => unreachable!("task shouldn't die while pending"),
            },

            Some(WriteState::Ready | WriteState::Err(_)) | None => {}
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
    use bytes::Bytes;
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
        fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), Error> {
            // // Safety: we're the only one accessing this fd, and we turn it back into a raw fd when
            // // we're done.
            // let mut file = unsafe { std::fs::File::from_raw_fd(self.fd.as_raw_fd()) };
            //
            // // Ensured this is nonblocking at construction of AsyncFdStream.
            // let read_res = file.read(dest);
            //
            // // Make sure that the file doesn't close the fd when it's dropped.
            // file.into_raw_fd();
            //
            // let n = read_res?;
            //
            // // TODO: figure out when the stream should be considered closed
            // // TODO: figure out how to handle the error conditions from the read call above
            //
            // Ok((n as u64, StreamState::Open))
            todo!()
        }

        async fn ready(&mut self) -> Result<(), Error> {
            /*
            let _ = self.fd.readable().await?;
            Ok(())
            */
            todo!()
        }
    }

    #[async_trait::async_trait]
    impl<T: AsRawFd + Send + Sync + Unpin + 'static> HostOutputStream for AsyncFdStream<T> {
        fn write(&mut self, bytes: Bytes) -> Result<(usize, StreamState), Error> {
            // // Safety: we're the only one accessing this fd, and we turn it back into a raw fd when
            // // we're done.
            // let mut file = unsafe { std::fs::File::from_raw_fd(self.fd.as_raw_fd()) };
            //
            // // Ensured this is nonblocking at construction of AsyncFdStream.
            // let write_res = file.write(buf);
            //
            // // Make sure that the file doesn't close the fd when it's dropped.
            // file.into_raw_fd();
            //
            // let n = write_res?;
            //
            // // TODO: figure out how to handle the error conditions from the write call above
            //
            // Ok(n as u64)
            todo!()
        }

        async fn ready(&mut self) -> Result<(), Error> {
            /*
            let _ = self.fd.writable().await?;
            Ok(())
            */
            todo!()
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
            fn read(&mut self, _size: usize) -> Result<(Bytes, StreamState), Error> {
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
            fn write(&mut self, _: Bytes) -> Result<(usize, StreamState), Error> {
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_empty_read_stream() {
        use tokio::io::AsyncReadExt;
        let mut buf = bytes::BytesMut::with_capacity(4096);
        let sent = tokio::io::empty().read_buf(&mut buf).await.unwrap();
        assert_eq!(sent, 0);

        let mut reader = AsyncReadStream::new(tokio::io::empty());
        let (bs, state) = reader.read(10).unwrap();
        assert!(bs.is_empty());

        // In a multi-threaded context, the value of state is not deterministic -- the spawned
        // reader task may run on a different thread.
        match state {
            // The reader task ran before we tried to read, and noticed that the input was empty.
            StreamState::Closed => {}

            // The reader task hasn't run yet, and we need to call `ready` to turn the crank.
            StreamState::Open => {
                tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
                    .await
                    .expect("the reader should be ready instantly")
                    .expect("ready is ok");
                let (bs, state) = reader.read(10).unwrap();
                assert!(bs.is_empty());
                assert_eq!(state, StreamState::Closed);
            }
        }
    }
}

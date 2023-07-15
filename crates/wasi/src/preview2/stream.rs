use crate::preview2::{Table, TableError};
use anyhow::Error;
use bytes::Bytes;

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
        crate::preview2::in_tokio(|| {
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
            })
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

        match self.receiver.try_recv() {
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

#[derive(Debug)]
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

        crate::preview2::in_tokio(|| {
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
                                    Err(e) => {
                                        let _ = result_sender.send(Err(e)).await;
                                        break 'outer;
                                    }
                                }
                            }
                        }

                        // The other side of the channel hung up, the task can exit now
                        None => break 'outer,
                    }
                }
            })
        });

        AsyncWriteStream {
            state: Some(WriteState::Ready),
            sender,
            result_receiver,
        }
    }

    fn send(&mut self, bytes: Bytes) -> anyhow::Result<(usize, StreamState)> {
        use tokio::sync::mpsc::error::TrySendError;

        debug_assert!(matches!(self.state, Some(WriteState::Ready)));

        let len = bytes.len();
        match self.sender.try_send(bytes) {
            Ok(_) => {
                self.state = Some(WriteState::Pending);
                Ok((len, StreamState::Open))
            }
            Err(TrySendError::Full(_)) => Ok((0, StreamState::Open)),
            Err(TrySendError::Closed(_)) => unreachable!("task shouldn't die while not closed"),
        }
    }
}

#[async_trait::async_trait]
impl HostOutputStream for AsyncWriteStream {
    fn write(&mut self, bytes: Bytes) -> Result<(usize, StreamState), anyhow::Error> {
        use tokio::sync::mpsc::error::TryRecvError;

        dbg!(&self.state);
        match self.state {
            Some(WriteState::Ready) => self.send(bytes),
            Some(WriteState::Pending) => match self.result_receiver.try_recv() {
                Ok(Ok(StreamState::Open)) => {
                    self.state = Some(WriteState::Ready);
                    self.send(bytes)
                }

                Ok(Ok(StreamState::Closed)) => {
                    self.state = None;
                    Ok((0, StreamState::Closed))
                }

                Ok(Err(e)) => {
                    self.state = None;
                    Err(e.into())
                }

                Err(TryRecvError::Empty) => {
                    self.state = Some(WriteState::Pending);
                    Ok((0, StreamState::Open))
                }

                Err(TryRecvError::Disconnected) => {
                    unreachable!("task shouldn't die while pending")
                }
            },
            Some(WriteState::Err(_)) => {
                // Move the error payload out of self.state, because errors are not Copy,
                // and set self.state to None, because the stream is now closed.
                if let Some(WriteState::Err(e)) = self.state.take() {
                    Err(e.into())
                } else {
                    unreachable!("self.state shown to be Some(Err(e)) in match clause")
                }
            }

            None => Ok((0, StreamState::Closed)),
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

#[cfg(test)]
mod test {
    use super::*;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
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
    async fn empty_read_stream() {
        let mut reader = AsyncReadStream::new(tokio::io::empty());
        let (bs, state) = reader.read(10).unwrap();
        assert!(bs.is_empty());

        // In a multi-threaded context, the value of state is not deterministic -- the spawned
        // reader task may run on a different thread.
        match state {
            // The reader task ran before we tried to read, and noticed that the input was empty.
            StreamState::Closed => {}

            // The reader task hasn't run yet. Call `ready` to await and fill the buffer.
            StreamState::Open => {
                tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
                    .await
                    .expect("the reader should be ready instantly")
                    .expect("ready is ok");
                let (bs, state) = reader.read(0).unwrap();
                assert!(bs.is_empty());
                assert_eq!(state, StreamState::Closed);
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn infinite_read_stream() {
        let mut reader = AsyncReadStream::new(tokio::io::repeat(0));

        let (bs, state) = reader.read(10).unwrap();
        assert_eq!(state, StreamState::Open);
        if bs.is_empty() {
            // Reader task hasn't run yet. Call `ready` to await and fill the buffer.
            tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
                .await
                .expect("the reader should be ready instantly")
                .expect("ready is ok");
            // Now a read should succeed
            let (bs, state) = reader.read(10).unwrap();
            assert_eq!(bs.len(), 10);
            assert_eq!(state, StreamState::Open);
        } else {
            assert_eq!(bs.len(), 10);
        }

        // Subsequent reads should succeed
        let (bs, state) = reader.read(10).unwrap();
        assert_eq!(state, StreamState::Open);
        assert_eq!(bs.len(), 10);

        // Even 0-length reads should succeed and show its open
        let (bs, state) = reader.read(0).unwrap();
        assert_eq!(state, StreamState::Open);
        assert_eq!(bs.len(), 0);
    }

    fn simplex(
        size: usize,
    ) -> (
        impl AsyncRead + Send + Sync + 'static,
        impl AsyncWrite + Send + Sync + 'static,
    ) {
        let (a, b) = tokio::io::duplex(size);
        let (_read_half, write_half) = tokio::io::split(a);
        let (read_half, _write_half) = tokio::io::split(b);
        (read_half, write_half)
    }

    async fn finite_async_reader(contents: &[u8]) -> impl AsyncRead + Send + Sync + 'static {
        let (r, mut w) = simplex(contents.len());
        w.write_all(contents).await.unwrap();
        r
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn finite_read_stream() {
        let mut reader = AsyncReadStream::new(finite_async_reader(&[1; 123]).await);

        let (bs, state) = reader.read(123).unwrap();
        assert_eq!(state, StreamState::Open);
        if bs.is_empty() {
            // Reader task hasn't run yet. Call `ready` to await and fill the buffer.
            tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
                .await
                .expect("the reader should be ready instantly")
                .expect("ready is ok");
            // Now a read should succeed
            let (bs, state) = reader.read(123).unwrap();
            assert_eq!(bs.len(), 123);
            assert_eq!(state, StreamState::Open);
        } else {
            assert_eq!(bs.len(), 123);
        }

        // The AsyncRead's should be empty now, but we have a race where the reader task hasn't
        // yet send that to the AsyncReadStream.
        let (bs, state) = reader.read(0).unwrap();
        assert!(bs.is_empty());
        match state {
            StreamState::Closed => {} // Correct!
            StreamState::Open => {
                // Need to await to give this side time to catch up
                tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
                    .await
                    .expect("the reader should be ready instantly")
                    .expect("ready is ok");
                // Now a read should show closed
                let (bs, state) = reader.read(0).unwrap();
                assert_eq!(bs.len(), 0);
                assert_eq!(state, StreamState::Closed);
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    // Test that you can write items into the stream, and they get read out in the order they were
    // written, with the proper indications of readiness for reading:
    async fn multiple_chunks_read_stream() {
        let (r, mut w) = simplex(1024);
        let mut reader = AsyncReadStream::new(r);

        w.write_all(&[123]).await.unwrap();

        let (bs, state) = reader.read(1).unwrap();
        assert_eq!(state, StreamState::Open);
        if bs.is_empty() {
            // Reader task hasn't run yet. Call `ready` to await and fill the buffer.
            tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
                .await
                .expect("the reader should be ready instantly")
                .expect("ready is ok");
            // Now a read should succeed
            let (bs, state) = reader.read(1).unwrap();
            assert_eq!(*bs, [123u8]);
            assert_eq!(state, StreamState::Open);
        } else {
            assert_eq!(*bs, [123u8]);
        }

        // The stream should be empty and open now:
        let (bs, state) = reader.read(1).unwrap();
        assert!(bs.is_empty());
        assert_eq!(state, StreamState::Open);

        // We can wait on readiness and it will time out:
        tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
            .await
            .err()
            .expect("the reader should time out");

        // Still open and empty:
        let (bs, state) = reader.read(1).unwrap();
        assert!(bs.is_empty());
        assert_eq!(state, StreamState::Open);

        // Put something else in the stream:
        w.write_all(&[45]).await.unwrap();

        // Wait readiness (yes we could possibly win the race and read it out faster, leaving that
        // out of the test for simplicity)
        tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
            .await
            .expect("the reader should be ready instantly")
            .expect("the ready is ok");

        // read the something else back out:
        let (bs, state) = reader.read(1).unwrap();
        assert_eq!(*bs, [45u8]);
        assert_eq!(state, StreamState::Open);

        // nothing else in there:
        let (bs, state) = reader.read(1).unwrap();
        assert!(bs.is_empty());
        assert_eq!(state, StreamState::Open);

        // We can wait on readiness and it will time out:
        tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
            .await
            .err()
            .expect("the reader should time out");

        // nothing else in there:
        let (bs, state) = reader.read(1).unwrap();
        assert!(bs.is_empty());
        assert_eq!(state, StreamState::Open);

        // Now close the pipe:
        drop(w);

        // Wait readiness (yes we could possibly win the race and read it out faster, leaving that
        // out of the test for simplicity)
        tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
            .await
            .expect("the reader should be ready instantly")
            .expect("the ready is ok");

        // empty and now closed:
        let (bs, state) = reader.read(1).unwrap();
        assert!(bs.is_empty());
        assert_eq!(state, StreamState::Closed);
    }

    #[tokio::test(flavor = "multi_thread")]
    // At the moment we are restricting AsyncReadStream from buffering more than 4k. This isn't a
    // suitable design for all applications, and we will probably make a knob or change the
    // behavior at some point, but this test shows the behavior as it is implemented:
    async fn backpressure_read_stream() {
        let (r, mut w) = simplex(16 * 1024); // Make sure this buffer isnt a bottleneck
        let mut reader = AsyncReadStream::new(r);

        let writer_task = tokio::task::spawn(async move {
            // Write twice as much as we can buffer up in an AsyncReadStream:
            w.write_all(&[123; 8192]).await.unwrap();
            w
        });

        tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
            .await
            .expect("the reader should be ready instantly")
            .expect("ready is ok");

        // Now we expect the reader task has sent 4k from the stream to the reader.
        // Try to read out one bigger than the buffer available:
        let (bs, state) = reader.read(4097).unwrap();
        assert_eq!(bs.len(), 4096);
        assert_eq!(state, StreamState::Open);

        // Allow the crank to turn more:
        tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
            .await
            .expect("the reader should be ready instantly")
            .expect("ready is ok");

        // Again we expect the reader task has sent 4k from the stream to the reader.
        // Try to read out one bigger than the buffer available:
        let (bs, state) = reader.read(4097).unwrap();
        assert_eq!(bs.len(), 4096);
        assert_eq!(state, StreamState::Open);

        // The writer task is now finished - join with it:
        let w = tokio::time::timeout(std::time::Duration::from_millis(10), writer_task)
            .await
            .expect("the join should be ready instantly");
        // And close the pipe:
        drop(w);

        // Allow the crank to turn more:
        tokio::time::timeout(std::time::Duration::from_millis(10), reader.ready())
            .await
            .expect("the reader should be ready instantly")
            .expect("ready is ok");

        // Now we expect the reader to be empty, and the stream closed:
        let (bs, state) = reader.read(4097).unwrap();
        assert_eq!(bs.len(), 0);
        assert_eq!(state, StreamState::Closed);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sink_write_stream() {
        let mut writer = AsyncWriteStream::new(tokio::io::sink());
        let chunk = Bytes::from_static(&[0; 1024]);

        // I can write whatever:
        let (len, state) = writer.write(chunk.clone()).unwrap();
        assert_eq!(len, chunk.len());
        assert_eq!(state, StreamState::Open);

        // But I expect this to block additional writes:
        let (len, state) = writer.write(chunk.clone()).unwrap();
        assert_eq!(len, 0);
        assert_eq!(state, StreamState::Open);

        tokio::time::timeout(std::time::Duration::from_millis(10), writer.ready())
            .await
            .expect("the writer should be ready instantly")
            .expect("ready is ok");

        // Now additional writes will work:
        let (len, state) = writer.write(chunk.clone()).unwrap();
        assert_eq!(len, chunk.len());
        assert_eq!(state, StreamState::Open);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn closed_write_stream() {
        let (reader, writer) = simplex(1024);
        drop(reader);
        let mut writer = AsyncWriteStream::new(writer);

        // Without checking write readiness, perform a nonblocking write: this should succeed
        // because we will buffer up the write.
        let chunk = Bytes::from_static(&[0; 1]);
        let (len, state) = writer.write(chunk.clone()).unwrap();

        assert_eq!(len, chunk.len());
        assert_eq!(state, StreamState::Open);

        // Check write readiness:
        tokio::time::timeout(std::time::Duration::from_millis(10), writer.ready())
            .await
            .expect("the writer should be ready instantly")
            .expect("ready is ok");

        // When we drop the simplex reader, that causes the simplex writer to return BrokenPipe on
        // its write. Now that the buffering crank has turned, our next write will give BrokenPipe.
        let err = writer.write(chunk.clone()).err().unwrap();
        assert_eq!(
            err.downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::BrokenPipe
        );

        // Now that we got the error out of the writer, it should be closed - subsequent writes
        // will not work
        let (len, state) = writer.write(chunk.clone()).unwrap();
        assert_eq!(len, 0);
        assert_eq!(state, StreamState::Closed);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn multiple_chunks_write_stream() {
        use std::ops::Deref;

        let (mut reader, writer) = simplex(1024);
        let mut writer = AsyncWriteStream::new(writer);

        // Write a chunk:
        let chunk = Bytes::from_static(&[123; 1]);
        let (len, state) = writer.write(chunk.clone()).unwrap();

        assert_eq!(len, chunk.len());
        assert_eq!(state, StreamState::Open);

        // After the write, still ready for more writing:
        tokio::time::timeout(std::time::Duration::from_millis(10), writer.ready())
            .await
            .expect("the writer should be ready instantly")
            .expect("ready is ok");

        let mut read_buf = vec![0; chunk.len()];
        let read_len = reader.read_exact(&mut read_buf).await.unwrap();
        assert_eq!(read_len, chunk.len());
        assert_eq!(read_buf.as_slice(), chunk.deref());

        // Write a second, different chunk:
        let chunk2 = Bytes::from_static(&[45; 1]);
        let (len, state) = writer.write(chunk2.clone()).unwrap();
        assert_eq!(len, chunk2.len());
        assert_eq!(state, StreamState::Open);

        // After the write, still ready for more writing:
        tokio::time::timeout(std::time::Duration::from_millis(10), writer.ready())
            .await
            .expect("the writer should be ready instantly")
            .expect("ready is ok");

        let mut read2_buf = vec![0; chunk2.len()];
        let read2_len = reader.read_exact(&mut read2_buf).await.unwrap();
        assert_eq!(read2_len, chunk2.len());
        assert_eq!(read2_buf.as_slice(), chunk2.deref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn backpressure_write_stream() {
        // Stream can buffer up to 1k, plus one write chunk, before not
        // accepting more input:
        let (mut reader, writer) = simplex(1024);
        let mut writer = AsyncWriteStream::new(writer);

        // Write enough to fill the simplex buffer:
        let chunk = Bytes::from_static(&[0; 1024]);
        let (len, state) = writer.write(chunk.clone()).unwrap();

        assert_eq!(len, chunk.len());
        assert_eq!(state, StreamState::Open);

        // turn the crank and it should be ready for writing again:
        tokio::time::timeout(std::time::Duration::from_millis(10), writer.ready())
            .await
            .expect("the writer should be ready instantly")
            .expect("ready is ok");

        // Now fill the buffer between here and the writer task:
        let (len, state) = writer.write(chunk.clone()).unwrap();
        assert_eq!(len, chunk.len());
        assert_eq!(state, StreamState::Open);

        // Try shoving even more down there, and it shouldnt accept more input:
        let (len, state) = writer.write(chunk.clone()).unwrap();
        assert_eq!(len, 0);
        assert_eq!(state, StreamState::Open);

        // turn the crank and it should Not become ready for writing until we read something out.
        tokio::time::timeout(std::time::Duration::from_millis(10), writer.ready())
            .await
            .err()
            .expect("the writer should be not become ready");

        // Still not ready from the .write interface either:
        let (len, state) = writer.write(chunk.clone()).unwrap();
        assert_eq!(len, 0);
        assert_eq!(state, StreamState::Open);

        // There is 2k in the buffer. I should be able to read all of it out:
        let mut buf = [0; 2048];
        reader.read_exact(&mut buf).await.unwrap();

        // and no more:
        tokio::time::timeout(std::time::Duration::from_millis(10), reader.read(&mut buf))
            .await
            .err()
            .expect("nothing more buffered in the system");

        // Now the backpressure should be cleared, and an additional write should be accepted.

        // immediately ready for writing:
        tokio::time::timeout(std::time::Duration::from_millis(10), writer.ready())
            .await
            .expect("the writer should be ready instantly")
            .expect("ready is ok");

        // and the write succeeds:
        let (len, state) = writer.write(chunk.clone()).unwrap();
        assert_eq!(len, chunk.len());
        assert_eq!(state, StreamState::Open);
    }
}

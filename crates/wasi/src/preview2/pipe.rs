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

#[derive(Debug)]
pub struct MemoryInputPipe {
    buffer: std::io::Cursor<Bytes>,
}

impl MemoryInputPipe {
    pub fn new(bytes: Bytes) -> Self {
        Self {
            buffer: std::io::Cursor::new(bytes),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.get_ref().len() as u64 == self.buffer.position()
    }
}

#[async_trait::async_trait]
impl HostInputStream for MemoryInputPipe {
    fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), Error> {
        if self.is_empty() {
            return Ok((Bytes::new(), StreamState::Closed));
        }

        let mut dest = bytes::BytesMut::zeroed(size);
        let nbytes = std::io::Read::read(&mut self.buffer, dest.as_mut())?;
        dest.truncate(nbytes);

        let state = if self.is_empty() {
            StreamState::Closed
        } else {
            StreamState::Open
        };
        Ok((dest.freeze(), state))
    }
    async fn ready(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MemoryOutputPipe {
    buffer: std::sync::Arc<std::sync::Mutex<bytes::BytesMut>>,
}

impl MemoryOutputPipe {
    pub fn new() -> Self {
        MemoryOutputPipe {
            buffer: std::sync::Arc::new(std::sync::Mutex::new(bytes::BytesMut::new())),
        }
    }

    pub fn contents(&self) -> bytes::Bytes {
        self.buffer.lock().unwrap().clone().freeze()
    }

    pub fn try_into_inner(self) -> Option<bytes::BytesMut> {
        std::sync::Arc::into_inner(self.buffer).map(|m| m.into_inner().unwrap())
    }
}

#[async_trait::async_trait]
impl HostOutputStream for MemoryOutputPipe {
    fn write(&mut self, bytes: Bytes) -> Result<(usize, StreamState), anyhow::Error> {
        let mut buf = self.buffer.lock().unwrap();
        buf.extend_from_slice(bytes.as_ref());
        Ok((bytes.len(), StreamState::Open))
    }

    async fn ready(&mut self) -> Result<(), Error> {
        // This stream is always ready for writing.
        Ok(())
    }
}

/// TODO
pub fn pipe(size: usize) -> (AsyncReadStream, AsyncWriteStream) {
    let (a, b) = tokio::io::duplex(size);
    let (_read_half, write_half) = tokio::io::split(a);
    let (read_half, _write_half) = tokio::io::split(b);
    (
        AsyncReadStream::new(read_half),
        AsyncWriteStream::new(write_half),
    )
}

/// Provides a [`HostInputStream`] impl from a [`tokio::io::AsyncRead`] impl
pub struct AsyncReadStream {
    state: StreamState,
    buffer: Option<Result<Bytes, std::io::Error>>,
    receiver: tokio::sync::mpsc::Receiver<Result<(Bytes, StreamState), std::io::Error>>,
    pub(crate) join_handle: tokio::task::JoinHandle<()>,
}

impl AsyncReadStream {
    /// Create a [`AsyncReadStream`]. In order to use the [`HostInputStream`] impl
    /// provided by this struct, the argument must impl [`tokio::io::AsyncRead`].
    pub fn new<T: tokio::io::AsyncRead + Send + Sync + Unpin + 'static>(mut reader: T) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let join_handle = crate::preview2::spawn(async move {
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
            join_handle,
        }
    }
}

impl Drop for AsyncReadStream {
    fn drop(&mut self) {
        self.join_handle.abort()
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
    join_handle: tokio::task::JoinHandle<()>,
}

impl AsyncWriteStream {
    /// Create a [`AsyncWriteStream`]. In order to use the [`HostOutputStream`] impl
    /// provided by this struct, the argument must impl [`tokio::io::AsyncWrite`].
    pub fn new<T: tokio::io::AsyncWrite + Send + Sync + Unpin + 'static>(mut writer: T) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Bytes>(1);
        let (result_sender, result_receiver) = tokio::sync::mpsc::channel(1);

        let join_handle = crate::preview2::spawn(async move {
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
        });

        AsyncWriteStream {
            state: Some(WriteState::Ready),
            sender,
            result_receiver,
            join_handle,
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
            Err(TrySendError::Full(_)) => {
                unreachable!("task shouldnt be full when writestate is ready")
            }
            Err(TrySendError::Closed(_)) => unreachable!("task shouldn't die while not closed"),
        }
    }
}

impl Drop for AsyncWriteStream {
    fn drop(&mut self) {
        self.join_handle.abort()
    }
}

#[async_trait::async_trait]
impl HostOutputStream for AsyncWriteStream {
    fn write(&mut self, bytes: Bytes) -> Result<(usize, StreamState), anyhow::Error> {
        use tokio::sync::mpsc::error::TryRecvError;

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

/// An output stream that consumes all input written to it, and is always ready.
pub struct SinkOutputStream;

#[async_trait::async_trait]
impl HostOutputStream for SinkOutputStream {
    fn write(&mut self, buf: Bytes) -> Result<(usize, StreamState), Error> {
        Ok((buf.len(), StreamState::Open))
    }

    async fn ready(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

/// A stream that is ready immediately, but will always report that it's closed.
pub struct ClosedInputStream;

#[async_trait::async_trait]
impl HostInputStream for ClosedInputStream {
    fn read(&mut self, _size: usize) -> Result<(Bytes, StreamState), Error> {
        Ok((Bytes::new(), StreamState::Closed))
    }

    async fn ready(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

/// An output stream that is always closed.
pub struct ClosedOutputStream;

#[async_trait::async_trait]
impl HostOutputStream for ClosedOutputStream {
    fn write(&mut self, _: Bytes) -> Result<(usize, StreamState), Error> {
        Ok((0, StreamState::Closed))
    }

    async fn ready(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    // 10ms was enough for every CI platform except linux riscv64:
    const REASONABLE_DURATION: std::time::Duration = std::time::Duration::from_millis(100);

    pub fn simplex(size: usize) -> (impl AsyncRead, impl AsyncWrite) {
        let (a, b) = tokio::io::duplex(size);
        let (_read_half, write_half) = tokio::io::split(a);
        let (read_half, _write_half) = tokio::io::split(b);
        (read_half, write_half)
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
                tokio::time::timeout(REASONABLE_DURATION, reader.ready())
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
            tokio::time::timeout(REASONABLE_DURATION, reader.ready())
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
            tokio::time::timeout(REASONABLE_DURATION, reader.ready())
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
                tokio::time::timeout(REASONABLE_DURATION, reader.ready())
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
            tokio::time::timeout(REASONABLE_DURATION, reader.ready())
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
        tokio::time::timeout(REASONABLE_DURATION, reader.ready())
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
        tokio::time::timeout(REASONABLE_DURATION, reader.ready())
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
        tokio::time::timeout(REASONABLE_DURATION, reader.ready())
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
        tokio::time::timeout(REASONABLE_DURATION, reader.ready())
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

        tokio::time::timeout(REASONABLE_DURATION, reader.ready())
            .await
            .expect("the reader should be ready instantly")
            .expect("ready is ok");

        // Now we expect the reader task has sent 4k from the stream to the reader.
        // Try to read out one bigger than the buffer available:
        let (bs, state) = reader.read(4097).unwrap();
        assert_eq!(bs.len(), 4096);
        assert_eq!(state, StreamState::Open);

        // Allow the crank to turn more:
        tokio::time::timeout(REASONABLE_DURATION, reader.ready())
            .await
            .expect("the reader should be ready instantly")
            .expect("ready is ok");

        // Again we expect the reader task has sent 4k from the stream to the reader.
        // Try to read out one bigger than the buffer available:
        let (bs, state) = reader.read(4097).unwrap();
        assert_eq!(bs.len(), 4096);
        assert_eq!(state, StreamState::Open);

        // The writer task is now finished - join with it:
        let w = tokio::time::timeout(REASONABLE_DURATION, writer_task)
            .await
            .expect("the join should be ready instantly");
        // And close the pipe:
        drop(w);

        // Allow the crank to turn more:
        tokio::time::timeout(REASONABLE_DURATION, reader.ready())
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

        // It is possible for subsequent writes to be refused, but it is nondeterminstic because
        // the worker task consuming them is in another thread:
        let (len, state) = writer.write(chunk.clone()).unwrap();
        assert_eq!(state, StreamState::Open);
        if !(len == 0 || len == chunk.len()) {
            unreachable!()
        }

        tokio::time::timeout(REASONABLE_DURATION, writer.ready())
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
        tokio::time::timeout(REASONABLE_DURATION, writer.ready())
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
        tokio::time::timeout(REASONABLE_DURATION, writer.ready())
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
        tokio::time::timeout(REASONABLE_DURATION, writer.ready())
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
        tokio::time::timeout(REASONABLE_DURATION, writer.ready())
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
        tokio::time::timeout(REASONABLE_DURATION, writer.ready())
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
        tokio::time::timeout(REASONABLE_DURATION, reader.read(&mut buf))
            .await
            .err()
            .expect("nothing more buffered in the system");

        // Now the backpressure should be cleared, and an additional write should be accepted.

        // immediately ready for writing:
        tokio::time::timeout(REASONABLE_DURATION, writer.ready())
            .await
            .expect("the writer should be ready instantly")
            .expect("ready is ok");

        // and the write succeeds:
        let (len, state) = writer.write(chunk.clone()).unwrap();
        assert_eq!(len, chunk.len());
        assert_eq!(state, StreamState::Open);
    }
}

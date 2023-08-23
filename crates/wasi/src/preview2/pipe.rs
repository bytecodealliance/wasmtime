//! Virtual pipes.
//!
//! These types provide easy implementations of `WasiFile` that mimic much of the behavior of Unix
//! pipes. These are particularly helpful for redirecting WASI stdio handles to destinations other
//! than OS files.
//!
//! Some convenience constructors are included for common backing types like `Vec<u8>` and `String`,
//! but the virtual pipes can be instantiated with any `Read` or `Write` type.
//!
use crate::preview2::{
    FlushResult, HostInputStream, HostOutputStream, StreamState, WriteReadiness,
};
use anyhow::Error;
use bytes::Bytes;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

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
    fn write(&mut self, bytes: Bytes) -> Result<Option<WriteReadiness>, anyhow::Error> {
        let mut buf = self.buffer.lock().unwrap();
        buf.extend_from_slice(bytes.as_ref());
        // Always ready for writing
        Ok(Some(WriteReadiness::Ready(64 * 1024)))
    }
    fn flush(&mut self) -> Result<Option<FlushResult>, anyhow::Error> {
        // This stream is always flushed
        Ok(Some(FlushResult::Done))
    }
    async fn write_ready(&mut self) -> Result<WriteReadiness, Error> {
        // This stream is always ready for writing.
        Ok(WriteReadiness::Ready(64 * 1024))
    }
    async fn flush_ready(&mut self) -> Result<FlushResult, Error> {
        // This stream is always flushed
        Ok(FlushResult::Done)
    }
}

/// FIXME: this needs docs
pub fn pipe(size: usize) -> (AsyncReadStream, AsyncWriteStream) {
    let (a, b) = tokio::io::duplex(size);
    let (_read_half, write_half) = tokio::io::split(a);
    let (read_half, _write_half) = tokio::io::split(b);
    (
        AsyncReadStream::new(read_half),
        AsyncWriteStream::new(size, write_half),
    )
}

/// Provides a [`HostInputStream`] impl from a [`tokio::io::AsyncRead`] impl
pub struct AsyncReadStream {
    state: StreamState,
    buffer: Option<Result<Bytes, std::io::Error>>,
    receiver: mpsc::Receiver<Result<(Bytes, StreamState), std::io::Error>>,
    pub(crate) join_handle: tokio::task::JoinHandle<()>,
}

impl AsyncReadStream {
    /// Create a [`AsyncReadStream`]. In order to use the [`HostInputStream`] impl
    /// provided by this struct, the argument must impl [`tokio::io::AsyncRead`].
    pub fn new<T: tokio::io::AsyncRead + Send + Sync + Unpin + 'static>(mut reader: T) -> Self {
        let (sender, receiver) = mpsc::channel(1);
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
        use mpsc::error::TryRecvError;

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
enum WriteMessage {
    Write(Bytes),
    Flush,
}

#[derive(Debug)]
enum FlushState {
    Enqueued,
    InProgress,
    Done,
}

#[derive(Debug)]
struct WorkerState {
    alive: bool,
    items: std::collections::VecDeque<WriteMessage>,
    write_budget: usize,
    flush_state: FlushState,
}

#[derive(Clone)]
struct Worker {
    state: Arc<Mutex<WorkerState>>,
    new_work: Arc<tokio::sync::Notify>,
    write_ready_changed: Arc<tokio::sync::Notify>,
    flush_result_changed: Arc<tokio::sync::Notify>,
}

impl Worker {
    fn new(write_budget: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(WorkerState {
                alive: true,
                items: std::collections::VecDeque::new(),
                write_budget,
                flush_state: FlushState::Done,
            })),
            new_work: Arc::new(tokio::sync::Notify::new()),
            write_ready_changed: Arc::new(tokio::sync::Notify::new()),
            flush_result_changed: Arc::new(tokio::sync::Notify::new()),
        }
    }
    fn check_write(&self) -> Option<WriteReadiness> {
        let state = self.state.lock().unwrap();
        if state.alive {
            if state.write_budget > 0 {
                Some(WriteReadiness::Ready(state.write_budget))
            } else {
                None
            }
        } else {
            Some(WriteReadiness::Closed)
        }
    }
    fn check_flush(&self) -> Option<FlushResult> {
        let state = self.state.lock().unwrap();
        if state.alive {
            if matches!(state.flush_state, FlushState::Done) {
                Some(FlushResult::Done)
            } else {
                None
            }
        } else {
            Some(FlushResult::Closed)
        }
    }
    fn push_to_worker(&self, msg: WriteMessage) -> anyhow::Result<StreamState> {
        let mut state = self.state.lock().unwrap();
        if !state.alive {
            return Ok(StreamState::Closed);
        }
        match msg {
            WriteMessage::Write(bytes) => match state.write_budget.checked_sub(bytes.len()) {
                Some(remaining_budget) => {
                    state.write_budget = remaining_budget;
                    state.items.push_back(WriteMessage::Write(bytes));
                }
                None => return Err(anyhow::anyhow!("write exceeded budget")),
            },
            WriteMessage::Flush => {
                match state.flush_state {
                    FlushState::Enqueued => {
                        // Only retain most recent flush:
                        state
                            .items
                            .retain(|msg| matches!(msg, WriteMessage::Write { .. }));
                    }
                    FlushState::InProgress | FlushState::Done => {
                        // Stop caring about in progress flush, if there is one.
                        state.flush_state = FlushState::Enqueued
                    }
                }
                state.items.push_back(WriteMessage::Flush);
            }
        }
        drop(state);
        self.new_work.notify_waiters();
        Ok(StreamState::Open)
    }

    async fn work<T: tokio::io::AsyncWrite + Send + Sync + Unpin + 'static>(&self, mut writer: T) {
        use tokio::io::AsyncWriteExt;
        loop {
            let notified = self.new_work.notified();
            while let Some(work) = self.pop_in_worker() {
                tracing::debug!("worker popped: {work:?}");
                match work {
                    WriteMessage::Write(mut bytes) => {
                        let len = bytes.len();
                        match writer.write_all_buf(&mut bytes).await {
                            Err(_) => {
                                self.die_in_worker();
                                return;
                            }
                            Ok(_) => {
                                self.state.lock().unwrap().write_budget += len;
                                self.write_ready_changed.notify_waiters();
                            }
                        }
                    }
                    WriteMessage::Flush => match writer.flush().await {
                        Ok(()) => self.finish_flush_in_worker(),
                        Err(_) => {
                            self.die_in_worker();
                            return;
                        }
                    },
                }
            }
            notified.await;
        }
    }

    fn pop_in_worker(&self) -> Option<WriteMessage> {
        let mut state = self.state.lock().unwrap();
        let item = state.items.pop_front();
        match &item {
            Some(WriteMessage::Write(_)) => drop(state),
            Some(WriteMessage::Flush) => state.flush_state = FlushState::InProgress,
            _ => {}
        }
        item
    }
    fn finish_flush_in_worker(&self) {
        tracing::debug!("finish flush in worker");
        let mut state = self.state.lock().unwrap();
        match state.flush_state {
            FlushState::InProgress => {
                state.flush_state = FlushState::Done;
                drop(state);
                self.flush_result_changed.notify_waiters();
            }
            _ => {}
        }
    }
    fn die_in_worker(&self) {
        tracing::debug!("dying in worker");
        self.state.lock().unwrap().alive = false;
        self.write_ready_changed.notify_waiters();
        self.flush_result_changed.notify_waiters();
    }
}

/// Provides a [`HostOutputStream`] impl from a [`tokio::io::AsyncWrite`] impl
pub struct AsyncWriteStream {
    worker: Worker,
    join_handle: tokio::task::JoinHandle<()>,
}

impl AsyncWriteStream {
    /// Create a [`AsyncWriteStream`]. In order to use the [`HostOutputStream`] impl
    /// provided by this struct, the argument must impl [`tokio::io::AsyncWrite`].
    pub fn new<T: tokio::io::AsyncWrite + Send + Sync + Unpin + 'static>(
        write_budget: usize,
        writer: T,
    ) -> Self {
        let worker = Worker::new(write_budget);

        let w = worker.clone();
        let join_handle = crate::preview2::spawn(async move { w.work(writer).await });

        AsyncWriteStream {
            worker,
            join_handle,
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
    fn write(&mut self, bytes: Bytes) -> Result<Option<WriteReadiness>, anyhow::Error> {
        let s = self.worker.push_to_worker(WriteMessage::Write(bytes))?;
        if matches!(s, StreamState::Closed) {
            return Ok(Some(WriteReadiness::Closed));
        }
        Ok(self.worker.check_write())
    }

    async fn write_ready(&mut self) -> Result<WriteReadiness, Error> {
        let notified = self.worker.write_ready_changed.notified();
        if let Some(readiness) = self.worker.check_write() {
            return Ok(readiness);
        }
        notified.await;
        self.worker.check_write().ok_or_else(|| {
            unreachable!(
                "should be impossible: write readiness changed but check_write was still None"
            )
        })
    }

    fn flush(&mut self) -> Result<Option<FlushResult>, anyhow::Error> {
        let s = self.worker.push_to_worker(WriteMessage::Flush)?;
        if matches!(s, StreamState::Closed) {
            return Ok(Some(FlushResult::Closed));
        }
        Ok(self.worker.check_flush())
    }

    async fn flush_ready(&mut self) -> Result<FlushResult, Error> {
        let notified = self.worker.flush_result_changed.notified();
        if let Some(readiness) = self.worker.check_flush() {
            return Ok(readiness);
        }
        notified.await;
        self.worker.check_flush().ok_or_else(|| {
            unreachable!(
                "should be impossible: flush result changed but check_flush was still None"
            )
        })
    }
}

/// An output stream that consumes all input written to it, and is always ready.
pub struct SinkOutputStream;

#[async_trait::async_trait]
impl HostOutputStream for SinkOutputStream {
    fn write(&mut self, _buf: Bytes) -> Result<Option<WriteReadiness>, Error> {
        Ok(Some(WriteReadiness::Ready(64 * 1024))) // made up constant
    }
    fn flush(&mut self) -> Result<Option<FlushResult>, anyhow::Error> {
        // This stream is always flushed
        Ok(Some(FlushResult::Done))
    }

    async fn write_ready(&mut self) -> Result<WriteReadiness, Error> {
        // This stream is always ready for writing.
        Ok(WriteReadiness::Ready(64 * 1024))
    }
    async fn flush_ready(&mut self) -> Result<FlushResult, Error> {
        // This stream is always flushed
        Ok(FlushResult::Done)
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
    fn write(&mut self, _: Bytes) -> Result<Option<WriteReadiness>, Error> {
        Ok(Some(WriteReadiness::Closed))
    }
    fn flush(&mut self) -> Result<Option<FlushResult>, anyhow::Error> {
        Ok(Some(FlushResult::Closed))
    }

    async fn write_ready(&mut self) -> Result<WriteReadiness, Error> {
        Ok(WriteReadiness::Closed)
    }
    async fn flush_ready(&mut self) -> Result<FlushResult, Error> {
        Ok(FlushResult::Closed)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    #[cfg(target_arch = "riscv64")]
    const TEST_ITERATIONS: usize = 10;

    #[cfg(target_arch = "riscv64")]
    const REASONABLE_DURATION: std::time::Duration = std::time::Duration::from_millis(100);

    #[cfg(not(target_arch = "riscv64"))]
    const TEST_ITERATIONS: usize = 100;

    #[cfg(not(target_arch = "riscv64"))]
    const REASONABLE_DURATION: std::time::Duration = std::time::Duration::from_millis(10);

    async fn resolves_immediately<F, O>(fut: F) -> O
    where
        F: futures::Future<Output = O>,
    {
        tokio::time::timeout(REASONABLE_DURATION, fut)
            .await
            .expect("operation timed out")
    }

    async fn never_resolves<F: futures::Future>(fut: F) {
        tokio::time::timeout(REASONABLE_DURATION, fut)
            .await
            .err()
            .expect("operation should time out");
    }

    pub fn simplex(size: usize) -> (impl AsyncRead, impl AsyncWrite) {
        let (a, b) = tokio::io::duplex(size);
        let (_read_half, write_half) = tokio::io::split(a);
        let (read_half, _write_half) = tokio::io::split(b);
        (read_half, write_half)
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
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
                resolves_immediately(reader.ready())
                    .await
                    .expect("ready is ok");
                let (bs, state) = reader.read(0).unwrap();
                assert!(bs.is_empty());
                assert_eq!(state, StreamState::Closed);
            }
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn infinite_read_stream() {
        let mut reader = AsyncReadStream::new(tokio::io::repeat(0));

        let (bs, state) = reader.read(10).unwrap();
        assert_eq!(state, StreamState::Open);
        if bs.is_empty() {
            // Reader task hasn't run yet. Call `ready` to await and fill the buffer.
            resolves_immediately(reader.ready())
                .await
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

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn finite_read_stream() {
        let mut reader = AsyncReadStream::new(finite_async_reader(&[1; 123]).await);

        let (bs, state) = reader.read(123).unwrap();
        assert_eq!(state, StreamState::Open);
        if bs.is_empty() {
            // Reader task hasn't run yet. Call `ready` to await and fill the buffer.
            resolves_immediately(reader.ready())
                .await
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
                resolves_immediately(reader.ready())
                    .await
                    .expect("ready is ok");
                // Now a read should show closed
                let (bs, state) = reader.read(0).unwrap();
                assert_eq!(bs.len(), 0);
                assert_eq!(state, StreamState::Closed);
            }
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
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
            resolves_immediately(reader.ready())
                .await
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
        never_resolves(reader.ready()).await;

        // Still open and empty:
        let (bs, state) = reader.read(1).unwrap();
        assert!(bs.is_empty());
        assert_eq!(state, StreamState::Open);

        // Put something else in the stream:
        w.write_all(&[45]).await.unwrap();

        // Wait readiness (yes we could possibly win the race and read it out faster, leaving that
        // out of the test for simplicity)
        resolves_immediately(reader.ready())
            .await
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
        never_resolves(reader.ready()).await;

        // nothing else in there:
        let (bs, state) = reader.read(1).unwrap();
        assert!(bs.is_empty());
        assert_eq!(state, StreamState::Open);

        // Now close the pipe:
        drop(w);

        // Wait readiness (yes we could possibly win the race and read it out faster, leaving that
        // out of the test for simplicity)
        resolves_immediately(reader.ready())
            .await
            .expect("the ready is ok");

        // empty and now closed:
        let (bs, state) = reader.read(1).unwrap();
        assert!(bs.is_empty());
        assert_eq!(state, StreamState::Closed);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
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

        resolves_immediately(reader.ready())
            .await
            .expect("ready is ok");

        // Now we expect the reader task has sent 4k from the stream to the reader.
        // Try to read out one bigger than the buffer available:
        let (bs, state) = reader.read(4097).unwrap();
        assert_eq!(bs.len(), 4096);
        assert_eq!(state, StreamState::Open);

        // Allow the crank to turn more:
        resolves_immediately(reader.ready())
            .await
            .expect("ready is ok");

        // Again we expect the reader task has sent 4k from the stream to the reader.
        // Try to read out one bigger than the buffer available:
        let (bs, state) = reader.read(4097).unwrap();
        assert_eq!(bs.len(), 4096);
        assert_eq!(state, StreamState::Open);

        // The writer task is now finished - join with it:
        let w = resolves_immediately(writer_task).await;

        // And close the pipe:
        drop(w);

        // Allow the crank to turn more:
        resolves_immediately(reader.ready())
            .await
            .expect("ready is ok");

        // Now we expect the reader to be empty, and the stream closed:
        let (bs, state) = reader.read(4097).unwrap();
        assert_eq!(bs.len(), 0);
        assert_eq!(state, StreamState::Closed);
    }

    #[test_log::test(test_log::test(tokio::test(flavor = "multi_thread")))]
    async fn sink_write_stream() {
        let mut writer = AsyncWriteStream::new(2048, tokio::io::sink());
        let chunk = Bytes::from_static(&[0; 1024]);

        // I can write whatever:
        let readiness = writer.write(chunk.clone()).expect("write does not trap");
        assert!(matches!(readiness, Some(WriteReadiness::Ready(1024))));

        // It is possible for subsequent writes to be refused, but it is nondeterminstic because
        // the worker task consuming them is in another thread:
        let readiness = writer.write(chunk.clone()).expect("write does not trap");
        match readiness {
            Some(WriteReadiness::Ready(budget)) => assert!(budget == 1024 || budget == 2048),
            None => {} // Also ok
            _ => panic!("readiness should not be {readiness:?}"),
        }

        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("write_ready does not trap");
        match permit {
            WriteReadiness::Ready(budget) => assert!(budget == 1024 || budget == 2048),
            _ => panic!("readiness should not be {readiness:?}"),
        }

        // Now additional writes will work:
        let readiness = writer.write(chunk.clone()).expect("write does not trap");
        match readiness {
            Some(WriteReadiness::Ready(budget)) => assert!(budget == 1024 || budget == 2048),
            None => {} // Also ok
            _ => panic!("readiness should not be {readiness:?}"),
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn closed_write_stream() {
        // Run many times because the test is nondeterministic:
        for n in 0..TEST_ITERATIONS {
            closed_write_stream_(n).await
        }
    }
    #[tracing::instrument]
    async fn closed_write_stream_(n: usize) {
        let (reader, writer) = simplex(1);
        let mut writer = AsyncWriteStream::new(1024, writer);

        // Drop the reader to allow the worker to transition to the closed state eventually.
        drop(reader);

        // Write some data to the stream to ensure we have data that cannot be flushed.
        let chunk = Bytes::from_static(&[0; 1]);
        match writer.write(chunk.clone()).expect("write does not trap") {
            Some(WriteReadiness::Ready(1023) | WriteReadiness::Closed) => {}
            a => panic!("invalid write result: {a:?}"),
        }

        // The rest of this test should be valid whether or not we check write readiness:
        if n % 2 == 0 {
            // Check write readiness:
            let permit = resolves_immediately(writer.write_ready())
                .await
                .expect("write ready does not trap");

            match permit {
                WriteReadiness::Ready(1023) => {}
                WriteReadiness::Ready(budget) => panic!("unexpected budget: {budget}"),
                WriteReadiness::Closed => {
                    tracing::debug!("discovered stream closed waiting for write_ready");
                }
            }
        }

        // When we drop the simplex reader, that causes the simplex writer to return BrokenPipe on
        // its write. Now that the buffering crank has turned, our next write will give BrokenPipe.
        let flush_result = writer.flush().expect("flush does not trap");
        match flush_result {
            Some(FlushResult::Closed) => {
                tracing::debug!("discovered stream closed trying to flush");
            }
            Some(FlushResult::Done) => panic!("flush should never succeed"),
            _ => {}
        }

        // Waiting for the flush to complete should always indicate that the channel has been
        // closed.
        let flush_result = resolves_immediately(writer.flush_ready())
            .await
            .expect("flush_ready does not trap");
        match flush_result {
            FlushResult::Closed => {
                tracing::debug!("discovered stream closed after flush_ready");
            }
            _ => {
                tracing::error!("");
                panic!("stream should be reported closed by the end of check_flush")
            }
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn multiple_chunks_write_stream() {
        use std::ops::Deref;

        let (mut reader, writer) = simplex(1024);
        let mut writer = AsyncWriteStream::new(1024, writer);

        // Write a chunk:
        let chunk = Bytes::from_static(&[123; 1]);
        let readiness = writer.write(chunk.clone()).expect("write does not trap");

        match readiness {
            Some(WriteReadiness::Ready(budget)) => assert!(
                budget == 1023 || budget == 1024,
                "unexpected budget: {budget}"
            ),
            _ => panic!("bad state for readiness: {readiness:?}"),
        }

        // After the write, still ready for more writing:
        let readiness = resolves_immediately(writer.write_ready())
            .await
            .expect("write_ready does not trap");
        match readiness {
            WriteReadiness::Ready(budget) => assert!(
                budget == 1024 || budget == 1023,
                "unexpected budget: {budget}"
            ),
            _ => panic!("bad state for readiness: {readiness:?}"),
        }

        let mut read_buf = vec![0; chunk.len()];
        let read_len = reader.read_exact(&mut read_buf).await.unwrap();
        assert_eq!(read_len, chunk.len());
        assert_eq!(read_buf.as_slice(), chunk.deref());

        // Write a second, different chunk:
        let chunk2 = Bytes::from_static(&[45; 1]);
        let readiness = writer.write(chunk2.clone()).expect("write does not trap");

        match readiness {
            Some(WriteReadiness::Ready(budget)) => assert!(
                budget == 1024 || budget == 1023,
                "unexpected budget: {budget}"
            ),
            _ => panic!("bad state for readiness: {readiness:?}"),
        }
        // After the write, still ready for more writing:
        let readiness = resolves_immediately(writer.write_ready())
            .await
            .expect("write_ready does not trap");
        match readiness {
            WriteReadiness::Ready(budget) => assert!(
                budget == 1024 || budget == 1023,
                "unexpected budget: {budget}"
            ),
            _ => panic!("bad state for readiness: {readiness:?}"),
        }

        let mut read2_buf = vec![0; chunk2.len()];
        let read2_len = reader.read_exact(&mut read2_buf).await.unwrap();
        assert_eq!(read2_len, chunk2.len());
        assert_eq!(read2_buf.as_slice(), chunk2.deref());
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn backpressure_write_stream() {
        // The channel can buffer up to 1k, plus another 1k in the stream, before not
        // accepting more input:
        let (mut reader, writer) = simplex(1024);
        let mut writer = AsyncWriteStream::new(1024, writer);

        let chunk = Bytes::from_static(&[0; 1024]);

        // Write enough to fill the simplex buffer:
        match writer.write(chunk.clone()).expect("write does not trap") {
            Some(WriteReadiness::Ready(1024)) => {}

            // If the worker hasn't picked up the write yet, the buffer will be full. We need to
            // wait for the worker to process the buffer before continuing.
            None => {
                match resolves_immediately(writer.write_ready())
                    .await
                    .expect("write_ready does not trap")
                {
                    WriteReadiness::Ready(1024) => {}
                    a => panic!("writer should be ready for more input: {a:?}"),
                }
            }

            Some(a) => panic!("invalid write readiness: {a:?}"),
        }

        // Now fill the buffer between here and the writer task. This should always indicate
        // back-pressure because now both buffers (simplex and worker) are full.
        match writer.write(chunk.clone()).expect("write does not trap") {
            None => {}
            Some(a) => panic!("expected backpressure: {a:?}"),
        }

        // Try shoving even more down there, and it shouldnt accept more input:
        writer
            .write(chunk.clone())
            .err()
            .expect("unpermitted write does trap");

        // No amount of waiting will resolve the situation, as nothing is emptying the simplex
        // buffer.
        never_resolves(writer.write_ready()).await;

        // There is 2k buffered between the simplex and worker buffers. I should be able to read
        // all of it out:
        let mut buf = [0; 2048];
        reader.read_exact(&mut buf).await.unwrap();

        // and no more:
        never_resolves(reader.read(&mut buf)).await;

        // Now the backpressure should be cleared, and an additional write should be accepted.
        match resolves_immediately(writer.write_ready())
            .await
            .expect("ready is ok")
        {
            WriteReadiness::Ready(1024) => {}
            a => panic!("invalid write readiness: {a:?}"),
        }

        // and the write succeeds:
        match writer.write(chunk.clone()).expect("write does not trap") {
            // There's a race here on how fast the worker consumes the input, so we might see that
            // either it's consumed everything, or that the buffer is currently full.
            Some(WriteReadiness::Ready(1024)) => {}

            // If the worker hasn't picked up the write yet, the buffer will be full. We need to
            // wait for the worker to process the buffer before continuing.
            None => {
                match resolves_immediately(writer.write_ready())
                    .await
                    .expect("write_ready does not trap")
                {
                    WriteReadiness::Ready(1024) => {}
                    a => panic!("writer should be ready for more input: {a:?}"),
                }
            }

            Some(a) => panic!("invalid write readiness: {a:?}"),
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn backpressure_write_stream_with_flush() {
        for n in 0..TEST_ITERATIONS {
            backpressure_write_stream_with_flush_aux(n).await;
        }
    }

    #[tracing::instrument]
    async fn backpressure_write_stream_with_flush_aux(n: usize) {
        tracing::info!("");

        // The channel can buffer up to 1k, plus another 1k in the stream, before not
        // accepting more input:
        let (mut reader, writer) = simplex(1024);
        let mut writer = AsyncWriteStream::new(1024, writer);

        let chunk = Bytes::from_static(&[0; 1024]);

        // Write enough to fill the simplex buffer:
        match writer.write(chunk.clone()).expect("write does not trap") {
            Some(WriteReadiness::Ready(1024)) => {}

            // If the worker hasn't picked up the write yet, the buffer will be full. We need to
            // wait for the worker to process the buffer before continuing.
            None => match writer.flush().expect("flush does not trap") {
                Some(FlushResult::Done) => {}
                None => match resolves_immediately(writer.flush_ready())
                    .await
                    .expect("flush_ready does not trap")
                {
                    FlushResult::Done => {}
                    a => panic!("invalid flush_ready result: {a:?}"),
                },
                a => panic!("invalid flush result: {a:?}"),
            },

            Some(a) => panic!("invalid write readiness: {a:?}"),
        }

        // Now fill the buffer between here and the writer task. This should always indicate
        // back-pressure because now both buffers (simplex and worker) are full.
        match writer.write(chunk.clone()).expect("write does not trap") {
            None => {}
            Some(a) => panic!("expected backpressure: {a:?}"),
        }

        // Flushing the buffer should not succeed.
        assert!(
            writer.flush().expect("flush does not trap").is_none(),
            "flush should not succeed"
        );
        never_resolves(writer.flush_ready()).await;

        // No amount of waiting will resolve the situation, as nothing is emptying the simplex
        // buffer.
        never_resolves(writer.write_ready()).await;

        // There is 2k buffered between the simplex and worker buffers. I should be able to read
        // all of it out:
        let mut buf = [0; 2048];
        reader.read_exact(&mut buf).await.unwrap();

        // and no more:
        never_resolves(reader.read(&mut buf)).await;

        // Now the backpressure should be cleared, and an additional write should be accepted.
        match resolves_immediately(writer.write_ready())
            .await
            .expect("ready is ok")
        {
            WriteReadiness::Ready(1024) => {}
            a => panic!("invalid write readiness: {a:?}"),
        }

        // The flush should be cleared as well.
        match resolves_immediately(writer.flush_ready())
            .await
            .expect("ready is ok")
        {
            FlushResult::Done => {}
            a => panic!("invalid write readiness: {a:?}"),
        }

        // and the write succeeds:
        match writer.write(chunk.clone()).expect("write does not trap") {
            // There's a race here on how fast the worker consumes the input, so we might see that
            // either it's consumed everything, or that the buffer is currently full.
            Some(WriteReadiness::Ready(1024)) => {}

            // If the worker hasn't picked up the write yet, the buffer will be full. We need to
            // wait for the worker to process the buffer before continuing.
            None => {
                match resolves_immediately(writer.write_ready())
                    .await
                    .expect("write_ready does not trap")
                {
                    WriteReadiness::Ready(1024) => {}
                    a => panic!("writer should be ready for more input: {a:?}"),
                }
            }

            Some(a) => panic!("invalid write readiness: {a:?}"),
        }
    }
}

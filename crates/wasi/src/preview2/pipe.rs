//! Virtual pipes.
//!
//! These types provide easy implementations of `WasiFile` that mimic much of the behavior of Unix
//! pipes. These are particularly helpful for redirecting WASI stdio handles to destinations other
//! than OS files.
//!
//! Some convenience constructors are included for common backing types like `Vec<u8>` and `String`,
//! but the virtual pipes can be instantiated with any `Read` or `Write` type.
//!
use crate::preview2::{HostInputStream, HostOutputStream, OutputStreamError, StreamState};
use anyhow::{anyhow, Error};
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
    capacity: usize,
    buffer: std::sync::Arc<std::sync::Mutex<bytes::BytesMut>>,
}

impl MemoryOutputPipe {
    pub fn new(capacity: usize) -> Self {
        MemoryOutputPipe {
            capacity,
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
    fn write(&mut self, bytes: Bytes) -> Result<(), OutputStreamError> {
        let mut buf = self.buffer.lock().unwrap();
        if bytes.len() > self.capacity - buf.len() {
            return Err(OutputStreamError::Trap(anyhow!(
                "write beyond capacity of MemoryOutputPipe"
            )));
        }
        buf.extend_from_slice(bytes.as_ref());
        // Always ready for writing
        Ok(())
    }
    fn flush(&mut self) -> Result<(), OutputStreamError> {
        // This stream is always flushed
        Ok(())
    }
    async fn write_ready(&mut self) -> Result<usize, OutputStreamError> {
        let consumed = self.buffer.lock().unwrap().len();
        if consumed < self.capacity {
            Ok(self.capacity - consumed)
        } else {
            // Since the buffer is full, no more bytes will ever be written
            Err(OutputStreamError::Closed)
        }
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
    _join_handle: crate::preview2::AbortOnDropJoinHandle<()>,
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
            _join_handle: join_handle,
        }
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
            Err(TryRecvError::Disconnected) => Err(anyhow!(
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
                return Err(anyhow!(
                    "no more sender for an open AsyncReadStream - should be impossible"
                ))
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct WorkerState {
    alive: bool,
    items: std::collections::VecDeque<Bytes>,
    write_budget: usize,
    flush_pending: bool,
    error: Option<anyhow::Error>,
}

impl WorkerState {
    fn check_error(&mut self) -> Result<(), OutputStreamError> {
        if let Some(e) = self.error.take() {
            return Err(OutputStreamError::LastOperationFailed(e));
        }
        if !self.alive {
            return Err(OutputStreamError::Closed);
        }
        Ok(())
    }
}

struct Worker {
    state: Mutex<WorkerState>,
    new_work: tokio::sync::Notify,
    write_ready_changed: tokio::sync::Notify,
}

enum Job {
    Flush,
    Write(Bytes),
}

enum WriteStatus<'a> {
    Done(Result<usize, OutputStreamError>),
    Pending(tokio::sync::futures::Notified<'a>),
}

impl Worker {
    fn new(write_budget: usize) -> Self {
        Self {
            state: Mutex::new(WorkerState {
                alive: true,
                items: std::collections::VecDeque::new(),
                write_budget,
                flush_pending: false,
                error: None,
            }),
            new_work: tokio::sync::Notify::new(),
            write_ready_changed: tokio::sync::Notify::new(),
        }
    }
    fn check_write(&self) -> WriteStatus<'_> {
        let mut state = self.state();
        if let Err(e) = state.check_error() {
            return WriteStatus::Done(Err(e));
        }

        if state.flush_pending || state.write_budget == 0 {
            return WriteStatus::Pending(self.write_ready_changed.notified());
        }

        WriteStatus::Done(Ok(state.write_budget))
    }
    fn state(&self) -> std::sync::MutexGuard<WorkerState> {
        self.state.lock().unwrap()
    }
    fn pop(&self) -> Option<Job> {
        let mut state = self.state();
        if state.items.is_empty() {
            if state.flush_pending {
                return Some(Job::Flush);
            }
        } else if let Some(bytes) = state.items.pop_front() {
            return Some(Job::Write(bytes));
        }

        None
    }
    fn report_error(&self, e: std::io::Error) {
        {
            let mut state = self.state();
            state.alive = false;
            state.error = Some(e.into());
            state.flush_pending = false;
        }
        self.write_ready_changed.notify_waiters();
    }
    async fn work<T: tokio::io::AsyncWrite + Send + Sync + Unpin + 'static>(&self, mut writer: T) {
        use tokio::io::AsyncWriteExt;
        loop {
            let notified = self.new_work.notified();
            while let Some(job) = self.pop() {
                match job {
                    Job::Flush => {
                        if let Err(e) = writer.flush().await {
                            self.report_error(e);
                            return;
                        }

                        tracing::debug!("worker marking flush complete");
                        self.state().flush_pending = false;
                    }

                    Job::Write(mut bytes) => {
                        tracing::debug!("worker writing: {bytes:?}");
                        let len = bytes.len();
                        match writer.write_all_buf(&mut bytes).await {
                            Err(e) => {
                                self.report_error(e);
                                return;
                            }
                            Ok(_) => {
                                self.state().write_budget += len;
                            }
                        }
                    }
                }

                self.write_ready_changed.notify_waiters();
            }

            notified.await;
        }
    }
}

/// Provides a [`HostOutputStream`] impl from a [`tokio::io::AsyncWrite`] impl
pub struct AsyncWriteStream {
    worker: Arc<Worker>,
    _join_handle: crate::preview2::AbortOnDropJoinHandle<()>,
}

impl AsyncWriteStream {
    /// Create a [`AsyncWriteStream`]. In order to use the [`HostOutputStream`] impl
    /// provided by this struct, the argument must impl [`tokio::io::AsyncWrite`].
    pub fn new<T: tokio::io::AsyncWrite + Send + Sync + Unpin + 'static>(
        write_budget: usize,
        writer: T,
    ) -> Self {
        let worker = Arc::new(Worker::new(write_budget));

        let w = Arc::clone(&worker);
        let join_handle = crate::preview2::spawn(async move { w.work(writer).await });

        AsyncWriteStream {
            worker,
            _join_handle: join_handle,
        }
    }
}

#[async_trait::async_trait]
impl HostOutputStream for AsyncWriteStream {
    fn write(&mut self, bytes: Bytes) -> Result<(), OutputStreamError> {
        let mut state = self.worker.state();
        state.check_error()?;
        if state.flush_pending {
            return Err(OutputStreamError::Trap(anyhow!(
                "write not permitted while flush pending"
            )));
        }
        match state.write_budget.checked_sub(bytes.len()) {
            Some(remaining_budget) => {
                state.write_budget = remaining_budget;
                state.items.push_back(bytes);
            }
            None => return Err(OutputStreamError::Trap(anyhow!("write exceeded budget"))),
        }
        drop(state);
        self.worker.new_work.notify_waiters();
        Ok(())
    }
    fn flush(&mut self) -> Result<(), OutputStreamError> {
        let mut state = self.worker.state();
        state.check_error()?;

        state.flush_pending = true;
        self.worker.new_work.notify_waiters();

        Ok(())
    }

    async fn write_ready(&mut self) -> Result<usize, OutputStreamError> {
        loop {
            match self.worker.check_write() {
                WriteStatus::Done(r) => return r,
                WriteStatus::Pending(notifier) => notifier.await,
            }
        }
    }
}

/// An output stream that consumes all input written to it, and is always ready.
pub struct SinkOutputStream;

#[async_trait::async_trait]
impl HostOutputStream for SinkOutputStream {
    fn write(&mut self, _buf: Bytes) -> Result<(), OutputStreamError> {
        Ok(())
    }
    fn flush(&mut self) -> Result<(), OutputStreamError> {
        // This stream is always flushed
        Ok(())
    }

    async fn write_ready(&mut self) -> Result<usize, OutputStreamError> {
        // This stream is always ready for writing.
        Ok(usize::MAX)
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
    fn write(&mut self, _: Bytes) -> Result<(), OutputStreamError> {
        Err(OutputStreamError::Closed)
    }
    fn flush(&mut self) -> Result<(), OutputStreamError> {
        Err(OutputStreamError::Closed)
    }

    async fn write_ready(&mut self) -> Result<usize, OutputStreamError> {
        Err(OutputStreamError::Closed)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    // This is a gross way to handle CI running under qemu for non-x86 architectures.
    #[cfg(not(target_arch = "x86_64"))]
    const TEST_ITERATIONS: usize = 10;

    // This is a gross way to handle CI running under qemu for non-x86 architectures.
    #[cfg(not(target_arch = "x86_64"))]
    const REASONABLE_DURATION: std::time::Duration = std::time::Duration::from_millis(200);

    #[cfg(target_arch = "x86_64")]
    const TEST_ITERATIONS: usize = 100;

    #[cfg(target_arch = "x86_64")]
    const REASONABLE_DURATION: std::time::Duration = std::time::Duration::from_millis(10);

    async fn resolves_immediately<F, O>(fut: F) -> O
    where
        F: futures::Future<Output = O>,
    {
        tokio::time::timeout(REASONABLE_DURATION, fut)
            .await
            .expect("operation timed out")
    }

    // TODO: is there a way to get tokio to warp through timeouts when it knows nothing is
    // happening?
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

        let readiness = resolves_immediately(writer.write_ready())
            .await
            .expect("write_ready does not trap");
        assert_eq!(readiness, 2048);
        // I can write whatever:
        writer.write(chunk.clone()).expect("write does not error");

        // This may consume 1k of the buffer:
        let readiness = resolves_immediately(writer.write_ready())
            .await
            .expect("write_ready does not trap");
        assert!(
            readiness == 1024 || readiness == 2048,
            "readiness should be 1024 or 2048, got {readiness}"
        );

        if readiness == 1024 {
            writer.write(chunk.clone()).expect("write does not error");

            let readiness = resolves_immediately(writer.write_ready())
                .await
                .expect("write_ready does not trap");
            assert!(
                readiness == 1024 || readiness == 2048,
                "readiness should be 1024 or 2048, got {readiness}"
            );
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

        // First the api is going to report the last operation failed, then subsequently
        // it will be reported as closed. We set this flag once we see LastOperationFailed.
        let mut should_be_closed = false;

        // Write some data to the stream to ensure we have data that cannot be flushed.
        let chunk = Bytes::from_static(&[0; 1]);
        writer
            .write(chunk.clone())
            .expect("first write should succeed");

        // The rest of this test should be valid whether or not we check write readiness:
        let mut write_ready_res = None;
        if n % 2 == 0 {
            let r = resolves_immediately(writer.write_ready()).await;
            // Check write readiness:
            match r {
                // worker hasn't processed write yet:
                Ok(1023) => {}
                // worker reports failure:
                Err(OutputStreamError::LastOperationFailed(_)) => {
                    tracing::debug!("discovered stream failure in first write_ready");
                    should_be_closed = true;
                }
                r => panic!("unexpected write_ready: {r:?}"),
            }
            write_ready_res = Some(r);
        }

        // When we drop the simplex reader, that causes the simplex writer to return BrokenPipe on
        // its write. Now that the buffering crank has turned, our next write will give BrokenPipe.
        let flush_res = writer.flush();
        match flush_res {
            // worker reports failure:
            Err(OutputStreamError::LastOperationFailed(_)) => {
                tracing::debug!("discovered stream failure trying to flush");
                assert!(!should_be_closed);
                should_be_closed = true;
            }
            // Already reported failure, now closed
            Err(OutputStreamError::Closed) => {
                assert!(
                    should_be_closed,
                    "expected a LastOperationFailed before we see Closed. {write_ready_res:?}"
                );
            }
            // Also possible the worker hasnt processed write yet:
            Ok(()) => {}
            Err(e) => panic!("unexpected flush error: {e:?} {write_ready_res:?}"),
        }

        // Waiting for the flush to complete should always indicate that the channel has been
        // closed.
        match resolves_immediately(writer.write_ready()).await {
            // worker reports failure:
            Err(OutputStreamError::LastOperationFailed(_)) => {
                tracing::debug!("discovered stream failure trying to flush");
                assert!(!should_be_closed);
            }
            // Already reported failure, now closed
            Err(OutputStreamError::Closed) => {
                assert!(should_be_closed);
            }
            r => {
                panic!("stream should be reported closed by the end of write_ready after flush, got {r:?}. {write_ready_res:?} {flush_res:?}")
            }
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn multiple_chunks_write_stream() {
        // Run many times because the test is nondeterministic:
        for n in 0..TEST_ITERATIONS {
            multiple_chunks_write_stream_aux(n).await
        }
    }
    #[tracing::instrument]
    async fn multiple_chunks_write_stream_aux(_: usize) {
        use std::ops::Deref;

        let (mut reader, writer) = simplex(1024);
        let mut writer = AsyncWriteStream::new(1024, writer);

        // Write a chunk:
        let chunk = Bytes::from_static(&[123; 1]);

        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("write should be ready");
        assert_eq!(permit, 1024);

        writer.write(chunk.clone()).expect("write does not trap");

        // At this point the message will either be waiting for the worker to process the write, or
        // it will be buffered in the simplex channel.
        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("write should be ready");
        assert!(matches!(permit, 1023 | 1024));

        let mut read_buf = vec![0; chunk.len()];
        let read_len = reader.read_exact(&mut read_buf).await.unwrap();
        assert_eq!(read_len, chunk.len());
        assert_eq!(read_buf.as_slice(), chunk.deref());

        // Write a second, different chunk:
        let chunk2 = Bytes::from_static(&[45; 1]);

        // We're only guaranteed to see a consistent write budget if we flush.
        writer.flush().expect("channel is still alive");

        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("write should be ready");
        assert_eq!(permit, 1024);

        writer.write(chunk2.clone()).expect("write does not trap");

        // At this point the message will either be waiting for the worker to process the write, or
        // it will be buffered in the simplex channel.
        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("write should be ready");
        assert!(matches!(permit, 1023 | 1024));

        let mut read2_buf = vec![0; chunk2.len()];
        let read2_len = reader.read_exact(&mut read2_buf).await.unwrap();
        assert_eq!(read2_len, chunk2.len());
        assert_eq!(read2_buf.as_slice(), chunk2.deref());

        // We're only guaranteed to see a consistent write budget if we flush.
        writer.flush().expect("channel is still alive");

        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("write should be ready");
        assert_eq!(permit, 1024);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn backpressure_write_stream() {
        // Run many times because the test is nondeterministic:
        for n in 0..TEST_ITERATIONS {
            backpressure_write_stream_aux(n).await
        }
    }
    #[tracing::instrument]
    async fn backpressure_write_stream_aux(_: usize) {
        use futures::future::poll_immediate;

        // The channel can buffer up to 1k, plus another 1k in the stream, before not
        // accepting more input:
        let (mut reader, writer) = simplex(1024);
        let mut writer = AsyncWriteStream::new(1024, writer);

        let chunk = Bytes::from_static(&[0; 1024]);

        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("write should be ready");
        assert_eq!(permit, 1024);

        writer.write(chunk.clone()).expect("write succeeds");

        // We might still be waiting for the worker to process the message, or the worker may have
        // processed it and released all the budget back to us.
        let permit = poll_immediate(writer.write_ready()).await;
        assert!(matches!(permit, None | Some(Ok(1024))));

        // Given a little time, the worker will process the message and release all the budget
        // back.
        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("write should be ready");
        assert_eq!(permit, 1024);

        // Now fill the buffer between here and the writer task. This should always indicate
        // back-pressure because now both buffers (simplex and worker) are full.
        writer.write(chunk.clone()).expect("write does not trap");

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
        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("ready is ok");
        assert_eq!(permit, 1024);

        // and the write succeeds:
        writer.write(chunk.clone()).expect("write does not trap");
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn backpressure_write_stream_with_flush() {
        for n in 0..TEST_ITERATIONS {
            backpressure_write_stream_with_flush_aux(n).await;
        }
    }

    async fn backpressure_write_stream_with_flush_aux(_: usize) {
        // The channel can buffer up to 1k, plus another 1k in the stream, before not
        // accepting more input:
        let (mut reader, writer) = simplex(1024);
        let mut writer = AsyncWriteStream::new(1024, writer);

        let chunk = Bytes::from_static(&[0; 1024]);

        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("write should be ready");
        assert_eq!(permit, 1024);

        writer.write(chunk.clone()).expect("write succeeds");

        writer.flush().expect("flush succeeds");

        // Waiting for write_ready to resolve after a flush should always show that we have the
        // full budget available, as the message will have flushed to the simplex channel.
        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("write_ready succeeds");
        assert_eq!(permit, 1024);

        // Write enough to fill the simplex buffer:
        writer.write(chunk.clone()).expect("write does not trap");

        // Writes should be refused until this flush succeeds.
        writer.flush().expect("flush succeeds");

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
        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("ready is ok");
        assert_eq!(permit, 1024);

        // and the write succeeds:
        writer.write(chunk.clone()).expect("write does not trap");

        writer.flush().expect("flush succeeds");

        let permit = resolves_immediately(writer.write_ready())
            .await
            .expect("ready is ok");
        assert_eq!(permit, 1024);
    }
}

use crate::p2::{
    DynInputStream, DynOutputStream, InputStream, OutputStream, Pollable, SocketError,
    SocketResult, StreamError,
};
use crate::runtime::AbortOnDropJoinHandle;
use crate::sockets::TcpSocket;
use anyhow::Result;
use io_lifetimes::AsSocketlike;
use rustix::io::Errno;
use std::io;
use std::mem;
use std::net::Shutdown;
use std::sync::Arc;
use tokio::sync::Mutex;

impl TcpSocket {
    pub(crate) fn p2_streams(&mut self) -> SocketResult<(DynInputStream, DynOutputStream)> {
        let client = self.tcp_stream_arc()?;
        let reader = Arc::new(Mutex::new(TcpReader::new(client.clone())));
        let writer = Arc::new(Mutex::new(TcpWriter::new(client.clone())));
        self.set_p2_streaming_state(P2TcpStreamingState {
            stream: client.clone(),
            reader: reader.clone(),
            writer: writer.clone(),
        })?;
        let input: DynInputStream = Box::new(TcpReadStream(reader));
        let output: DynOutputStream = Box::new(TcpWriteStream(writer));
        Ok((input, output))
    }
}

pub(crate) struct P2TcpStreamingState {
    pub(crate) stream: Arc<tokio::net::TcpStream>,
    reader: Arc<Mutex<TcpReader>>,
    writer: Arc<Mutex<TcpWriter>>,
}

impl P2TcpStreamingState {
    pub(crate) fn shutdown(&self, how: Shutdown) -> SocketResult<()> {
        if let Shutdown::Both | Shutdown::Read = how {
            try_lock_for_socket(&self.reader)?.shutdown();
        }

        if let Shutdown::Both | Shutdown::Write = how {
            try_lock_for_socket(&self.writer)?.shutdown();
        }

        Ok(())
    }
}

struct TcpReader {
    stream: Arc<tokio::net::TcpStream>,
    closed: bool,
}

impl TcpReader {
    fn new(stream: Arc<tokio::net::TcpStream>) -> Self {
        Self {
            stream,
            closed: false,
        }
    }
    fn read(&mut self, size: usize) -> Result<bytes::Bytes, StreamError> {
        if self.closed {
            return Err(StreamError::Closed);
        }
        if size == 0 {
            return Ok(bytes::Bytes::new());
        }

        let mut buf = bytes::BytesMut::with_capacity(size);
        let n = match self.stream.try_read_buf(&mut buf) {
            // A 0-byte read indicates that the stream has closed.
            Ok(0) => {
                self.closed = true;
                return Err(StreamError::Closed);
            }
            Ok(n) => n,

            // Failing with `EWOULDBLOCK` is how we differentiate between a closed channel and no
            // data to read right now.
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => 0,

            Err(e) => {
                self.closed = true;
                return Err(StreamError::LastOperationFailed(e.into()));
            }
        };

        buf.truncate(n);
        Ok(buf.freeze())
    }

    fn shutdown(&mut self) {
        native_shutdown(&self.stream, Shutdown::Read);
        self.closed = true;
    }

    async fn ready(&mut self) {
        if self.closed {
            return;
        }

        self.stream.readable().await.unwrap();
    }
}

struct TcpReadStream(Arc<Mutex<TcpReader>>);

#[async_trait::async_trait]
impl InputStream for TcpReadStream {
    fn read(&mut self, size: usize) -> Result<bytes::Bytes, StreamError> {
        try_lock_for_stream(&self.0)?.read(size)
    }
}

#[async_trait::async_trait]
impl Pollable for TcpReadStream {
    async fn ready(&mut self) {
        self.0.lock().await.ready().await
    }
}

const SOCKET_READY_SIZE: usize = 1024 * 1024 * 1024;

struct TcpWriter {
    stream: Arc<tokio::net::TcpStream>,
    state: WriteState,
}

enum WriteState {
    Ready,
    Writing(AbortOnDropJoinHandle<io::Result<()>>),
    Closing(AbortOnDropJoinHandle<io::Result<()>>),
    Closed,
    Error(io::Error),
}

impl TcpWriter {
    fn new(stream: Arc<tokio::net::TcpStream>) -> Self {
        Self {
            stream,
            state: WriteState::Ready,
        }
    }

    fn try_write_portable(stream: &tokio::net::TcpStream, buf: &[u8]) -> io::Result<usize> {
        stream.try_write(buf).map_err(|error| {
            match Errno::from_io_error(&error) {
                // Windows returns `WSAESHUTDOWN` when writing to a shut down socket.
                // We normalize this to EPIPE, because that is what the other platforms return.
                // See: https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-send#:~:text=WSAESHUTDOWN
                #[cfg(windows)]
                Some(Errno::SHUTDOWN) => io::Error::new(io::ErrorKind::BrokenPipe, error),

                _ => error,
            }
        })
    }

    /// Write `bytes` in a background task, remembering the task handle for use in a future call to
    /// `write_ready`
    fn background_write(&mut self, mut bytes: bytes::Bytes) {
        assert!(matches!(self.state, WriteState::Ready));

        let stream = self.stream.clone();
        self.state = WriteState::Writing(crate::runtime::spawn(async move {
            // Note: we are not using the AsyncWrite impl here, and instead using the TcpStream
            // primitive try_write, which goes directly to attempt a write with mio. This has
            // two advantages: 1. this operation takes a &TcpStream instead of a &mut TcpStream
            // required to AsyncWrite, and 2. it eliminates any buffering in tokio we may need
            // to flush.
            while !bytes.is_empty() {
                stream.writable().await?;
                match Self::try_write_portable(&stream, &bytes) {
                    Ok(n) => {
                        let _ = bytes.split_to(n);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(e) => return Err(e),
                }
            }

            Ok(())
        }));
    }

    fn write(&mut self, mut bytes: bytes::Bytes) -> Result<(), StreamError> {
        match self.state {
            WriteState::Ready => {}
            WriteState::Closed => return Err(StreamError::Closed),
            WriteState::Writing(_) | WriteState::Closing(_) | WriteState::Error(_) => {
                return Err(StreamError::Trap(anyhow::anyhow!(
                    "unpermitted: must call check_write first"
                )));
            }
        }
        while !bytes.is_empty() {
            match Self::try_write_portable(&self.stream, &bytes) {
                Ok(n) => {
                    let _ = bytes.split_to(n);
                }

                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // As `try_write` indicated that it would have blocked, we'll perform the write
                    // in the background to allow us to return immediately.
                    self.background_write(bytes);

                    return Ok(());
                }

                Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                    self.state = WriteState::Closed;
                    return Err(StreamError::Closed);
                }

                Err(e) => return Err(StreamError::LastOperationFailed(e.into())),
            }
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), StreamError> {
        // `flush` is a no-op here, as we're not managing any internal buffer. Additionally,
        // `write_ready` will join the background write task if it's active, so following `flush`
        // with `write_ready` will have the desired effect.
        match self.state {
            WriteState::Ready
            | WriteState::Writing(_)
            | WriteState::Closing(_)
            | WriteState::Error(_) => Ok(()),
            WriteState::Closed => Err(StreamError::Closed),
        }
    }

    fn check_write(&mut self) -> Result<usize, StreamError> {
        match mem::replace(&mut self.state, WriteState::Closed) {
            WriteState::Writing(task) => {
                self.state = WriteState::Writing(task);
                return Ok(0);
            }
            WriteState::Closing(task) => {
                self.state = WriteState::Closing(task);
                return Ok(0);
            }
            WriteState::Ready => {
                self.state = WriteState::Ready;
            }
            WriteState::Closed => return Err(StreamError::Closed),
            WriteState::Error(e) => return Err(StreamError::LastOperationFailed(e.into())),
        }

        let writable = self.stream.writable();
        futures::pin_mut!(writable);
        if crate::runtime::poll_noop(writable).is_none() {
            return Ok(0);
        }
        Ok(SOCKET_READY_SIZE)
    }

    fn shutdown(&mut self) {
        self.state = match mem::replace(&mut self.state, WriteState::Closed) {
            // No write in progress, immediately shut down:
            WriteState::Ready => {
                native_shutdown(&self.stream, Shutdown::Write);
                WriteState::Closed
            }

            // Schedule the shutdown after the current write has finished:
            WriteState::Writing(write) => {
                let stream = self.stream.clone();
                WriteState::Closing(crate::runtime::spawn(async move {
                    let result = write.await;
                    native_shutdown(&stream, Shutdown::Write);
                    result
                }))
            }

            s => s,
        };
    }

    async fn cancel(&mut self) {
        match mem::replace(&mut self.state, WriteState::Closed) {
            WriteState::Writing(task) | WriteState::Closing(task) => _ = task.cancel().await,
            _ => {}
        }
    }

    async fn ready(&mut self) {
        match &mut self.state {
            WriteState::Writing(task) => {
                self.state = match task.await {
                    Ok(()) => WriteState::Ready,
                    Err(e) => WriteState::Error(e),
                }
            }
            WriteState::Closing(task) => {
                self.state = match task.await {
                    Ok(()) => WriteState::Closed,
                    Err(e) => WriteState::Error(e),
                }
            }
            _ => {}
        }

        if let WriteState::Ready = self.state {
            self.stream.writable().await.unwrap();
        }
    }
}

struct TcpWriteStream(Arc<Mutex<TcpWriter>>);

#[async_trait::async_trait]
impl OutputStream for TcpWriteStream {
    fn write(&mut self, bytes: bytes::Bytes) -> Result<(), StreamError> {
        try_lock_for_stream(&self.0)?.write(bytes)
    }

    fn flush(&mut self) -> Result<(), StreamError> {
        try_lock_for_stream(&self.0)?.flush()
    }

    fn check_write(&mut self) -> Result<usize, StreamError> {
        try_lock_for_stream(&self.0)?.check_write()
    }

    async fn cancel(&mut self) {
        self.0.lock().await.cancel().await
    }
}

#[async_trait::async_trait]
impl Pollable for TcpWriteStream {
    async fn ready(&mut self) {
        self.0.lock().await.ready().await
    }
}

fn native_shutdown(stream: &tokio::net::TcpStream, how: Shutdown) {
    _ = stream
        .as_socketlike_view::<std::net::TcpStream>()
        .shutdown(how);
}

fn try_lock_for_stream<T>(mutex: &Mutex<T>) -> Result<tokio::sync::MutexGuard<'_, T>, StreamError> {
    mutex
        .try_lock()
        .map_err(|_| StreamError::trap("concurrent access to resource not supported"))
}

fn try_lock_for_socket<T>(mutex: &Mutex<T>) -> SocketResult<tokio::sync::MutexGuard<'_, T>> {
    mutex.try_lock().map_err(|_| {
        SocketError::trap(anyhow::anyhow!(
            "concurrent access to resource not supported"
        ))
    })
}

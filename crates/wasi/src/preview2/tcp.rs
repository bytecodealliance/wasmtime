use super::{HostInputStream, HostOutputStream, OutputStreamError};
use crate::preview2::poll::Subscribe;
use crate::preview2::stream::{InputStream, OutputStream};
use crate::preview2::{with_ambient_tokio_runtime, AbortOnDropJoinHandle, StreamState};
use anyhow::{Error, Result};
use cap_net_ext::{AddressFamily, Blocking, TcpListenerExt};
use cap_std::net::TcpListener;
use io_lifetimes::raw::{FromRawSocketlike, IntoRawSocketlike};
use std::io;
use std::mem;
use std::sync::Arc;
use tokio::io::Interest;

/// The state of a TCP socket.
///
/// This represents the various states a socket can be in during the
/// activities of binding, listening, accepting, and connecting.
pub(crate) enum TcpState {
    /// The initial state for a newly-created socket.
    Default,

    /// Binding started via `start_bind`.
    BindStarted,

    /// Binding finished via `finish_bind`. The socket has an address but
    /// is not yet listening for connections.
    Bound,

    /// Listening started via `listen_start`.
    ListenStarted,

    /// The socket is now listening and waiting for an incoming connection.
    Listening,

    /// An outgoing connection is started via `start_connect`.
    Connecting,

    /// An outgoing connection is ready to be established.
    ConnectReady,

    /// An outgoing connection has been established.
    Connected,
}

/// A host TCP socket, plus associated bookkeeping.
///
/// The inner state is wrapped in an Arc because the same underlying socket is
/// used for implementing the stream types.
pub struct TcpSocket {
    /// The part of a `TcpSocket` which is reference-counted so that we
    /// can pass it to async tasks.
    pub(crate) inner: Arc<tokio::net::TcpStream>,

    /// The current state in the bind/listen/accept/connect progression.
    pub(crate) tcp_state: TcpState,
}

pub(crate) struct TcpReadStream {
    stream: Arc<tokio::net::TcpStream>,
    closed: bool,
}

impl TcpReadStream {
    fn new(stream: Arc<tokio::net::TcpStream>) -> Self {
        Self {
            stream,
            closed: false,
        }
    }
    fn stream_state(&self) -> StreamState {
        if self.closed {
            StreamState::Closed
        } else {
            StreamState::Open
        }
    }
}

#[async_trait::async_trait]
impl HostInputStream for TcpReadStream {
    fn read(&mut self, size: usize) -> Result<(bytes::Bytes, StreamState), anyhow::Error> {
        if size == 0 || self.closed {
            return Ok((bytes::Bytes::new(), self.stream_state()));
        }

        let mut buf = bytes::BytesMut::with_capacity(size);
        let n = match self.stream.try_read_buf(&mut buf) {
            // A 0-byte read indicates that the stream has closed.
            Ok(0) => {
                self.closed = true;
                0
            }
            Ok(n) => n,

            // Failing with `EWOULDBLOCK` is how we differentiate between a closed channel and no
            // data to read right now.
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => 0,

            Err(e) => {
                tracing::debug!("unexpected error on TcpReadStream read: {e:?}");
                self.closed = true;
                0
            }
        };

        buf.truncate(n);
        Ok((buf.freeze(), self.stream_state()))
    }
}

#[async_trait::async_trait]
impl Subscribe for TcpReadStream {
    async fn ready(&mut self) {
        if self.closed {
            return;
        }
        self.stream.readable().await.unwrap();
    }
}

const SOCKET_READY_SIZE: usize = 1024 * 1024 * 1024;

pub(crate) struct TcpWriteStream {
    stream: Arc<tokio::net::TcpStream>,
    last_write: LastWrite,
}

enum LastWrite {
    Waiting(AbortOnDropJoinHandle<Result<()>>),
    Error(Error),
    Done,
}

impl TcpWriteStream {
    pub(crate) fn new(stream: Arc<tokio::net::TcpStream>) -> Self {
        Self {
            stream,
            last_write: LastWrite::Done,
        }
    }

    /// Write `bytes` in a background task, remembering the task handle for use in a future call to
    /// `write_ready`
    fn background_write(&mut self, mut bytes: bytes::Bytes) {
        assert!(matches!(self.last_write, LastWrite::Done));

        let stream = self.stream.clone();
        self.last_write = LastWrite::Waiting(crate::preview2::spawn(async move {
            // Note: we are not using the AsyncWrite impl here, and instead using the TcpStream
            // primitive try_write, which goes directly to attempt a write with mio. This has
            // two advantages: 1. this operation takes a &TcpStream instead of a &mut TcpStream
            // required to AsyncWrite, and 2. it eliminates any buffering in tokio we may need
            // to flush.
            while !bytes.is_empty() {
                stream.writable().await?;
                match stream.try_write(&bytes) {
                    Ok(n) => {
                        let _ = bytes.split_to(n);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(e) => return Err(e.into()),
                }
            }

            Ok(())
        }));
    }
}

impl HostOutputStream for TcpWriteStream {
    fn write(&mut self, mut bytes: bytes::Bytes) -> Result<(), OutputStreamError> {
        match self.last_write {
            LastWrite::Done => {}
            LastWrite::Waiting(_) | LastWrite::Error(_) => {
                return Err(OutputStreamError::Trap(anyhow::anyhow!(
                    "unpermitted: must call check_write first"
                )));
            }
        }
        while !bytes.is_empty() {
            match self.stream.try_write(&bytes) {
                Ok(n) => {
                    let _ = bytes.split_to(n);
                }

                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // As `try_write` indicated that it would have blocked, we'll perform the write
                    // in the background to allow us to return immediately.
                    self.background_write(bytes);

                    return Ok(());
                }

                Err(e) => return Err(OutputStreamError::LastOperationFailed(e.into())),
            }
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), OutputStreamError> {
        // `flush` is a no-op here, as we're not managing any internal buffer. Additionally,
        // `write_ready` will join the background write task if it's active, so following `flush`
        // with `write_ready` will have the desired effect.
        Ok(())
    }

    fn check_write(&mut self) -> Result<usize, OutputStreamError> {
        match mem::replace(&mut self.last_write, LastWrite::Done) {
            LastWrite::Waiting(task) => {
                self.last_write = LastWrite::Waiting(task);
                return Ok(0);
            }
            LastWrite::Done => {}
            LastWrite::Error(e) => return Err(OutputStreamError::LastOperationFailed(e.into())),
        }

        let writable = self.stream.writable();
        futures::pin_mut!(writable);
        if super::poll_noop(writable).is_none() {
            return Ok(0);
        }
        Ok(SOCKET_READY_SIZE)
    }
}

#[async_trait::async_trait]
impl Subscribe for TcpWriteStream {
    async fn ready(&mut self) {
        if let LastWrite::Waiting(task) = &mut self.last_write {
            self.last_write = match task.await {
                Ok(()) => LastWrite::Done,
                Err(e) => LastWrite::Error(e),
            };
        }
        if let LastWrite::Done = self.last_write {
            self.stream.writable().await.unwrap();
        }
    }
}

impl TcpSocket {
    /// Create a new socket in the given family.
    pub fn new(family: AddressFamily) -> io::Result<Self> {
        // Create a new host socket and set it to non-blocking, which is needed
        // by our async implementation.
        let tcp_listener = TcpListener::new(family, Blocking::No)?;
        Self::from_tcp_listener(tcp_listener)
    }

    /// Create a `TcpSocket` from an existing socket.
    ///
    /// The socket must be in non-blocking mode.
    pub fn from_tcp_stream(tcp_socket: cap_std::net::TcpStream) -> io::Result<Self> {
        let tcp_listener = TcpListener::from(rustix::fd::OwnedFd::from(tcp_socket));
        Self::from_tcp_listener(tcp_listener)
    }

    pub fn from_tcp_listener(tcp_listener: cap_std::net::TcpListener) -> io::Result<Self> {
        let fd = tcp_listener.into_raw_socketlike();
        let std_stream = unsafe { std::net::TcpStream::from_raw_socketlike(fd) };
        let stream = with_ambient_tokio_runtime(|| tokio::net::TcpStream::try_from(std_stream))?;

        Ok(Self {
            inner: Arc::new(stream),
            tcp_state: TcpState::Default,
        })
    }

    pub fn tcp_socket(&self) -> &tokio::net::TcpStream {
        &self.inner
    }

    /// Create the input/output stream pair for a tcp socket.
    pub fn as_split(&self) -> (InputStream, OutputStream) {
        let input = Box::new(TcpReadStream::new(self.inner.clone()));
        let output = Box::new(TcpWriteStream::new(self.inner.clone()));
        (InputStream::Host(input), output)
    }
}

#[async_trait::async_trait]
impl Subscribe for TcpSocket {
    async fn ready(&mut self) {
        // Some states are ready immediately.
        match self.tcp_state {
            TcpState::BindStarted | TcpState::ListenStarted | TcpState::ConnectReady => return,
            _ => {}
        }

        // FIXME: Add `Interest::ERROR` when we update to tokio 1.32.
        self.inner
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await
            .unwrap();
    }
}

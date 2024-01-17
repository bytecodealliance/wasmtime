use super::network::SocketAddressFamily;
use super::{HostInputStream, HostOutputStream, StreamError};
use crate::preview2::host::network::util;
use crate::preview2::{
    with_ambient_tokio_runtime, AbortOnDropJoinHandle, InputStream, OutputStream, Subscribe,
};
use anyhow::{Error, Result};
use cap_net_ext::{AddressFamily, Blocking};
use io_lifetimes::raw::{FromRawSocketlike, IntoRawSocketlike};
use rustix::net::sockopt;
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

    /// An outgoing connection was attempted but failed.
    ConnectFailed,

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

    /// The desired listen queue size. Set to None to use the system's default.
    pub(crate) listen_backlog_size: Option<i32>,

    pub(crate) family: SocketAddressFamily,

    // The socket options below are not automatically inherited from the listener
    // on all platforms. So we keep track of which options have been explicitly
    // set and manually apply those values to newly accepted clients.
    #[cfg(target_os = "macos")]
    pub(crate) receive_buffer_size: Option<usize>,
    #[cfg(target_os = "macos")]
    pub(crate) send_buffer_size: Option<usize>,
    #[cfg(target_os = "macos")]
    pub(crate) hop_limit: Option<u8>,
    #[cfg(target_os = "macos")]
    pub(crate) keep_alive_idle_time: Option<std::time::Duration>,
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
}

#[async_trait::async_trait]
impl HostInputStream for TcpReadStream {
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
                0
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
    fn write(&mut self, mut bytes: bytes::Bytes) -> Result<(), StreamError> {
        match self.last_write {
            LastWrite::Done => {}
            LastWrite::Waiting(_) | LastWrite::Error(_) => {
                return Err(StreamError::Trap(anyhow::anyhow!(
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

                Err(e) => return Err(StreamError::LastOperationFailed(e.into())),
            }
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), StreamError> {
        // `flush` is a no-op here, as we're not managing any internal buffer. Additionally,
        // `write_ready` will join the background write task if it's active, so following `flush`
        // with `write_ready` will have the desired effect.
        Ok(())
    }

    fn check_write(&mut self) -> Result<usize, StreamError> {
        match mem::replace(&mut self.last_write, LastWrite::Done) {
            LastWrite::Waiting(task) => {
                self.last_write = LastWrite::Waiting(task);
                return Ok(0);
            }
            LastWrite::Done => {}
            LastWrite::Error(e) => return Err(StreamError::LastOperationFailed(e.into())),
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
        let fd = util::tcp_socket(family, Blocking::No)?;

        let socket_address_family = match family {
            AddressFamily::Ipv4 => SocketAddressFamily::Ipv4,
            AddressFamily::Ipv6 => {
                sockopt::set_ipv6_v6only(&fd, true)?;
                SocketAddressFamily::Ipv6
            }
        };

        Self::from_fd(fd, socket_address_family)
    }

    /// Create a `TcpSocket` from an existing socket.
    ///
    /// The socket must be in non-blocking mode.
    pub(crate) fn from_fd(
        fd: rustix::fd::OwnedFd,
        family: SocketAddressFamily,
    ) -> io::Result<Self> {
        let stream = Self::setup_tokio_tcp_stream(fd)?;

        Ok(Self {
            inner: Arc::new(stream),
            tcp_state: TcpState::Default,
            listen_backlog_size: None,
            family,
            #[cfg(target_os = "macos")]
            receive_buffer_size: None,
            #[cfg(target_os = "macos")]
            send_buffer_size: None,
            #[cfg(target_os = "macos")]
            hop_limit: None,
            #[cfg(target_os = "macos")]
            keep_alive_idle_time: None,
        })
    }

    fn setup_tokio_tcp_stream(fd: rustix::fd::OwnedFd) -> io::Result<tokio::net::TcpStream> {
        let std_stream =
            unsafe { std::net::TcpStream::from_raw_socketlike(fd.into_raw_socketlike()) };
        with_ambient_tokio_runtime(|| tokio::net::TcpStream::try_from(std_stream))
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

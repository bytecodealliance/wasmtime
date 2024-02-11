use super::network::SocketAddressFamily;
use super::{
    with_ambient_tokio_runtime, HostInputStream, HostOutputStream, SocketResult, StreamError,
};
use crate::preview2::{AbortOnDropJoinHandle, Subscribe};
use anyhow::{Error, Result};
use cap_net_ext::AddressFamily;
use futures::Future;
use io_lifetimes::views::SocketlikeView;
use io_lifetimes::AsSocketlike;
use rustix::net::sockopt;
use std::io;
use std::mem;
use std::pin::Pin;
use std::sync::Arc;

/// Value taken from rust std library.
const DEFAULT_BACKLOG: u32 = 128;

/// The state of a TCP socket.
///
/// This represents the various states a socket can be in during the
/// activities of binding, listening, accepting, and connecting.
pub(crate) enum TcpState {
    /// The initial state for a newly-created socket.
    Default(tokio::net::TcpSocket),

    /// Binding started via `start_bind`.
    BindStarted(tokio::net::TcpSocket),

    /// Binding finished via `finish_bind`. The socket has an address but
    /// is not yet listening for connections.
    Bound(tokio::net::TcpSocket),

    /// Listening started via `listen_start`.
    ListenStarted(tokio::net::TcpSocket),

    /// The socket is now listening and waiting for an incoming connection.
    Listening {
        listener: tokio::net::TcpListener,
        pending_accept: Option<io::Result<tokio::net::TcpStream>>,
    },

    /// An outgoing connection is started via `start_connect`.
    Connecting(Pin<Box<dyn Future<Output = io::Result<tokio::net::TcpStream>> + Send>>),

    /// An outgoing connection is ready to be established.
    ConnectReady(io::Result<tokio::net::TcpStream>),

    /// An outgoing connection has been established.
    Connected(Arc<tokio::net::TcpStream>),

    Closed,
}

/// A host TCP socket, plus associated bookkeeping.
///
/// The inner state is wrapped in an Arc because the same underlying socket is
/// used for implementing the stream types.
pub struct TcpSocket {
    /// The current state in the bind/listen/accept/connect progression.
    pub(crate) tcp_state: TcpState,

    /// The desired listen queue size.
    pub(crate) listen_backlog_size: u32,

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
    pub(crate) fn new(stream: Arc<tokio::net::TcpStream>) -> Self {
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
        with_ambient_tokio_runtime(|| {
            let (socket, family) = match family {
                AddressFamily::Ipv4 => {
                    let socket = tokio::net::TcpSocket::new_v4()?;
                    (socket, SocketAddressFamily::Ipv4)
                }
                AddressFamily::Ipv6 => {
                    let socket = tokio::net::TcpSocket::new_v6()?;
                    sockopt::set_ipv6_v6only(&socket, true)?;
                    (socket, SocketAddressFamily::Ipv6)
                }
            };

            Self::from_state(TcpState::Default(socket), family)
        })
    }

    /// Create a `TcpSocket` from an existing socket.
    pub(crate) fn from_state(state: TcpState, family: SocketAddressFamily) -> io::Result<Self> {
        Ok(Self {
            tcp_state: state,
            listen_backlog_size: DEFAULT_BACKLOG,
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

    pub(crate) fn as_std_view(&self) -> SocketResult<SocketlikeView<'_, std::net::TcpStream>> {
        use crate::preview2::bindings::sockets::network::ErrorCode;

        match &self.tcp_state {
            TcpState::Default(socket) | TcpState::Bound(socket) => {
                Ok(socket.as_socketlike_view::<std::net::TcpStream>())
            }
            TcpState::Connected(stream) => Ok(stream.as_socketlike_view::<std::net::TcpStream>()),
            TcpState::Listening { listener, .. } => {
                Ok(listener.as_socketlike_view::<std::net::TcpStream>())
            }

            TcpState::BindStarted(..)
            | TcpState::ListenStarted(..)
            | TcpState::Connecting(..)
            | TcpState::ConnectReady(..)
            | TcpState::Closed => Err(ErrorCode::InvalidState.into()),
        }
    }
}

#[async_trait::async_trait]
impl Subscribe for TcpSocket {
    async fn ready(&mut self) {
        match &mut self.tcp_state {
            TcpState::Default(..)
            | TcpState::BindStarted(..)
            | TcpState::Bound(..)
            | TcpState::ListenStarted(..)
            | TcpState::ConnectReady(..)
            | TcpState::Closed
            | TcpState::Connected(..) => {
                // No async operation in progress.
            }
            TcpState::Connecting(future) => {
                self.tcp_state = TcpState::ConnectReady(future.as_mut().await);
            }
            TcpState::Listening {
                listener,
                pending_accept,
            } => match pending_accept {
                Some(_) => {}
                None => {
                    let result = futures::future::poll_fn(|cx| {
                        listener.poll_accept(cx).map_ok(|(stream, _)| stream)
                    })
                    .await;
                    *pending_accept = Some(result);
                }
            },
        }
    }
}

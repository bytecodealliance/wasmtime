use crate::preview2::{HostInputStream, HostOutputStream, StreamState, Table, TableError};
use bytes::{Bytes, BytesMut};
use cap_net_ext::{AddressFamily, Blocking, TcpListenerExt};
use cap_std::net::{TcpListener, TcpStream};
use io_lifetimes::raw::{FromRawSocketlike, IntoRawSocketlike};
use io_lifetimes::AsSocketlike;
use std::io;
use std::sync::Arc;
use system_interface::io::IoExt;
use tokio::io::Interest;

/// The state of a TCP socket.
///
/// This represents the various states a socket can be in during the
/// activities of binding, listening, accepting, and connecting.
pub(crate) enum HostTcpState {
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
pub(crate) struct HostTcpSocket {
    /// The part of a `HostTcpSocket` which is reference-counted so that we
    /// can pass it to async tasks.
    pub(crate) inner: Arc<HostTcpSocketInner>,

    /// The current state in the bind/listen/accept/connect progression.
    pub(crate) tcp_state: HostTcpState,
    
    /// The desired listen queue size.
    pub(crate) listen_backlog_size: i32,
}

/// The inner reference-counted state of a `HostTcpSocket`.
pub(crate) struct HostTcpSocketInner {
    pub(crate) tcp_socket: tokio::net::TcpStream,
}

impl HostTcpSocket {

    // The following DEFAULT_BACKLOG logic is from
    // <https://github.com/rust-lang/rust/blob/master/library/std/src/sys_common/net.rs>
    // at revision defa2456246a8272ceace9c1cdccdf2e4c36175e.

    // The 3DS doesn't support a big connection backlog. Sometimes
    // it allows up to about 37, but other times it doesn't even
    // accept 32. There may be a global limitation causing this.
    #[cfg(target_os = "horizon")]
    const DEFAULT_BACKLOG: i32 = 20;

    // The default for all other platforms
    #[cfg(not(target_os = "horizon"))]
    const DEFAULT_BACKLOG: i32 = 128;



    /// Create a new socket in the given family.
    pub fn new(family: AddressFamily) -> io::Result<Self> {
        // Create a new host socket and set it to non-blocking, which is needed
        // by our async implementation.
        let tcp_socket = TcpListener::new(family, Blocking::No)?;

        let std_socket =
            unsafe { std::net::TcpStream::from_raw_socketlike(tcp_socket.into_raw_socketlike()) };

        let tokio_tcp_socket = crate::preview2::with_ambient_tokio_runtime(|| {
            tokio::net::TcpStream::try_from(std_socket).unwrap()
        });

        Ok(Self {
            inner: Arc::new(HostTcpSocketInner {
                tcp_socket: tokio_tcp_socket,
            }),
            tcp_state: HostTcpState::Default,
            listen_backlog_size: Self::DEFAULT_BACKLOG,
        })
    }

    /// Create a `HostTcpSocket` from an existing socket.
    ///
    /// The socket must be in non-blocking mode.
    pub fn from_tcp_stream(tcp_socket: cap_std::net::TcpStream) -> io::Result<Self> {
        let fd = rustix::fd::OwnedFd::from(tcp_socket);
        let tcp_socket = TcpListener::from(fd);

        let std_tcp_socket =
            unsafe { std::net::TcpStream::from_raw_socketlike(tcp_socket.into_raw_socketlike()) };
        let tokio_tcp_socket = crate::preview2::with_ambient_tokio_runtime(|| {
            tokio::net::TcpStream::try_from(std_tcp_socket).unwrap()
        });

        Ok(Self {
            inner: Arc::new(HostTcpSocketInner {
                tcp_socket: tokio_tcp_socket,
            }),
            tcp_state: HostTcpState::Default,
            listen_backlog_size: Self::DEFAULT_BACKLOG,
        })
    }

    pub fn tcp_socket(&self) -> &tokio::net::TcpStream {
        self.inner.tcp_socket()
    }

    pub fn clone_inner(&self) -> Arc<HostTcpSocketInner> {
        Arc::clone(&self.inner)
    }
}

impl HostTcpSocketInner {
    pub fn tcp_socket(&self) -> &tokio::net::TcpStream {
        let tcp_socket = &self.tcp_socket;

        tcp_socket
    }
}

#[async_trait::async_trait]
impl HostInputStream for Arc<HostTcpSocketInner> {
    fn read(&mut self, size: usize) -> anyhow::Result<(Bytes, StreamState)> {
        if size == 0 {
            return Ok((Bytes::new(), StreamState::Open));
        }
        let mut buf = BytesMut::zeroed(size);
        let socket = self.tcp_socket();
        let r = socket.try_io(Interest::READABLE, || {
            socket.as_socketlike_view::<TcpStream>().read(&mut buf)
        });
        let (n, state) = read_result(r)?;
        buf.truncate(n);
        Ok((buf.freeze(), state))
    }

    async fn ready(&mut self) -> anyhow::Result<()> {
        self.tcp_socket.readable().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl HostOutputStream for Arc<HostTcpSocketInner> {
    fn write(&mut self, buf: Bytes) -> anyhow::Result<(usize, StreamState)> {
        if buf.is_empty() {
            return Ok((0, StreamState::Open));
        }
        let socket = self.tcp_socket();
        let r = socket.try_io(Interest::WRITABLE, || {
            socket.as_socketlike_view::<TcpStream>().write(buf.as_ref())
        });
        let (n, state) = write_result(r)?;
        Ok((n, state))
    }

    async fn ready(&mut self) -> anyhow::Result<()> {
        self.tcp_socket.writable().await?;
        Ok(())
    }
}

pub(crate) trait TableTcpSocketExt {
    fn push_tcp_socket(&mut self, tcp_socket: HostTcpSocket) -> Result<u32, TableError>;
    fn delete_tcp_socket(&mut self, fd: u32) -> Result<HostTcpSocket, TableError>;
    fn is_tcp_socket(&self, fd: u32) -> bool;
    fn get_tcp_socket(&self, fd: u32) -> Result<&HostTcpSocket, TableError>;
    fn get_tcp_socket_mut(&mut self, fd: u32) -> Result<&mut HostTcpSocket, TableError>;
}

impl TableTcpSocketExt for Table {
    fn push_tcp_socket(&mut self, tcp_socket: HostTcpSocket) -> Result<u32, TableError> {
        self.push(Box::new(tcp_socket))
    }
    fn delete_tcp_socket(&mut self, fd: u32) -> Result<HostTcpSocket, TableError> {
        self.delete(fd)
    }
    fn is_tcp_socket(&self, fd: u32) -> bool {
        self.is::<HostTcpSocket>(fd)
    }
    fn get_tcp_socket(&self, fd: u32) -> Result<&HostTcpSocket, TableError> {
        self.get(fd)
    }
    fn get_tcp_socket_mut(&mut self, fd: u32) -> Result<&mut HostTcpSocket, TableError> {
        self.get_mut(fd)
    }
}

pub(crate) fn read_result(r: io::Result<usize>) -> io::Result<(usize, StreamState)> {
    match r {
        Ok(0) => Ok((0, StreamState::Closed)),
        Ok(n) => Ok((n, StreamState::Open)),
        Err(e)
            if e.kind() == io::ErrorKind::Interrupted || e.kind() == io::ErrorKind::WouldBlock =>
        {
            Ok((0, StreamState::Open))
        }
        Err(e) => Err(e),
    }
}

pub(crate) fn write_result(r: io::Result<usize>) -> io::Result<(usize, StreamState)> {
    match r {
        // We special-case zero-write stores ourselves, so if we get a zero
        // back from a `write`, it means the stream is closed on some
        // platforms.
        Ok(0) => Ok((0, StreamState::Closed)),
        Ok(n) => Ok((n, StreamState::Open)),
        #[cfg(not(windows))]
        Err(e) if e.raw_os_error() == Some(rustix::io::Errno::PIPE.raw_os_error()) => {
            Ok((0, StreamState::Closed))
        }
        Err(e) => Err(e),
    }
}

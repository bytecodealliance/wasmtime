use crate::preview2::{HostInputStream, HostOutputStream, StreamState, Table, TableError};
use bytes::{Bytes, BytesMut};
use cap_net_ext::{AddressFamily, Blocking, TcpListenerExt};
use cap_std::net::{TcpListener, TcpStream};
use io_lifetimes::AsSocketlike;
use std::io;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use system_interface::io::IoExt;

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
/// used for implementing the stream types. Also needed for [`spawn_blocking`].
///
/// [`spawn_blocking`]: Self::spawn_blocking
pub(crate) struct HostTcpSocket {
    /// The part of a `HostTcpSocket` which is reference-counted so that we
    /// can pass it to async tasks.
    pub(crate) inner: Arc<HostTcpSocketInner>,
}

/// The inner reference-counted state of a `HostTcpSocket`.
pub(crate) struct HostTcpSocketInner {
    /// On Unix-family platforms we can use `AsyncFd` for efficient polling.
    #[cfg(unix)]
    pub(crate) tcp_socket: tokio::io::unix::AsyncFd<cap_std::net::TcpListener>,

    /// On non-Unix, we can use plain `poll`.
    #[cfg(not(unix))]
    pub(crate) tcp_socket: cap_std::net::TcpListener,

    /// The current state in the bind/listen/accept/connect progression.
    pub(crate) tcp_state: RwLock<HostTcpState>,
}

impl HostTcpSocket {
    /// Create a new socket in the given family.
    pub fn new(family: AddressFamily) -> io::Result<Self> {
        // Create a new host socket and set it to non-blocking, which is needed
        // by our async implementation.
        let tcp_socket = TcpListener::new(family, Blocking::No)?;

        // On Unix, pack it up in an `AsyncFd` so we can efficiently poll it.
        #[cfg(unix)]
        let tcp_socket = tokio::io::unix::AsyncFd::new(tcp_socket)?;

        Ok(Self {
            inner: Arc::new(HostTcpSocketInner {
                tcp_socket,
                tcp_state: RwLock::new(HostTcpState::Default),
            }),
        })
    }

    /// Create a `HostTcpSocket` from an existing socket.
    ///
    /// The socket must be in non-blocking mode.
    pub fn from_tcp_stream(tcp_socket: cap_std::net::TcpStream) -> io::Result<Self> {
        let fd = rustix::fd::OwnedFd::from(tcp_socket);
        let tcp_socket = TcpListener::from(fd);

        // On Unix, pack it up in an `AsyncFd` so we can efficiently poll it.
        #[cfg(unix)]
        let tcp_socket = tokio::io::unix::AsyncFd::new(tcp_socket)?;

        Ok(Self {
            inner: Arc::new(HostTcpSocketInner {
                tcp_socket,
                tcp_state: RwLock::new(HostTcpState::Default),
            }),
        })
    }

    pub fn tcp_socket(&self) -> &cap_std::net::TcpListener {
        self.inner.tcp_socket()
    }

    pub fn clone_inner(&self) -> Arc<HostTcpSocketInner> {
        Arc::clone(&self.inner)
    }

    /// Acquire a reader lock for `self.tcp_state`.
    pub fn tcp_state_read_lock(&self) -> RwLockReadGuard<HostTcpState> {
        self.inner.tcp_state.read().unwrap()
    }

    /// Acquire a writer lock for `self.tcp_state`.
    pub fn tcp_state_write_lock(&self) -> RwLockWriteGuard<HostTcpState> {
        self.inner.tcp_state.write().unwrap()
    }
}

impl HostTcpSocketInner {
    pub fn tcp_socket(&self) -> &cap_std::net::TcpListener {
        let tcp_socket = &self.tcp_socket;

        // Unpack the `AsyncFd`.
        #[cfg(unix)]
        let tcp_socket = tcp_socket.get_ref();

        tcp_socket
    }

    pub fn set_state(&self, new_state: HostTcpState) {
        *self.tcp_state.write().unwrap() = new_state;
    }

    /// Spawn a task on tokio's blocking thread for performing blocking
    /// syscalls on the underlying [`cap_std::net::TcpListener`].
    #[cfg(not(unix))]
    pub(crate) async fn spawn_blocking<F, R>(self: &Arc<Self>, body: F) -> R
    where
        F: FnOnce(&cap_std::net::TcpListener) -> R + Send + 'static,
        R: Send + 'static,
    {
        let s = Arc::clone(self);
        tokio::task::spawn_blocking(move || body(s.tcp_socket()))
            .await
            .unwrap()
    }
}

#[async_trait::async_trait]
impl HostInputStream for Arc<HostTcpSocketInner> {
    fn read(&mut self, size: usize) -> anyhow::Result<(Bytes, StreamState)> {
        if size == 0 {
            return Ok((Bytes::new(), StreamState::Open));
        }
        let mut buf = BytesMut::zeroed(size);
        let r = self
            .tcp_socket()
            .as_socketlike_view::<TcpStream>()
            .read(&mut buf);
        let (n, state) = read_result(r)?;
        buf.truncate(n);
        Ok((buf.freeze(), state))
    }

    async fn ready(&mut self) -> anyhow::Result<()> {
        #[cfg(unix)]
        {
            self.tcp_socket.readable().await?.retain_ready();
            Ok(())
        }

        #[cfg(not(unix))]
        {
            self.spawn_blocking(move |tcp_socket| {
                match rustix::event::poll(
                    &mut [rustix::event::PollFd::new(
                        tcp_socket,
                        rustix::event::PollFlags::IN
                            | rustix::event::PollFlags::ERR
                            | rustix::event::PollFlags::HUP,
                    )],
                    -1,
                ) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(err.into()),
                }
            })
            .await
        }
    }
}

#[async_trait::async_trait]
impl HostOutputStream for Arc<HostTcpSocketInner> {
    fn write(&mut self, buf: Bytes) -> anyhow::Result<(usize, StreamState)> {
        if buf.is_empty() {
            return Ok((0, StreamState::Open));
        }
        let r = self
            .tcp_socket
            .as_socketlike_view::<TcpStream>()
            .write(buf.as_ref());
        let (n, state) = write_result(r)?;
        Ok((n, state))
    }

    async fn ready(&mut self) -> anyhow::Result<()> {
        #[cfg(unix)]
        {
            self.tcp_socket.writable().await?.retain_ready();
            Ok(())
        }

        #[cfg(not(unix))]
        {
            self.spawn_blocking(move |tcp_socket| {
                match rustix::event::poll(
                    &mut [rustix::event::PollFd::new(
                        tcp_socket,
                        rustix::event::PollFlags::OUT
                            | rustix::event::PollFlags::ERR
                            | rustix::event::PollFlags::HUP,
                    )],
                    -1,
                ) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(err.into()),
                }
            })
            .await
        }
    }
}

pub(crate) trait TableTcpSocketExt {
    fn push_tcp_socket(&mut self, tcp_socket: HostTcpSocket) -> Result<u32, TableError>;
    fn delete_tcp_socket(&mut self, fd: u32) -> Result<HostTcpSocket, TableError>;
    fn is_tcp_socket(&self, fd: u32) -> bool;
    fn get_tcp_socket(&self, fd: u32) -> Result<&HostTcpSocket, TableError>;
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
}

pub(crate) fn read_result(r: io::Result<usize>) -> io::Result<(usize, StreamState)> {
    match r {
        Ok(0) => Ok((0, StreamState::Closed)),
        Ok(n) => Ok((n, StreamState::Open)),
        Err(e) if e.kind() == io::ErrorKind::Interrupted => Ok((0, StreamState::Open)),
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

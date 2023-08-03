use crate::preview2::{HostInputStream, HostOutputStream, StreamState, Table, TableError};
use bytes::{Bytes, BytesMut};
use cap_net_ext::{AddressFamily, Blocking, TcpListenerExt};
use cap_std::net::{TcpListener, TcpStream};
use io_lifetimes::AsSocketlike;
use std::io;
use std::sync::Arc;
use std::sync::RwLock;
use system_interface::io::IoExt;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub(crate) enum HostTcpState {
    Default,
    Bind(JoinHandle<io::Result<()>>),
    Bound,
    Listen(JoinHandle<io::Result<()>>),
    Listening,
    Connect(JoinHandle<io::Result<()>>),
    Connected,
}

// The inner state is wrapped in an Arc because the same underlying socket is
// used for implementing the stream types. Also needed for [`spawn_blocking`].
//
// [`spawn_blocking`]: Self::spawn_blocking
pub(crate) struct HostTcpSocket(pub(crate) Arc<HostTcpSocketInner>);

pub(crate) struct HostTcpSocketInner {
    #[cfg(unix)]
    pub(crate) tcp_socket: tokio::io::unix::AsyncFd<cap_std::net::TcpListener>,

    #[cfg(not(unix))]
    pub(crate) tcp_socket: cap_std::net::TcpListener,

    pub(crate) tcp_state: RwLock<HostTcpState>,
    pub(crate) notify: Notify,
}

impl HostTcpSocket {
    pub fn new(family: AddressFamily) -> io::Result<Self> {
        let tcp_socket = TcpListener::new(family, Blocking::No)?;

        // On Unix, pack it up in an `AsyncFd` so we can efficiently poll it.
        #[cfg(unix)]
        let tcp_socket = tokio::io::unix::AsyncFd::new(tcp_socket)?;

        Ok(Self(Arc::new(HostTcpSocketInner {
            tcp_socket,
            tcp_state: RwLock::new(HostTcpState::Default),
            notify: Notify::new(),
        })))
    }

    pub fn from_tcp_stream(tcp_socket: cap_std::net::TcpStream) -> io::Result<Self> {
        let fd = rustix::fd::OwnedFd::from(tcp_socket);
        let tcp_socket = TcpListener::from(fd);

        // On Unix, pack it up in an `AsyncFd` so we can efficiently poll it.
        #[cfg(unix)]
        let tcp_socket = tokio::io::unix::AsyncFd::new(tcp_socket)?;

        Ok(Self(Arc::new(HostTcpSocketInner {
            tcp_socket,
            tcp_state: RwLock::new(HostTcpState::Default),
            notify: Notify::new(),
        })))
    }

    pub fn tcp_socket(&self) -> &cap_std::net::TcpListener {
        let tcp_socket = &self.0.tcp_socket;

        // Unpack the `AsyncFd`.
        #[cfg(unix)]
        let tcp_socket = tcp_socket.get_ref();

        tcp_socket
    }

    pub fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }

    /// Spawn a task on tokio's blocking thread for performing blocking
    /// syscalls on the underlying [`cap_std::net::TcpListener`].
    #[cfg(not(unix))]
    pub(crate) async fn spawn_blocking<F, R>(&self, body: F) -> R
    where
        F: FnOnce(&cap_std::net::TcpListener) -> R + Send + 'static,
        R: Send + 'static,
    {
        let s = self.clone();
        tokio::task::spawn_blocking(move || body(s.tcp_socket()))
            .await
            .unwrap()
    }
}

#[async_trait::async_trait]
impl HostInputStream for HostTcpSocket {
    fn read(&mut self, size: usize) -> anyhow::Result<(Bytes, StreamState)> {
        let mut buf = BytesMut::zeroed(size);
        let r = self
            .0
            .tcp_socket
            .as_socketlike_view::<TcpStream>()
            .read(&mut buf);
        let (n, state) = read_result(r)?;
        buf.truncate(n);
        Ok((buf.freeze(), state))
    }

    async fn ready(&mut self) -> anyhow::Result<()> {
        #[cfg(unix)]
        self.0.tcp_socket.readable().await?.retain_ready();

        #[cfg(not(unix))]
        {
            self.spawn_blocking(move |tcp_socket| {
                match rustix::event::poll(
                    &mut [rustix::event::PollFd::new(
                        tcp_socket,
                        rustix::event::PollFlags::IN,
                    )],
                    -1,
                ) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(err.into()),
                }
            })
            .await
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl HostOutputStream for HostTcpSocket {
    fn write(&mut self, buf: Bytes) -> anyhow::Result<(usize, StreamState)> {
        let r = self
            .0
            .tcp_socket
            .as_socketlike_view::<TcpStream>()
            .write(buf.as_ref());
        let (n, state) = write_result(r)?;
        Ok((n, state))
    }

    async fn ready(&mut self) -> anyhow::Result<()> {
        #[cfg(unix)]
        self.0.tcp_socket.writable().await?.retain_ready();

        #[cfg(not(unix))]
        {
            self.spawn_blocking(move |tcp_socket| {
                match rustix::event::poll(
                    &mut [rustix::event::PollFd::new(
                        tcp_socket,
                        rustix::event::PollFlags::OUT,
                    )],
                    -1,
                ) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(err.into()),
                }
            })
            .await
        }
        Ok(())
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

pub(crate) fn read_result(
    r: Result<usize, std::io::Error>,
) -> Result<(usize, StreamState), std::io::Error> {
    match r {
        Ok(0) => Ok((0, StreamState::Closed)),
        Ok(n) => Ok((n, StreamState::Open)),
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Ok((0, StreamState::Open)),
        Err(e) => Err(e),
    }
}

pub(crate) fn write_result(
    r: Result<usize, std::io::Error>,
) -> Result<(usize, StreamState), std::io::Error> {
    match r {
        Ok(0) => Ok((0, StreamState::Closed)),
        Ok(n) => Ok((n, StreamState::Open)),
        Err(e) => Err(e),
    }
}

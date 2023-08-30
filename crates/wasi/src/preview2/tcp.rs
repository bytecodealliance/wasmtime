use crate::preview2::{AbortOnDropJoinHandle, StreamState, Table, TableError};
use cap_net_ext::{AddressFamily, Blocking, TcpListenerExt};
use cap_std::net::TcpListener;
use io_lifetimes::raw::{FromRawSocketlike, IntoRawSocketlike};
use std::io;
use std::sync::Arc;

use super::{HostInputStream, HostOutputStream, OutputStreamError};

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
    pub(crate) inner: HostTcpSocketInner,

    /// The current state in the bind/listen/accept/connect progression.
    pub(crate) tcp_state: HostTcpState,
}

pub(crate) struct HostTcpSocketInner {
    stream: Arc<tokio::net::TcpStream>,
}

impl HostTcpSocketInner {
    fn new(stream: cap_std::net::TcpListener) -> Self {
        let fd = stream.into_raw_socketlike();
        let stream = unsafe { std::net::TcpStream::from_raw_socketlike(fd) };
        let stream = tokio::net::TcpStream::try_from(stream).unwrap();

        Self {
            stream: Arc::new(stream),
        }
    }

    pub(crate) fn tcp_socket(&self) -> &tokio::net::TcpStream {
        &self.stream
    }

    pub(crate) fn clone(&self) -> Self {
        Self {
            stream: Arc::clone(&self.stream),
        }
    }
}

#[async_trait::async_trait]
impl HostInputStream for HostTcpSocketInner {
    fn read(&mut self, size: usize) -> Result<(bytes::Bytes, StreamState), anyhow::Error> {
        if size == 0 {
            return Ok((bytes::Bytes::new(), StreamState::Open));
        }

        let mut buf = bytes::BytesMut::with_capacity(size);
        let n = match self.stream.try_read_buf(&mut buf) {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => 0,
            Err(_) => {
                // FIXME: this is a closed stream, but we need to record it for future calls to
                // ready
                return Ok((bytes::Bytes::new(), StreamState::Closed));
            }
        };

        buf.truncate(n);
        Ok((buf.freeze(), StreamState::Open))
    }

    async fn ready(&mut self) -> Result<(), anyhow::Error> {
        self.stream.readable().await?;
        Ok(())
    }
}

const SOCKET_READY_SIZE: usize = 1024 * 1024 * 1024;

pub(crate) struct TcpWriteStream {
    stream: HostTcpSocketInner,
    write_handle: Option<AbortOnDropJoinHandle<anyhow::Result<()>>>,
}

impl TcpWriteStream {
    pub(crate) fn new(stream: HostTcpSocketInner) -> Self {
        Self {
            stream,
            write_handle: None,
        }
    }

    /// Write `bytes` in a background task, remembering the task handle for use in a future call to
    /// `write_ready`
    fn background_write(&mut self, mut bytes: bytes::Bytes) {
        assert!(self.write_handle.is_none());

        let stream = self.stream.clone();
        self.write_handle.replace(
            tokio::spawn(async move {
                while !bytes.is_empty() {
                    stream.tcp_socket().writable().await?;
                    let n = stream.tcp_socket().try_write(&bytes)?;
                    let _ = bytes.split_to(n);
                }

                Ok(())
            })
            .into(),
        );
    }
}

#[async_trait::async_trait]
impl HostOutputStream for TcpWriteStream {
    fn write(&mut self, mut bytes: bytes::Bytes) -> Result<(), OutputStreamError> {
        while !bytes.is_empty() {
            match self.stream.tcp_socket().try_write(&bytes) {
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

    async fn write_ready(&mut self) -> Result<usize, OutputStreamError> {
        if self.write_handle.is_some() {
            self.write_handle
                .as_mut()
                .unwrap()
                .await
                .map_err(|e| OutputStreamError::Trap(e.into()))?
                .map_err(|e| OutputStreamError::LastOperationFailed(e.into()))?;

            // Only clear out the write handle once the task has exited, to ensure that
            // `write_ready` remains cancel-safe.
            self.write_handle = None;
        }

        self.stream
            .tcp_socket()
            .writable()
            .await
            .map_err(|e| OutputStreamError::LastOperationFailed(e.into()))?;

        Ok(SOCKET_READY_SIZE)
    }
}

impl HostTcpSocket {
    /// Create a new socket in the given family.
    pub fn new(family: AddressFamily) -> io::Result<Self> {
        // Create a new host socket and set it to non-blocking, which is needed
        // by our async implementation.
        let tcp_socket = TcpListener::new(family, Blocking::No)?;

        Ok(Self {
            inner: HostTcpSocketInner::new(tcp_socket),
            tcp_state: HostTcpState::Default,
        })
    }

    /// Create a `HostTcpSocket` from an existing socket.
    ///
    /// The socket must be in non-blocking mode.
    pub fn from_tcp_stream(tcp_socket: cap_std::net::TcpStream) -> io::Result<Self> {
        let tcp_socket = TcpListener::from(rustix::fd::OwnedFd::from(tcp_socket));

        Ok(Self {
            inner: HostTcpSocketInner::new(tcp_socket),
            tcp_state: HostTcpState::Default,
        })
    }

    pub fn tcp_socket(&self) -> &tokio::net::TcpStream {
        self.inner.tcp_socket()
    }

    /// Create the input/output stream pair for a tcp socket.
    pub fn as_split(&self) -> (Box<impl HostInputStream>, Box<impl HostOutputStream>) {
        let input = Box::new(self.inner.clone());
        let output = Box::new(TcpWriteStream::new(self.inner.clone()));
        (input, output)
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

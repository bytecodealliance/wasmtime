use super::{HostInputStream, HostOutputStream, OutputStreamError};
use crate::preview2::bindings::sockets::tcp::TcpSocket;
use crate::preview2::{
    with_ambient_tokio_runtime, AbortOnDropJoinHandle, StreamState, Table, TableError,
};
use cap_net_ext::{AddressFamily, Blocking, TcpListenerExt};
use cap_std::net::TcpListener;
use io_lifetimes::raw::{FromRawSocketlike, IntoRawSocketlike};
use std::io;
use std::sync::Arc;
use wasmtime::component::Resource;

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
pub(crate) struct HostTcpSocketState {
    /// The part of a `HostTcpSocketState` which is reference-counted so that we
    /// can pass it to async tasks.
    pub(crate) inner: Arc<tokio::net::TcpStream>,

    /// The current state in the bind/listen/accept/connect progression.
    pub(crate) tcp_state: HostTcpState,
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

    async fn ready(&mut self) -> Result<(), anyhow::Error> {
        if self.closed {
            return Ok(());
        }
        self.stream.readable().await?;
        Ok(())
    }
}

const SOCKET_READY_SIZE: usize = 1024 * 1024 * 1024;

pub(crate) struct TcpWriteStream {
    stream: Arc<tokio::net::TcpStream>,
    write_handle: Option<AbortOnDropJoinHandle<anyhow::Result<()>>>,
}

impl TcpWriteStream {
    pub(crate) fn new(stream: Arc<tokio::net::TcpStream>) -> Self {
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
        self.write_handle
            .replace(crate::preview2::spawn(async move {
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

#[async_trait::async_trait]
impl HostOutputStream for TcpWriteStream {
    fn write(&mut self, mut bytes: bytes::Bytes) -> Result<(), OutputStreamError> {
        if self.write_handle.is_some() {
            return Err(OutputStreamError::Trap(anyhow::anyhow!(
                "unpermitted: cannot write while background write ongoing"
            )));
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

    async fn write_ready(&mut self) -> Result<usize, OutputStreamError> {
        if let Some(handle) = &mut self.write_handle {
            handle
                .await
                .map_err(|e| OutputStreamError::LastOperationFailed(e.into()))?;

            // Only clear out the write handle once the task has exited, to ensure that
            // `write_ready` remains cancel-safe.
            self.write_handle = None;
        }

        self.stream
            .writable()
            .await
            .map_err(|e| OutputStreamError::LastOperationFailed(e.into()))?;

        Ok(SOCKET_READY_SIZE)
    }
}

impl HostTcpSocketState {
    /// Create a new socket in the given family.
    pub fn new(family: AddressFamily) -> io::Result<Self> {
        // Create a new host socket and set it to non-blocking, which is needed
        // by our async implementation.
        let tcp_listener = TcpListener::new(family, Blocking::No)?;
        Self::from_tcp_listener(tcp_listener)
    }

    /// Create a `HostTcpSocketState` from an existing socket.
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
            tcp_state: HostTcpState::Default,
        })
    }

    pub fn tcp_socket(&self) -> &tokio::net::TcpStream {
        &self.inner
    }

    /// Create the input/output stream pair for a tcp socket.
    pub fn as_split(&self) -> (Box<impl HostInputStream>, Box<impl HostOutputStream>) {
        let input = Box::new(TcpReadStream::new(self.inner.clone()));
        let output = Box::new(TcpWriteStream::new(self.inner.clone()));
        (input, output)
    }
}

pub(crate) trait TableTcpSocketExt {
    fn push_tcp_socket(
        &mut self,
        tcp_socket: HostTcpSocketState,
    ) -> Result<Resource<TcpSocket>, TableError>;
    fn delete_tcp_socket(
        &mut self,
        fd: Resource<TcpSocket>,
    ) -> Result<HostTcpSocketState, TableError>;
    fn is_tcp_socket(&self, fd: &Resource<TcpSocket>) -> bool;
    fn get_tcp_socket(&self, fd: &Resource<TcpSocket>) -> Result<&HostTcpSocketState, TableError>;
    fn get_tcp_socket_mut(
        &mut self,
        fd: &Resource<TcpSocket>,
    ) -> Result<&mut HostTcpSocketState, TableError>;
}

impl TableTcpSocketExt for Table {
    fn push_tcp_socket(
        &mut self,
        tcp_socket: HostTcpSocketState,
    ) -> Result<Resource<TcpSocket>, TableError> {
        Ok(Resource::new_own(self.push(Box::new(tcp_socket))?))
    }
    fn delete_tcp_socket(
        &mut self,
        fd: Resource<TcpSocket>,
    ) -> Result<HostTcpSocketState, TableError> {
        self.delete(fd.rep())
    }
    fn is_tcp_socket(&self, fd: &Resource<TcpSocket>) -> bool {
        self.is::<HostTcpSocketState>(fd.rep())
    }
    fn get_tcp_socket(&self, fd: &Resource<TcpSocket>) -> Result<&HostTcpSocketState, TableError> {
        self.get(fd.rep())
    }
    fn get_tcp_socket_mut(
        &mut self,
        fd: &Resource<TcpSocket>,
    ) -> Result<&mut HostTcpSocketState, TableError> {
        self.get_mut(fd.rep())
    }
}

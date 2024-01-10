use crate::preview2::host::network::util;
use crate::preview2::network::SocketAddressFamily;
use crate::preview2::SocketAddrFamily;
use cap_net_ext::Blocking;
use rustix::net::sockopt;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::Interest;

/// A cross-platform WASI-compliant `TcpSocket` implementation.
pub struct SystemTcpSocket {
    pub(crate) stream: Arc<tokio::net::TcpStream>,

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

impl SystemTcpSocket {
    /// Create a new socket in the given family.
    pub fn new(family: SocketAddrFamily) -> io::Result<Self> {
        // Create a new host socket and set it to non-blocking, which is needed
        // by our async implementation.
        let fd = util::tcp_socket(family, cap_net_ext::Blocking::No)?;

        let socket_address_family = match family {
            SocketAddrFamily::V4 => SocketAddressFamily::Ipv4,
            SocketAddrFamily::V6 => SocketAddressFamily::Ipv6 {
                v6only: sockopt::get_ipv6_v6only(&fd)?,
            },
        };

        Self::from_fd(fd, socket_address_family)
    }

    fn from_fd(fd: rustix::fd::OwnedFd, family: SocketAddressFamily) -> io::Result<Self> {
        let stream = Self::setup_tokio_tcp_stream(fd)?;

        Ok(Self {
            stream: Arc::new(stream),
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
        use io_lifetimes::raw::{FromRawSocketlike, IntoRawSocketlike};

        let std_stream =
            unsafe { std::net::TcpStream::from_raw_socketlike(fd.into_raw_socketlike()) };
        super::with_ambient_tokio_runtime(|| tokio::net::TcpStream::try_from(std_stream))
    }

    pub(crate) fn split(&self) -> (SystemTcpReader, SystemTcpWriter) {
        (
            SystemTcpReader {
                inner: self.stream.clone(),
            },
            SystemTcpWriter {
                inner: self.stream.clone(),
            },
        )
    }

    pub(crate) fn try_accept(&mut self) -> io::Result<SystemTcpSocket> {
        let stream = self.stream.as_ref();
        let (client_fd, _addr) = stream.try_io(Interest::READABLE, || {
            util::tcp_accept(stream, Blocking::No)
        })?;

        Self::from_fd(client_fd, self.family)
    }
}

/// We can't just use `tokio::net::tcp::OwnedReadHalf` because we need to keep
/// access to the original TcpStream.
pub(crate) struct SystemTcpReader {
    inner: Arc<tokio::net::TcpStream>,
}

impl tokio::io::AsyncRead for SystemTcpReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        while self.inner.poll_read_ready(cx).is_ready() {
            match self.inner.try_read_buf(buf) {
                Ok(_) => return Poll::Ready(Ok(())),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Poll::Ready(Err(e)),
            }
        }

        Poll::Pending
    }
}

/// We can't just use `tokio::net::tcp::OwnedWriteHalf` because we need to keep
/// access to the original TcpStream. Also, `OwnedWriteHalf` calls `shutdown` on
/// the underlying socket, which is not what we want.
pub(crate) struct SystemTcpWriter {
    pub(crate) inner: Arc<tokio::net::TcpStream>,
}

impl tokio::io::AsyncWrite for SystemTcpWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        while self.inner.poll_write_ready(cx).is_ready() {
            match self.inner.try_write(buf) {
                Ok(n) => return Poll::Ready(Ok(n)),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Poll::Ready(Err(e)),
            }
        }

        Poll::Pending
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        // We're not managing any internal buffer so we have nothing to flush.
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        // This method is never called by the WASI wrappers.
        // And even if it was, we wouldn't want to call `shutdown` because we don't own the socket.
        Poll::Ready(Ok(()))
    }
}

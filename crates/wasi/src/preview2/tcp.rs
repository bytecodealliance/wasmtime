use crate::preview2::host::network::util;
use crate::preview2::network::SocketAddressFamily;
use crate::preview2::SocketAddrFamily;
use cap_net_ext::Blocking;
use io_lifetimes::AsSocketlike;
use rustix::io::Errno;
use rustix::net::sockopt;
use std::io;
use std::net::{Shutdown, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
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
    receive_buffer_size: Option<usize>,
    #[cfg(target_os = "macos")]
    send_buffer_size: Option<usize>,
    #[cfg(target_os = "macos")]
    hop_limit: Option<u8>,
    #[cfg(target_os = "macos")]
    keep_alive_idle_time: Option<std::time::Duration>,
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

        #[cfg(target_os = "macos")]
        {
            // Manually inherit socket options from listener. We only have to
            // do this on platforms that don't already do this automatically
            // and only if a specific value was explicitly set on the listener.

            if let Some(size) = self.receive_buffer_size {
                _ = util::set_socket_recv_buffer_size(&self.stream, size); // Ignore potential error.
            }

            if let Some(size) = self.send_buffer_size {
                _ = util::set_socket_send_buffer_size(&self.stream, size); // Ignore potential error.
            }

            // For some reason, IP_TTL is inherited, but IPV6_UNICAST_HOPS isn't.
            if let (SocketAddressFamily::Ipv6 { .. }, Some(ttl)) = (self.family, self.hop_limit) {
                _ = util::set_ipv6_unicast_hops(&self.stream, ttl); // Ignore potential error.
            }

            if let Some(value) = self.keep_alive_idle_time {
                _ = util::set_tcp_keepidle(&self.stream, value); // Ignore potential error.
            }
        }

        Self::from_fd(client_fd, self.family)
    }

    pub fn shutdown(&mut self, how: Shutdown) -> io::Result<()> {
        self.stream
            .as_socketlike_view::<std::net::TcpStream>()
            .shutdown(how)?;
        Ok(())
    }

    pub fn local_address(&self) -> io::Result<SocketAddr> {
        self.stream.local_addr()
    }

    pub fn remote_address(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    pub fn address_family(&self) -> SocketAddrFamily {
        match self.family {
            SocketAddressFamily::Ipv4 => SocketAddrFamily::V4,
            SocketAddressFamily::Ipv6 { .. } => SocketAddrFamily::V6,
        }
    }

    pub fn ipv6_only(&self) -> io::Result<bool> {
        // Instead of just calling the OS we return our own internal state, because
        // MacOS doesn't propagate the V6ONLY state on to accepted client sockets.

        match self.family {
            SocketAddressFamily::Ipv4 => Err(Errno::AFNOSUPPORT.into()),
            SocketAddressFamily::Ipv6 { v6only } => Ok(v6only),
        }
    }

    pub fn set_ipv6_only(&mut self, value: bool) -> io::Result<()> {
        match self.family {
            SocketAddressFamily::Ipv4 => Err(Errno::AFNOSUPPORT.into()),
            SocketAddressFamily::Ipv6 { .. } => {
                sockopt::set_ipv6_v6only(&self.stream, value)?;
                self.family = SocketAddressFamily::Ipv6 { v6only: value };
                Ok(())
            }
        }
    }

    pub fn keep_alive_enabled(&self) -> io::Result<bool> {
        Ok(sockopt::get_socket_keepalive(&self.stream)?)
    }

    pub fn set_keep_alive_enabled(&mut self, value: bool) -> io::Result<()> {
        Ok(sockopt::set_socket_keepalive(&self.stream, value)?)
    }

    pub fn keep_alive_idle_time(&self) -> io::Result<Duration> {
        Ok(sockopt::get_tcp_keepidle(&self.stream)?)
    }

    pub fn set_keep_alive_idle_time(&mut self, value: Duration) -> io::Result<()> {
        util::set_tcp_keepidle(&self.stream, value)?;

        #[cfg(target_os = "macos")]
        {
            self.keep_alive_idle_time = Some(value);
        }

        Ok(())
    }

    pub fn keep_alive_interval(&self) -> io::Result<Duration> {
        Ok(sockopt::get_tcp_keepintvl(&self.stream)?)
    }

    pub fn set_keep_alive_interval(&mut self, value: Duration) -> io::Result<()> {
        Ok(util::set_tcp_keepintvl(&self.stream, value)?)
    }

    pub fn keep_alive_count(&self) -> io::Result<u32> {
        Ok(sockopt::get_tcp_keepcnt(&self.stream)?)
    }

    pub fn set_keep_alive_count(&mut self, value: u32) -> io::Result<()> {
        Ok(util::set_tcp_keepcnt(&self.stream, value)?)
    }

    pub fn hop_limit(&self) -> io::Result<u8> {
        let ttl = match self.family {
            SocketAddressFamily::Ipv4 => util::get_ip_ttl(&self.stream)?,
            SocketAddressFamily::Ipv6 { .. } => util::get_ipv6_unicast_hops(&self.stream)?,
        };

        Ok(ttl)
    }

    pub fn set_hop_limit(&mut self, value: u8) -> io::Result<()> {
        match self.family {
            SocketAddressFamily::Ipv4 => util::set_ip_ttl(&self.stream, value)?,
            SocketAddressFamily::Ipv6 { .. } => util::set_ipv6_unicast_hops(&self.stream, value)?,
        }

        #[cfg(target_os = "macos")]
        {
            self.hop_limit = Some(value);
        }

        Ok(())
    }

    pub fn receive_buffer_size(&self) -> io::Result<usize> {
        Ok(util::get_socket_recv_buffer_size(&self.stream)?)
    }

    pub fn set_receive_buffer_size(&mut self, value: usize) -> io::Result<()> {
        util::set_socket_recv_buffer_size(&self.stream, value)?;

        #[cfg(target_os = "macos")]
        {
            self.receive_buffer_size = Some(value);
        }

        Ok(())
    }

    pub fn send_buffer_size(&self) -> io::Result<usize> {
        Ok(util::get_socket_send_buffer_size(&self.stream)?)
    }

    pub fn set_send_buffer_size(&mut self, value: usize) -> io::Result<()> {
        util::set_socket_send_buffer_size(&self.stream, value)?;

        #[cfg(target_os = "macos")]
        {
            self.send_buffer_size = Some(value);
        }

        Ok(())
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

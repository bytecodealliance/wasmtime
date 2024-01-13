use crate::preview2::host::network::util;
use crate::preview2::network::SocketAddressFamily;
use crate::preview2::{DynFuture, SocketAddrFamily};
use cap_net_ext::{Blocking, TcpListenerExt};
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
    stream: Arc<tokio::net::TcpStream>,

    /// The desired listen queue size. Set to None to use the system's default.
    listen_backlog_size: i32,
    is_listening: bool,

    family: SocketAddressFamily,

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
    const DEFAULT_BACKLOG_SIZE: i32 = 128;

    /// Create a new socket in the given family.
    pub fn new(family: SocketAddrFamily) -> io::Result<Self> {
        // Delegate socket creation to cap_net_ext. They handle a couple of things for us:
        // - On Windows: call WSAStartup if not done before.
        // - Set the NONBLOCK and CLOEXEC flags. Either immediately during socket creation,
        //   or afterwards using ioctl or fcntl. Exact method depends on the platform.
        let fd = rustix::fd::OwnedFd::from(cap_std::net::TcpListener::new(
            family.into(),
            cap_net_ext::Blocking::No,
        )?);

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
            listen_backlog_size: Self::DEFAULT_BACKLOG_SIZE,
            is_listening: false,
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

    #[allow(unused_variables)] // Parameters are not used on Windows
    fn set_reuseaddr(&mut self, value: bool) -> rustix::io::Result<()> {
        // When a TCP socket is closed, the system may
        // temporarily reserve that specific address+port pair in a so called
        // TIME_WAIT state. During that period, any attempt to rebind to that pair
        // will fail. Setting SO_REUSEADDR to true bypasses that behaviour. Unlike
        // the name "SO_REUSEADDR" might suggest, it does not allow multiple
        // active sockets to share the same local address.

        // On Windows that behavior is the default, so there is no need to manually
        // configure such an option. But (!), Windows _does_ have an identically
        // named socket option which allows users to "hijack" active sockets.
        // This is definitely not what we want to do here.

        // Microsoft's own documentation[1] states that we should set SO_EXCLUSIVEADDRUSE
        // instead (to the inverse value), however the github issue below[2] seems
        // to indicate that that may no longer be correct.
        // [1]: https://docs.microsoft.com/en-us/windows/win32/winsock/using-so-reuseaddr-and-so-exclusiveaddruse
        // [2]: https://github.com/python-trio/trio/issues/928

        #[cfg(not(windows))]
        sockopt::set_socket_reuseaddr(&self.stream, value)?;

        Ok(())
    }

    pub fn bind(&mut self, local_address: &SocketAddr) -> io::Result<()> {
        util::validate_unicast(&local_address)?;
        util::validate_address_family(&local_address, &self.family)?;

        // Automatically bypass the TIME_WAIT state when the user is trying
        // to bind to a specific port:
        let reuse_addr = local_address.port() > 0;

        // Unconditionally (re)set SO_REUSEADDR, even when the value is false.
        // This ensures we're not accidentally affected by any socket option
        // state left behind by a previous failed call to this method (start_bind).
        self.set_reuseaddr(reuse_addr)?;

        // Perform the OS bind call.
        rustix::net::bind(&self.stream, &local_address).map_err(|error| match error {
            // See: https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-bind#:~:text=WSAENOBUFS
            // Windows returns WSAENOBUFS when the ephemeral ports have been exhausted.
            #[cfg(windows)]
            Errno::NOBUFS => Errno::ADDRINUSE,
            _ => error,
        })?;

        Ok(())
    }

    pub fn connect(
        &mut self,
        remote_address: &SocketAddr,
    ) -> DynFuture<io::Result<(SystemTcpReader, SystemTcpWriter)>> {
        fn initiate_connect(
            me: &SystemTcpSocket,
            remote_address: &SocketAddr,
        ) -> io::Result<(SystemTcpReader, SystemTcpWriter)> {
            util::validate_unicast(&remote_address)?;
            util::validate_remote_address(&remote_address)?;
            util::validate_address_family(&remote_address, &me.family)?;

            rustix::net::connect(&me.stream, remote_address).map_err(|error| match error {
                // On POSIX, non-blocking `connect` returns `EINPROGRESS`.
                // Windows returns `WSAEWOULDBLOCK`.
                #[cfg(windows)]
                Errno::WOULDBLOCK => Errno::INPROGRESS,
                _ => error,
            })?;

            Ok((
                SystemTcpReader::new(me.stream.clone()),
                SystemTcpWriter::new(me.stream.clone()),
            ))
        }

        async fn await_connection(
            stream: Arc<tokio::net::TcpStream>,
        ) -> io::Result<(SystemTcpReader, SystemTcpWriter)> {
            stream.writable().await.unwrap();

            // Check whether the connect succeeded.
            match sockopt::get_socket_error(&stream) {
                Ok(Ok(())) => Ok((
                    SystemTcpReader::new(stream.clone()),
                    SystemTcpWriter::new(stream.clone()),
                )),
                Err(err) | Ok(Err(err)) => return Err(err.into()),
            }
        }

        match initiate_connect(self, remote_address) {
            Err(e) if Errno::from_io_error(&e) == Some(Errno::INPROGRESS) => {
                DynFuture::boxed(await_connection(self.stream.clone()))
            }
            r => DynFuture::ready(r),
        }
    }

    pub fn listen(&mut self) -> io::Result<()> {
        if self.is_listening {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Socket already listening.",
            ));
        }

        rustix::net::listen(&self.stream, self.listen_backlog_size).map_err(
            |error| match error {
                // See: https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-listen#:~:text=WSAEMFILE
                // According to the docs, `listen` can return EMFILE on Windows.
                // This is odd, because we're not trying to create a new socket
                // or file descriptor of any kind. So we rewrite it to less
                // surprising error code.
                //
                // At the time of writing, this behavior has never been experimentally
                // observed by any of the wasmtime authors, so we're relying fully
                // on Microsoft's documentation here.
                #[cfg(windows)]
                Some(Errno::MFILE) => Errno::NOBUFS.into(),

                _ => error,
            },
        )?;

        self.is_listening = true;
        Ok(())
    }

    fn try_accept(&mut self) -> io::Result<(SystemTcpSocket, SystemTcpReader, SystemTcpWriter)> {
        let stream = self.stream.as_ref();
        let client_fd = stream.try_io(Interest::READABLE, || {
            // Delegate `accept` to cap_net_ext. They set the NONBLOCK and CLOEXEC flags
            // for us. Either immediately as a flag to `accept`, or afterwards using
            // ioctl or fcntl. Exact method depends on the platform.

            let (client, _addr) = stream
                .as_socketlike_view::<cap_std::net::TcpListener>()
                .accept_with(Blocking::No)
                .map_err(|error| match Errno::from_io_error(&error) {
                    // From: https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-accept#:~:text=WSAEINPROGRESS
                    // > WSAEINPROGRESS: A blocking Windows Sockets 1.1 call is in progress,
                    // > or the service provider is still processing a callback function.
                    //
                    // wasi-sockets doesn't have an equivalent to the EINPROGRESS error,
                    // because in POSIX this error is only returned by a non-blocking
                    // `connect` and wasi-sockets has a different solution for that.
                    #[cfg(windows)]
                    Some(Errno::INPROGRESS) => Errno::INTR.into(),

                    // Normalize Linux' non-standard behavior.
                    //
                    // From https://man7.org/linux/man-pages/man2/accept.2.html:
                    // > Linux accept() passes already-pending network errors on the
                    // > new socket as an error code from accept(). This behavior
                    // > differs from other BSD socket implementations. (...)
                    #[cfg(target_os = "linux")]
                    Some(
                        Errno::CONNRESET
                        | Errno::NETRESET
                        | Errno::HOSTUNREACH
                        | Errno::HOSTDOWN
                        | Errno::NETDOWN
                        | Errno::NETUNREACH
                        | Errno::PROTO
                        | Errno::NOPROTOOPT
                        | Errno::NONET
                        | Errno::OPNOTSUPP,
                    ) => Errno::CONNABORTED.into(),

                    _ => error,
                })?;

            Ok(client.into())
        })?;

        #[cfg(target_os = "macos")]
        {
            // Manually inherit socket options from listener. We only have to
            // do this on platforms that don't already do this automatically
            // and only if a specific value was explicitly set on the listener.

            if let Some(size) = self.receive_buffer_size {
                _ = util::set_socket_recv_buffer_size(&client_fd, size); // Ignore potential error.
            }

            if let Some(size) = self.send_buffer_size {
                _ = util::set_socket_send_buffer_size(&client_fd, size); // Ignore potential error.
            }

            // For some reason, IP_TTL is inherited, but IPV6_UNICAST_HOPS isn't.
            if let (SocketAddressFamily::Ipv6 { .. }, Some(ttl)) = (self.family, self.hop_limit) {
                _ = util::set_ipv6_unicast_hops(&client_fd, ttl); // Ignore potential error.
            }

            if let Some(value) = self.keep_alive_idle_time {
                _ = util::set_tcp_keepidle(&client_fd, value); // Ignore potential error.
            }
        }

        let client = Self::from_fd(client_fd, self.family)?;
        let reader = SystemTcpReader::new(client.stream.clone());
        let writer = SystemTcpWriter::new(client.stream.clone());

        Ok((client, reader, writer))
    }

    pub fn poll_accept(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<(SystemTcpSocket, SystemTcpReader, SystemTcpWriter)>> {
        while self.stream.poll_read_ready(cx).is_ready() {
            match self.try_accept() {
                Ok(s) => return Poll::Ready(Ok(s)),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Poll::Ready(Err(e)),
            }
        }

        Poll::Pending
    }

    pub async fn accept(
        &mut self,
    ) -> io::Result<(SystemTcpSocket, SystemTcpReader, SystemTcpWriter)> {
        futures::future::poll_fn(|cx| self.poll_accept(cx)).await
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

    pub fn listen_backlog_size(&self) -> io::Result<usize> {
        Ok(self.listen_backlog_size.try_into().unwrap())
    }

    pub fn set_listen_backlog_size(&mut self, value: usize) -> io::Result<()> {
        if value == 0 {
            return Err(Errno::INVAL.into());
        }

        const MIN_BACKLOG: i32 = 1;
        const MAX_BACKLOG: i32 = i32::MAX; // OS'es will most likely limit it down even further.

        // Silently clamp backlog size. This is OK for us to do, because operating systems do this too.
        let value = value
            .try_into()
            .unwrap_or(i32::MAX)
            .clamp(MIN_BACKLOG, MAX_BACKLOG);

        if self.is_listening {
            // Try to update the backlog by calling `listen` again.
            // Not all platforms support this. We'll only update our own value
            // if the OS supports changing the backlog size after the fact.
            rustix::net::listen(&self.stream, value).map_err(|_| Errno::OPNOTSUPP)?;
        }

        self.listen_backlog_size = value;

        Ok(())
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
        if value <= Duration::ZERO {
            // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
            return Err(Errno::INVAL.into());
        }

        // Ensure that the value passed to the actual syscall never gets rounded down to 0.
        const MIN_SECS: u64 = 1;

        // Cap it at Linux' maximum, which appears to have the lowest limit across our supported platforms.
        const MAX_SECS: u64 = i16::MAX as u64;

        sockopt::set_tcp_keepidle(
            &self.stream,
            value.clamp(Duration::from_secs(MIN_SECS), Duration::from_secs(MAX_SECS)),
        )?;

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
        if value <= Duration::ZERO {
            // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
            return Err(Errno::INVAL.into());
        }

        // Ensure that any fractional value passed to the actual syscall never gets rounded down to 0.
        const MIN_SECS: u64 = 1;

        // Cap it at Linux' maximum, which appears to have the lowest limit across our supported platforms.
        const MAX_SECS: u64 = i16::MAX as u64;

        sockopt::set_tcp_keepintvl(
            &self.stream,
            value.clamp(Duration::from_secs(MIN_SECS), Duration::from_secs(MAX_SECS)),
        )?;

        Ok(())
    }

    pub fn keep_alive_count(&self) -> io::Result<u32> {
        Ok(sockopt::get_tcp_keepcnt(&self.stream)?)
    }

    pub fn set_keep_alive_count(&mut self, value: u32) -> io::Result<()> {
        if value == 0 {
            // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
            return Err(Errno::INVAL.into());
        }

        const MIN_CNT: u32 = 1;
        // Cap it at Linux' maximum, which appears to have the lowest limit across our supported platforms.
        const MAX_CNT: u32 = i8::MAX as u32;

        sockopt::set_tcp_keepcnt(&self.stream, value.clamp(MIN_CNT, MAX_CNT))?;
        Ok(())
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
pub struct SystemTcpReader {
    inner: Arc<tokio::net::TcpStream>,
}

impl SystemTcpReader {
    fn new(inner: Arc<tokio::net::TcpStream>) -> Self {
        Self { inner }
    }
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
pub struct SystemTcpWriter {
    inner: Arc<tokio::net::TcpStream>,
}

impl SystemTcpWriter {
    fn new(inner: Arc<tokio::net::TcpStream>) -> Self {
        Self { inner }
    }
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

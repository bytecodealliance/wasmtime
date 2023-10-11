use crate::preview2::tcp::{TcpSocket, TcpState};
use crate::preview2::{
    bindings::{
        io::streams::{InputStream, OutputStream},
        sockets::network::{ErrorCode, IpAddressFamily, IpSocketAddress, Network},
        sockets::tcp::{self, ShutdownType},
    },
    tcp::SocketAddressFamily,
};
use crate::preview2::{Pollable, SocketResult, WasiView};
use cap_net_ext::{Blocking, PoolExt, TcpListenerExt};
use cap_std::net::TcpListener;
use io_lifetimes::AsSocketlike;
use rustix::io::Errno;
use rustix::net::sockopt;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use tokio::io::Interest;
use wasmtime::component::Resource;

impl<T: WasiView> tcp::Host for T {}

impl<T: WasiView> crate::preview2::host::tcp::tcp::HostTcpSocket for T {
    fn start_bind(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_resource(&this)?;
        let network = table.get_resource(&network)?;
        let local_address: SocketAddr = local_address.into();

        match socket.tcp_state {
            TcpState::Default => {}
            TcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        validate_unicast(&local_address)?;
        validate_address_family(&socket, &local_address)?;

        let binder = network.pool.tcp_binder(local_address)?;

        // Perform the OS bind call.
        binder
            .bind_existing_tcp_listener(&*socket.tcp_socket().as_socketlike_view::<TcpListener>())
            .map_err(|error| match Errno::from_io_error(&error) {
                Some(Errno::AFNOSUPPORT) => ErrorCode::InvalidArgument, // Just in case our own validations weren't sufficient.
                #[cfg(windows)]
                Some(Errno::NOBUFS) => ErrorCode::AddressInUse, // Windows returns WSAENOBUFS when the ephemeral ports have been exhausted.
                _ => ErrorCode::from(error),
            })?;

        let socket = table.get_resource_mut(&this)?;
        socket.tcp_state = TcpState::BindStarted;

        Ok(())
    }

    fn finish_bind(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_resource_mut(&this)?;

        match socket.tcp_state {
            TcpState::BindStarted => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        socket.tcp_state = TcpState::Bound;

        Ok(())
    }

    fn start_connect(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        network: Resource<Network>,
        remote_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let r = {
            let socket = table.get_resource(&this)?;
            let network = table.get_resource(&network)?;
            let remote_address: SocketAddr = remote_address.into();

            match socket.tcp_state {
                TcpState::Default => {}
                TcpState::Bound
                | TcpState::Connected
                | TcpState::ConnectFailed
                | TcpState::Listening => return Err(ErrorCode::InvalidState.into()),
                TcpState::Connecting
                | TcpState::ConnectReady
                | TcpState::ListenStarted
                | TcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            }

            validate_unicast(&remote_address)?;
            validate_remote_address(&remote_address)?;
            validate_address_family(&socket, &remote_address)?;

            let connecter = network.pool.tcp_connecter(remote_address)?;

            // Do an OS `connect`. Our socket is non-blocking, so it'll either...
            {
                let view = &*socket.tcp_socket().as_socketlike_view::<TcpListener>();
                let r = connecter.connect_existing_tcp_listener(view);
                r
            }
        };

        match r {
            // succeed immediately,
            Ok(()) => {
                let socket = table.get_resource_mut(&this)?;
                socket.tcp_state = TcpState::ConnectReady;
                return Ok(());
            }
            // continue in progress,
            Err(err) if Errno::from_io_error(&err) == Some(INPROGRESS) => {}
            // or fail immediately.
            Err(err) => {
                return Err(match Errno::from_io_error(&err) {
                    Some(Errno::AFNOSUPPORT) => ErrorCode::InvalidArgument.into(), // Just in case our own validations weren't sufficient.
                    _ => err.into(),
                });
            }
        }

        let socket = table.get_resource_mut(&this)?;
        socket.tcp_state = TcpState::Connecting;

        Ok(())
    }

    fn finish_connect(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> SocketResult<(Resource<InputStream>, Resource<OutputStream>)> {
        let table = self.table_mut();
        let socket = table.get_resource_mut(&this)?;

        match socket.tcp_state {
            TcpState::ConnectReady => {}
            TcpState::Connecting => {
                // Do a `poll` to test for completion, using a timeout of zero
                // to avoid blocking.
                match rustix::event::poll(
                    &mut [rustix::event::PollFd::new(
                        socket.tcp_socket(),
                        rustix::event::PollFlags::OUT,
                    )],
                    0,
                ) {
                    Ok(0) => return Err(ErrorCode::WouldBlock.into()),
                    Ok(_) => (),
                    Err(err) => Err(err).unwrap(),
                }

                // Check whether the connect succeeded.
                match sockopt::get_socket_error(socket.tcp_socket()) {
                    Ok(Ok(())) => {}
                    Err(err) | Ok(Err(err)) => {
                        socket.tcp_state = TcpState::ConnectFailed;
                        return Err(err.into());
                    }
                }
            }
            _ => return Err(ErrorCode::NotInProgress.into()),
        };

        socket.tcp_state = TcpState::Connected;
        let (input, output) = socket.as_split();
        let input_stream = self.table_mut().push_child_resource(input, &this)?;
        let output_stream = self.table_mut().push_child_resource(output, &this)?;

        Ok((input_stream, output_stream))
    }

    fn start_listen(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_resource_mut(&this)?;

        match socket.tcp_state {
            TcpState::Bound => {}
            TcpState::Default
            | TcpState::Connected
            | TcpState::ConnectFailed
            | TcpState::Listening => return Err(ErrorCode::InvalidState.into()),
            TcpState::ListenStarted
            | TcpState::Connecting
            | TcpState::ConnectReady
            | TcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
        }

        socket
            .tcp_socket()
            .as_socketlike_view::<TcpListener>()
            .listen(socket.listen_backlog_size)
            .map_err(|error| match Errno::from_io_error(&error) {
                #[cfg(windows)]
                Some(Errno::MFILE) => ErrorCode::OutOfMemory, // We're not trying to create a new socket. Rewrite it to less surprising error code.
                _ => ErrorCode::from(error),
            })?;

        socket.tcp_state = TcpState::ListenStarted;

        Ok(())
    }

    fn finish_listen(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_resource_mut(&this)?;

        match socket.tcp_state {
            TcpState::ListenStarted => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        socket.tcp_state = TcpState::Listening;

        Ok(())
    }

    fn accept(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> SocketResult<(
        Resource<tcp::TcpSocket>,
        Resource<InputStream>,
        Resource<OutputStream>,
    )> {
        let table = self.table();
        let socket = table.get_resource(&this)?;

        match socket.tcp_state {
            TcpState::Listening => {}
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        // Do the OS accept call.
        let tcp_socket = socket.tcp_socket();
        let (connection, _addr) = tcp_socket
            .try_io(Interest::READABLE, || {
                tcp_socket
                    .as_socketlike_view::<TcpListener>()
                    .accept_with(Blocking::No)
            })
            .map_err(|error| match Errno::from_io_error(&error) {
                #[cfg(windows)]
                Some(Errno::INPROGRESS) => ErrorCode::WouldBlock, // "A blocking Windows Sockets 1.1 call is in progress, or the service provider is still processing a callback function."

                // Normalize Linux' non-standard behavior.
                // "Linux accept() passes already-pending network errors on the new socket as an error code from accept(). This behavior differs from other BSD socket implementations."
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
                ) => ErrorCode::ConnectionAborted,

                _ => ErrorCode::from(error),
            })?;

        #[cfg(target_os = "macos")]
        {
            // Manually inherit socket options from listener. We only have to
            // do this on platforms that don't already do this automatically
            // and only if a specific value was explicitly set on the listener.

            if let Some(size) = socket.receive_buffer_size {
                _ = sockopt::set_socket_recv_buffer_size(&connection, size); // Ignore potential error.
            }

            if let Some(size) = socket.send_buffer_size {
                _ = sockopt::set_socket_send_buffer_size(&connection, size); // Ignore potential error.
            }

            // For some reason, IP_TTL is inherited, but IPV6_UNICAST_HOPS isn't.
            if let (SocketAddressFamily::Ipv6 { .. }, Some(ttl)) = (socket.family, socket.hop_limit)
            {
                _ = sockopt::set_ipv6_unicast_hops(&connection, Some(ttl));
                // Ignore potential error.
            }
        }

        let mut tcp_socket = TcpSocket::from_tcp_stream(connection, socket.family)?;

        // Mark the socket as connected so that we can exit early from methods like `start-bind`.
        tcp_socket.tcp_state = TcpState::Connected;

        let (input, output) = tcp_socket.as_split();
        let output: OutputStream = output;

        let tcp_socket = self.table_mut().push_resource(tcp_socket)?;
        let input_stream = self.table_mut().push_child_resource(input, &tcp_socket)?;
        let output_stream = self.table_mut().push_child_resource(output, &tcp_socket)?;

        Ok((tcp_socket, input_stream, output_stream))
    }

    fn local_address(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get_resource(&this)?;

        match socket.tcp_state {
            TcpState::Default => return Err(ErrorCode::InvalidState.into()),
            TcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => {}
        }

        let addr = socket
            .tcp_socket()
            .as_socketlike_view::<std::net::TcpStream>()
            .local_addr()?;
        Ok(addr.into())
    }

    fn remote_address(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get_resource(&this)?;

        match socket.tcp_state {
            TcpState::Connected => {}
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        let addr = socket
            .tcp_socket()
            .as_socketlike_view::<std::net::TcpStream>()
            .peer_addr()?;
        Ok(addr.into())
    }

    fn address_family(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> Result<IpAddressFamily, anyhow::Error> {
        let table = self.table();
        let socket = table.get_resource(&this)?;

        match socket.family {
            SocketAddressFamily::Ipv4 => Ok(IpAddressFamily::Ipv4),
            SocketAddressFamily::Ipv6 { .. } => Ok(IpAddressFamily::Ipv6),
        }
    }

    fn ipv6_only(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<bool> {
        let table = self.table();
        let socket = table.get_resource(&this)?;

        // Instead of just calling the OS we return our own internal state, because
        // MacOS doesn't propogate the V6ONLY state on to accepted client sockets.

        match socket.family {
            SocketAddressFamily::Ipv4 => Err(ErrorCode::NotSupported.into()),
            SocketAddressFamily::Ipv6 { v6only } => Ok(v6only),
        }
    }

    fn set_ipv6_only(&mut self, this: Resource<tcp::TcpSocket>, value: bool) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_resource_mut(&this)?;

        match socket.family {
            SocketAddressFamily::Ipv4 => Err(ErrorCode::NotSupported.into()),
            SocketAddressFamily::Ipv6 { .. } => match socket.tcp_state {
                TcpState::Default => {
                    sockopt::set_ipv6_v6only(socket.tcp_socket(), value)?;
                    socket.family = SocketAddressFamily::Ipv6 { v6only: value };
                    Ok(())
                }
                TcpState::BindStarted => Err(ErrorCode::ConcurrencyConflict.into()),
                _ => Err(ErrorCode::InvalidState.into()),
            },
        }
    }

    fn set_listen_backlog_size(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        const MIN_BACKLOG: i32 = 1;
        const MAX_BACKLOG: i32 = i32::MAX; // OS'es will most likely limit it down even further.

        let table = self.table_mut();
        let socket = table.get_resource_mut(&this)?;

        // Silently clamp backlog size. This is OK for us to do, because operating systems do this too.
        let value = value
            .try_into()
            .unwrap_or(i32::MAX)
            .clamp(MIN_BACKLOG, MAX_BACKLOG);

        match socket.tcp_state {
            TcpState::Default | TcpState::BindStarted | TcpState::Bound => {
                // Socket not listening yet. Stash value for first invocation to `listen`.
                socket.listen_backlog_size = Some(value);

                Ok(())
            }
            TcpState::Listening => {
                // Try to update the backlog by calling `listen` again.
                // Not all platforms support this. We'll only update our own value if the OS supports changing the backlog size after the fact.

                rustix::net::listen(socket.tcp_socket(), value)
                    .map_err(|_| ErrorCode::NotSupported)?;

                socket.listen_backlog_size = Some(value);

                Ok(())
            }
            TcpState::Connected | TcpState::ConnectFailed => Err(ErrorCode::InvalidState.into()),
            TcpState::Connecting | TcpState::ConnectReady | TcpState::ListenStarted => {
                Err(ErrorCode::ConcurrencyConflict.into())
            }
        }
    }

    fn keep_alive(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<bool> {
        let table = self.table();
        let socket = table.get_resource(&this)?;
        Ok(sockopt::get_socket_keepalive(socket.tcp_socket())?)
    }

    fn set_keep_alive(&mut self, this: Resource<tcp::TcpSocket>, value: bool) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_resource(&this)?;
        Ok(sockopt::set_socket_keepalive(socket.tcp_socket(), value)?)
    }

    fn no_delay(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<bool> {
        let table = self.table();
        let socket = table.get_resource(&this)?;
        Ok(sockopt::get_tcp_nodelay(socket.tcp_socket())?)
    }

    fn set_no_delay(&mut self, this: Resource<tcp::TcpSocket>, value: bool) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_resource(&this)?;
        Ok(sockopt::set_tcp_nodelay(socket.tcp_socket(), value)?)
    }

    fn unicast_hop_limit(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u8> {
        let table = self.table();
        let socket = table.get_resource(&this)?;

        let ttl = match socket.family {
            SocketAddressFamily::Ipv4 => sockopt::get_ip_ttl(socket.tcp_socket())?
                .try_into()
                .unwrap(),
            SocketAddressFamily::Ipv6 { .. } => {
                sockopt::get_ipv6_unicast_hops(socket.tcp_socket())?
            }
        };

        Ok(ttl)
    }

    fn set_unicast_hop_limit(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u8,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_resource_mut(&this)?;

        if value == 0 {
            // A well-behaved IP application should never send out new packets with TTL 0.
            // We validate the value ourselves because OS'es are not consistent in this.
            // On Linux the validation is even inconsistent between their IPv4 and IPv6 implementation.
            return Err(ErrorCode::InvalidArgument.into());
        }

        match socket.family {
            SocketAddressFamily::Ipv4 => sockopt::set_ip_ttl(socket.tcp_socket(), value.into())?,
            SocketAddressFamily::Ipv6 { .. } => {
                sockopt::set_ipv6_unicast_hops(socket.tcp_socket(), Some(value))?
            }
        }

        #[cfg(target_os = "macos")]
        {
            socket.hop_limit = Some(value);
        }

        Ok(())
    }

    fn receive_buffer_size(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get_resource(&this)?;

        let value = sockopt::get_socket_recv_buffer_size(socket.tcp_socket())? as u64;
        Ok(normalize_getsockopt_buffer_size(value))
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_resource_mut(&this)?;
        let value = normalize_setsockopt_buffer_size(value);

        match sockopt::set_socket_recv_buffer_size(socket.tcp_socket(), value) {
            // Most platforms (Linux, Windows, Fuchsia, Solaris, Illumos, Haiku, ESP-IDF, ..and more?) treat the value
            // passed to SO_SNDBUF/SO_RCVBUF as a performance tuning hint and silently clamp the input if it exceeds
            // their capability.
            // As far as I can see, only the *BSD family views this option as a hard requirement and fails when the
            // value is out of range. We normalize this behavior in favor of the more commonly understood
            // "performance hint" semantics. In other words; even ENOBUFS is "Ok".
            // A future improvement could be to query the corresponding sysctl on *BSD platforms and clamp the input
            // `size` ourselves, to completely close the gap with other platforms.
            Err(Errno::NOBUFS) => Ok(()),
            r => r,
        }?;

        #[cfg(target_os = "macos")]
        {
            socket.receive_buffer_size = Some(value);
        }

        Ok(())
    }

    fn send_buffer_size(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get_resource(&this)?;

        let value = sockopt::get_socket_send_buffer_size(socket.tcp_socket())? as u64;
        Ok(normalize_getsockopt_buffer_size(value))
    }

    fn set_send_buffer_size(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_resource_mut(&this)?;
        let value = normalize_setsockopt_buffer_size(value);

        match sockopt::set_socket_send_buffer_size(socket.tcp_socket(), value) {
            Err(Errno::NOBUFS) => Ok(()), // See `set_receive_buffer_size`
            r => r,
        }?;

        #[cfg(target_os = "macos")]
        {
            socket.send_buffer_size = Some(value);
        }

        Ok(())
    }

    fn subscribe(&mut self, this: Resource<tcp::TcpSocket>) -> anyhow::Result<Resource<Pollable>> {
        crate::preview2::poll::subscribe(self.table_mut(), this)
    }

    fn shutdown(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        shutdown_type: ShutdownType,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_resource(&this)?;

        match socket.tcp_state {
            TcpState::Connected => {}
            TcpState::Connecting | TcpState::ConnectReady => {
                return Err(ErrorCode::ConcurrencyConflict.into())
            }
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        let how = match shutdown_type {
            ShutdownType::Receive => std::net::Shutdown::Read,
            ShutdownType::Send => std::net::Shutdown::Write,
            ShutdownType::Both => std::net::Shutdown::Both,
        };

        socket
            .tcp_socket()
            .as_socketlike_view::<std::net::TcpStream>()
            .shutdown(how)?;
        Ok(())
    }

    fn drop(&mut self, this: Resource<tcp::TcpSocket>) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete_resource(this)?;
        drop(dropped);

        Ok(())
    }
}

// On POSIX, non-blocking TCP socket `connect` uses `EINPROGRESS`.
// <https://pubs.opengroup.org/onlinepubs/9699919799/functions/connect.html>
#[cfg(not(windows))]
const INPROGRESS: Errno = Errno::INPROGRESS;

// On Windows, non-blocking TCP socket `connect` uses `WSAEWOULDBLOCK`.
// <https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-connect>
#[cfg(windows)]
const INPROGRESS: Errno = Errno::WOULDBLOCK;

fn validate_unicast(addr: &SocketAddr) -> SocketResult<()> {
    match to_canonical(&addr.ip()) {
        IpAddr::V4(ipv4) => {
            if ipv4.is_multicast() || ipv4.is_broadcast() {
                Err(ErrorCode::InvalidArgument.into())
            } else {
                Ok(())
            }
        }
        IpAddr::V6(ipv6) => {
            if ipv6.is_multicast() {
                Err(ErrorCode::InvalidArgument.into())
            } else {
                Ok(())
            }
        }
    }
}

fn validate_remote_address(addr: &SocketAddr) -> SocketResult<()> {
    if to_canonical(&addr.ip()).is_unspecified() {
        return Err(ErrorCode::InvalidArgument.into());
    }

    if addr.port() == 0 {
        return Err(ErrorCode::InvalidArgument.into());
    }

    Ok(())
}

fn validate_address_family(socket: &TcpSocket, addr: &SocketAddr) -> SocketResult<()> {
    match (socket.family, addr.ip()) {
        (SocketAddressFamily::Ipv4, IpAddr::V4(_)) => Ok(()),
        (SocketAddressFamily::Ipv6 { v6only }, IpAddr::V6(ipv6)) => {
            if is_deprecated_ipv4_compatible(&ipv6) {
                // Reject IPv4-*compatible* IPv6 addresses. They have been deprecated
                // since 2006, OS handling of them is inconsistent and our own
                // validations don't take them into account either.
                // Note that these are not the same as IPv4-*mapped* IPv6 addresses.
                Err(ErrorCode::InvalidArgument.into())
            } else if v6only && ipv6.to_ipv4_mapped().is_some() {
                Err(ErrorCode::InvalidArgument.into())
            } else {
                Ok(())
            }
        }
        _ => Err(ErrorCode::InvalidArgument.into()),
    }
}

// Can be removed once `IpAddr::to_canonical` becomes stable.
fn to_canonical(addr: &IpAddr) -> IpAddr {
    match addr {
        IpAddr::V4(ipv4) => IpAddr::V4(*ipv4),
        IpAddr::V6(ipv6) => {
            if let Some(ipv4) = ipv6.to_ipv4_mapped() {
                IpAddr::V4(ipv4)
            } else {
                IpAddr::V6(*ipv6)
            }
        }
    }
}

fn is_deprecated_ipv4_compatible(addr: &Ipv6Addr) -> bool {
    matches!(addr.segments(), [0, 0, 0, 0, 0, 0, _, _])
        && *addr != Ipv6Addr::UNSPECIFIED
        && *addr != Ipv6Addr::LOCALHOST
}

fn normalize_setsockopt_buffer_size(value: u64) -> usize {
    value.clamp(1, i32::MAX as u64).try_into().unwrap()
}

fn normalize_getsockopt_buffer_size(value: u64) -> u64 {
    if cfg!(target_os = "linux") {
        // Linux doubles the value passed to setsockopt to allow space for bookkeeping overhead.
        // getsockopt returns this internally doubled value.
        // We'll half the value to at least get it back into the same ballpark that the application requested it in.
        value / 2
    } else {
        value
    }
}

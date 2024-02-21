use crate::host::network::util;
use crate::network::SocketAddrUse;
use crate::tcp::{TcpReadStream, TcpSocket, TcpState, TcpWriteStream};
use crate::{
    bindings::{
        io::streams::{InputStream, OutputStream},
        sockets::network::{ErrorCode, IpAddressFamily, IpSocketAddress, Network},
        sockets::tcp::{self, ShutdownType},
    },
    network::SocketAddressFamily,
};
use crate::{with_ambient_tokio_runtime, Pollable, SocketResult, WasiView};
use io_lifetimes::AsSocketlike;
use rustix::io::Errno;
use rustix::net::sockopt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use wasmtime::component::Resource;

impl<T: WasiView> tcp::Host for T {}

impl<T: WasiView> crate::host::tcp::tcp::HostTcpSocket for T {
    fn start_bind(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        self.ctx().allowed_network_uses.check_allowed_tcp()?;
        let table = self.table();
        let socket = table.get(&this)?;
        let network = table.get(&network)?;
        let local_address: SocketAddr = local_address.into();

        let tokio_socket = match &socket.tcp_state {
            TcpState::Default(socket) => socket,
            TcpState::BindStarted(..) => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => return Err(ErrorCode::InvalidState.into()),
        };

        util::validate_unicast(&local_address)?;
        util::validate_address_family(&local_address, &socket.family)?;

        {
            // Ensure that we're allowed to connect to this address.
            network.check_socket_addr(&local_address, SocketAddrUse::TcpBind)?;

            // Automatically bypass the TIME_WAIT state when the user is trying
            // to bind to a specific port:
            let reuse_addr = local_address.port() > 0;

            // Unconditionally (re)set SO_REUSEADDR, even when the value is false.
            // This ensures we're not accidentally affected by any socket option
            // state left behind by a previous failed call to this method (start_bind).
            util::set_tcp_reuseaddr(&tokio_socket, reuse_addr)?;

            // Perform the OS bind call.
            tokio_socket.bind(local_address).map_err(|error| {
                match Errno::from_io_error(&error) {
                    // From https://pubs.opengroup.org/onlinepubs/9699919799/functions/bind.html:
                    // > [EAFNOSUPPORT] The specified address is not a valid address for the address family of the specified socket
                    //
                    // The most common reasons for this error should have already
                    // been handled by our own validation slightly higher up in this
                    // function. This error mapping is here just in case there is
                    // an edge case we didn't catch.
                    Some(Errno::AFNOSUPPORT) => ErrorCode::InvalidArgument,

                    // See: https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-bind#:~:text=WSAENOBUFS
                    // Windows returns WSAENOBUFS when the ephemeral ports have been exhausted.
                    #[cfg(windows)]
                    Some(Errno::NOBUFS) => ErrorCode::AddressInUse,

                    _ => ErrorCode::from(error),
                }
            })?;
        }

        let socket = table.get_mut(&this)?;

        socket.tcp_state = match std::mem::replace(&mut socket.tcp_state, TcpState::Closed) {
            TcpState::Default(socket) => TcpState::BindStarted(socket),
            _ => unreachable!(),
        };

        Ok(())
    }

    fn finish_bind(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;

        match socket.tcp_state {
            TcpState::BindStarted(..) => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        socket.tcp_state = match std::mem::replace(&mut socket.tcp_state, TcpState::Closed) {
            TcpState::BindStarted(socket) => TcpState::Bound(socket),
            _ => unreachable!(),
        };

        Ok(())
    }

    fn start_connect(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        network: Resource<Network>,
        remote_address: IpSocketAddress,
    ) -> SocketResult<()> {
        self.ctx().allowed_network_uses.check_allowed_tcp()?;
        let table = self.table();
        let socket = table.get(&this)?;
        let network = table.get(&network)?;
        let remote_address: SocketAddr = remote_address.into();

        match socket.tcp_state {
            TcpState::Default(..) => {}

            TcpState::Connecting(..) | TcpState::ConnectReady(..) => {
                return Err(ErrorCode::ConcurrencyConflict.into())
            }

            _ => return Err(ErrorCode::InvalidState.into()),
        };

        util::validate_unicast(&remote_address)?;
        util::validate_remote_address(&remote_address)?;
        util::validate_address_family(&remote_address, &socket.family)?;

        // Ensure that we're allowed to connect to this address.
        network.check_socket_addr(&remote_address, SocketAddrUse::TcpConnect)?;

        let socket = table.get_mut(&this)?;
        let TcpState::Default(tokio_socket) =
            std::mem::replace(&mut socket.tcp_state, TcpState::Closed)
        else {
            unreachable!();
        };

        let future = tokio_socket.connect(remote_address);

        socket.tcp_state = TcpState::Connecting(Box::pin(future));

        Ok(())
    }

    fn finish_connect(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> SocketResult<(Resource<InputStream>, Resource<OutputStream>)> {
        let table = self.table();
        let socket = table.get_mut(&this)?;

        let previous_state = std::mem::replace(&mut socket.tcp_state, TcpState::Closed);
        let result = match previous_state {
            TcpState::ConnectReady(result) => result,
            TcpState::Connecting(mut future) => {
                let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());
                match with_ambient_tokio_runtime(|| future.as_mut().poll(&mut cx)) {
                    Poll::Ready(result) => result,
                    Poll::Pending => {
                        socket.tcp_state = TcpState::Connecting(future);
                        return Err(ErrorCode::WouldBlock.into());
                    }
                }
            }
            previous_state => {
                socket.tcp_state = previous_state;
                return Err(ErrorCode::NotInProgress.into());
            }
        };

        match result {
            Ok(stream) => {
                let stream = Arc::new(stream);

                let input: InputStream =
                    InputStream::Host(Box::new(TcpReadStream::new(stream.clone())));
                let output: OutputStream = Box::new(TcpWriteStream::new(stream.clone()));

                let input_stream = self.table().push_child(input, &this)?;
                let output_stream = self.table().push_child(output, &this)?;

                let socket = self.table().get_mut(&this)?;
                socket.tcp_state = TcpState::Connected(stream);
                Ok((input_stream, output_stream))
            }
            Err(err) => {
                socket.tcp_state = TcpState::Closed;
                Err(err.into())
            }
        }
    }

    fn start_listen(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<()> {
        self.ctx().allowed_network_uses.check_allowed_tcp()?;
        let table = self.table();
        let socket = table.get_mut(&this)?;

        match std::mem::replace(&mut socket.tcp_state, TcpState::Closed) {
            TcpState::Bound(tokio_socket) => {
                socket.tcp_state = TcpState::ListenStarted(tokio_socket);
                Ok(())
            }
            TcpState::ListenStarted(tokio_socket) => {
                socket.tcp_state = TcpState::ListenStarted(tokio_socket);
                Err(ErrorCode::ConcurrencyConflict.into())
            }
            previous_state => {
                socket.tcp_state = previous_state;
                Err(ErrorCode::InvalidState.into())
            }
        }
    }

    fn finish_listen(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;

        let tokio_socket = match std::mem::replace(&mut socket.tcp_state, TcpState::Closed) {
            TcpState::ListenStarted(tokio_socket) => tokio_socket,
            previous_state => {
                socket.tcp_state = previous_state;
                return Err(ErrorCode::NotInProgress.into());
            }
        };

        match with_ambient_tokio_runtime(|| tokio_socket.listen(socket.listen_backlog_size)) {
            Ok(listener) => {
                socket.tcp_state = TcpState::Listening {
                    listener,
                    pending_accept: None,
                };
                Ok(())
            }
            Err(err) => {
                socket.tcp_state = TcpState::Closed;

                Err(match Errno::from_io_error(&err) {
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

                    _ => err.into(),
                })
            }
        }
    }

    fn accept(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> SocketResult<(
        Resource<tcp::TcpSocket>,
        Resource<InputStream>,
        Resource<OutputStream>,
    )> {
        self.ctx().allowed_network_uses.check_allowed_tcp()?;
        let table = self.table();
        let socket = table.get_mut(&this)?;

        let TcpState::Listening {
            listener,
            pending_accept,
        } = &mut socket.tcp_state
        else {
            return Err(ErrorCode::InvalidState.into());
        };

        let result = match pending_accept.take() {
            Some(result) => result,
            None => {
                let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());
                match with_ambient_tokio_runtime(|| listener.poll_accept(&mut cx))
                    .map_ok(|(stream, _)| stream)
                {
                    Poll::Ready(result) => result,
                    Poll::Pending => Err(Errno::WOULDBLOCK.into()),
                }
            }
        };

        let client = result.map_err(|err| match Errno::from_io_error(&err) {
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

            _ => err,
        })?;

        #[cfg(target_os = "macos")]
        {
            // Manually inherit socket options from listener. We only have to
            // do this on platforms that don't already do this automatically
            // and only if a specific value was explicitly set on the listener.

            if let Some(size) = socket.receive_buffer_size {
                _ = util::set_socket_recv_buffer_size(&client, size); // Ignore potential error.
            }

            if let Some(size) = socket.send_buffer_size {
                _ = util::set_socket_send_buffer_size(&client, size); // Ignore potential error.
            }

            // For some reason, IP_TTL is inherited, but IPV6_UNICAST_HOPS isn't.
            if let (SocketAddressFamily::Ipv6, Some(ttl)) = (socket.family, socket.hop_limit) {
                _ = util::set_ipv6_unicast_hops(&client, ttl); // Ignore potential error.
            }

            if let Some(value) = socket.keep_alive_idle_time {
                _ = util::set_tcp_keepidle(&client, value); // Ignore potential error.
            }
        }

        let client = Arc::new(client);

        let input: InputStream = InputStream::Host(Box::new(TcpReadStream::new(client.clone())));
        let output: OutputStream = Box::new(TcpWriteStream::new(client.clone()));
        let tcp_socket = TcpSocket::from_state(TcpState::Connected(client), socket.family)?;

        let tcp_socket = self.table().push(tcp_socket)?;
        let input_stream = self.table().push_child(input, &tcp_socket)?;
        let output_stream = self.table().push_child(output, &tcp_socket)?;

        Ok((tcp_socket, input_stream, output_stream))
    }

    fn local_address(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?;

        let view = match socket.tcp_state {
            TcpState::Default(..) => return Err(ErrorCode::InvalidState.into()),
            TcpState::BindStarted(..) => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => socket.as_std_view()?,
        };

        Ok(view.local_addr()?.into())
    }

    fn remote_address(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?;

        let view = match socket.tcp_state {
            TcpState::Connected(..) => socket.as_std_view()?,
            TcpState::Connecting(..) | TcpState::ConnectReady(..) => {
                return Err(ErrorCode::ConcurrencyConflict.into())
            }
            _ => return Err(ErrorCode::InvalidState.into()),
        };

        Ok(view.peer_addr()?.into())
    }

    fn is_listening(&mut self, this: Resource<tcp::TcpSocket>) -> Result<bool, anyhow::Error> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.tcp_state {
            TcpState::Listening { .. } => Ok(true),
            _ => Ok(false),
        }
    }

    fn address_family(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> Result<IpAddressFamily, anyhow::Error> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.family {
            SocketAddressFamily::Ipv4 => Ok(IpAddressFamily::Ipv4),
            SocketAddressFamily::Ipv6 => Ok(IpAddressFamily::Ipv6),
        }
    }

    fn set_listen_backlog_size(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        const MIN_BACKLOG: u32 = 1;
        const MAX_BACKLOG: u32 = i32::MAX as u32; // OS'es will most likely limit it down even further.

        let table = self.table();
        let socket = table.get_mut(&this)?;

        if value == 0 {
            return Err(ErrorCode::InvalidArgument.into());
        }

        // Silently clamp backlog size. This is OK for us to do, because operating systems do this too.
        let value = value
            .try_into()
            .unwrap_or(u32::MAX)
            .clamp(MIN_BACKLOG, MAX_BACKLOG);

        match &socket.tcp_state {
            TcpState::Default(..) | TcpState::Bound(..) => {
                // Socket not listening yet. Stash value for first invocation to `listen`.
                socket.listen_backlog_size = value;

                Ok(())
            }
            TcpState::Listening { listener, .. } => {
                // Try to update the backlog by calling `listen` again.
                // Not all platforms support this. We'll only update our own value if the OS supports changing the backlog size after the fact.

                rustix::net::listen(&listener, value.try_into().unwrap())
                    .map_err(|_| ErrorCode::NotSupported)?;

                socket.listen_backlog_size = value;

                Ok(())
            }
            _ => Err(ErrorCode::InvalidState.into()),
        }
    }

    fn keep_alive_enabled(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<bool> {
        let table = self.table();
        let socket = table.get(&this)?;
        let view = &*socket.as_std_view()?;
        Ok(sockopt::get_socket_keepalive(view)?)
    }

    fn set_keep_alive_enabled(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: bool,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        let view = &*socket.as_std_view()?;
        Ok(sockopt::set_socket_keepalive(view, value)?)
    }

    fn keep_alive_idle_time(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;
        let view = &*socket.as_std_view()?;
        Ok(sockopt::get_tcp_keepidle(view)?.as_nanos() as u64)
    }

    fn set_keep_alive_idle_time(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;
        let duration = Duration::from_nanos(value);
        {
            let view = &*socket.as_std_view()?;

            util::set_tcp_keepidle(view, duration)?;
        }

        #[cfg(target_os = "macos")]
        {
            socket.keep_alive_idle_time = Some(duration);
        }

        Ok(())
    }

    fn keep_alive_interval(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;
        let view = &*socket.as_std_view()?;
        Ok(sockopt::get_tcp_keepintvl(view)?.as_nanos() as u64)
    }

    fn set_keep_alive_interval(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        let view = &*socket.as_std_view()?;
        Ok(util::set_tcp_keepintvl(view, Duration::from_nanos(value))?)
    }

    fn keep_alive_count(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u32> {
        let table = self.table();
        let socket = table.get(&this)?;
        let view = &*socket.as_std_view()?;
        Ok(sockopt::get_tcp_keepcnt(view)?)
    }

    fn set_keep_alive_count(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u32,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        let view = &*socket.as_std_view()?;
        Ok(util::set_tcp_keepcnt(view, value)?)
    }

    fn hop_limit(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u8> {
        let table = self.table();
        let socket = table.get(&this)?;
        let view = &*socket.as_std_view()?;

        let ttl = match socket.family {
            SocketAddressFamily::Ipv4 => util::get_ip_ttl(view)?,
            SocketAddressFamily::Ipv6 => util::get_ipv6_unicast_hops(view)?,
        };

        Ok(ttl)
    }

    fn set_hop_limit(&mut self, this: Resource<tcp::TcpSocket>, value: u8) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;
        {
            let view = &*socket.as_std_view()?;

            match socket.family {
                SocketAddressFamily::Ipv4 => util::set_ip_ttl(view, value)?,
                SocketAddressFamily::Ipv6 => util::set_ipv6_unicast_hops(view, value)?,
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
        let socket = table.get(&this)?;
        let view = &*socket.as_std_view()?;

        let value = util::get_socket_recv_buffer_size(view)?;
        Ok(value as u64)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);
        {
            let view = &*socket.as_std_view()?;

            util::set_socket_recv_buffer_size(view, value)?;
        }

        #[cfg(target_os = "macos")]
        {
            socket.receive_buffer_size = Some(value);
        }

        Ok(())
    }

    fn send_buffer_size(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;
        let view = &*socket.as_std_view()?;

        let value = util::get_socket_send_buffer_size(view)?;
        Ok(value as u64)
    }

    fn set_send_buffer_size(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);
        {
            let view = &*socket.as_std_view()?;

            util::set_socket_send_buffer_size(view, value)?;
        }

        #[cfg(target_os = "macos")]
        {
            socket.send_buffer_size = Some(value);
        }

        Ok(())
    }

    fn subscribe(&mut self, this: Resource<tcp::TcpSocket>) -> anyhow::Result<Resource<Pollable>> {
        crate::poll::subscribe(self.table(), this)
    }

    fn shutdown(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        shutdown_type: ShutdownType,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;

        let stream = match &socket.tcp_state {
            TcpState::Connected(stream) => stream,
            _ => return Err(ErrorCode::InvalidState.into()),
        };

        let how = match shutdown_type {
            ShutdownType::Receive => std::net::Shutdown::Read,
            ShutdownType::Send => std::net::Shutdown::Write,
            ShutdownType::Both => std::net::Shutdown::Both,
        };

        stream
            .as_socketlike_view::<std::net::TcpStream>()
            .shutdown(how)?;
        Ok(())
    }

    fn drop(&mut self, this: Resource<tcp::TcpSocket>) -> Result<(), anyhow::Error> {
        let table = self.table();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

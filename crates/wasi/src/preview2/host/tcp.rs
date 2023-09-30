use crate::preview2::bindings::{
    io::streams::{InputStream, OutputStream},
    poll::poll::Pollable,
    sockets::network::{self, ErrorCode, IpAddressFamily, IpSocketAddress, Network},
    sockets::tcp::{self, ShutdownType},
};
use crate::preview2::network::TableNetworkExt;
use crate::preview2::poll::TablePollableExt;
use crate::preview2::stream::TableStreamExt;
use crate::preview2::tcp::{HostTcpSocket, HostTcpState, TableTcpSocketExt};
use crate::preview2::{HostPollable, PollableFuture, WasiView};
use cap_net_ext::{AddressFamily, Blocking, PoolExt, TcpListenerExt};
use cap_std::net::TcpListener;
use io_lifetimes::AsSocketlike;
use rustix::io::Errno;
use rustix::net::sockopt;
use std::{
    any::Any,
    net::{IpAddr, SocketAddr},
};
use tokio::io::Interest;

use super::network::SystemError;

impl<T: WasiView> tcp::Host for T {
    fn start_bind(
        &mut self,
        this: tcp::TcpSocket,
        network: Network,
        local_address: IpSocketAddress,
    ) -> Result<(), network::Error> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(this)?;
        let network = table.get_network(network)?;
        let local_address: SocketAddr = local_address.into();

        match socket.tcp_state {
            HostTcpState::Default => {}
            HostTcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        validate_unicast(&local_address)?;
        validate_address_family(&socket, &local_address)?;

        let binder = network.0.tcp_binder(local_address)?;

        // Perform the OS bind call.
        binder
            .bind_existing_tcp_listener(&*socket.tcp_socket().as_socketlike_view::<TcpListener>())
            .map_err(|error| match error.errno() {
                Some(Errno::AFNOSUPPORT) => ErrorCode::InvalidArgument.into(),
                _ => Into::<network::Error>::into(error),
            })?;

        let socket = table.get_tcp_socket_mut(this)?;
        socket.tcp_state = HostTcpState::BindStarted;

        Ok(())
    }

    fn finish_bind(&mut self, this: tcp::TcpSocket) -> Result<(), network::Error> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket_mut(this)?;

        match socket.tcp_state {
            HostTcpState::BindStarted => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        socket.tcp_state = HostTcpState::Bound;

        Ok(())
    }

    fn start_connect(
        &mut self,
        this: tcp::TcpSocket,
        network: Network,
        remote_address: IpSocketAddress,
    ) -> Result<(), network::Error> {
        let table = self.table_mut();
        let r = {
            let socket = table.get_tcp_socket(this)?;
            let network = table.get_network(network)?;
            let remote_address: SocketAddr = remote_address.into();

            match socket.tcp_state {
                HostTcpState::Default => {}
                HostTcpState::Bound
                | HostTcpState::Connected
                | HostTcpState::ConnectFailed
                | HostTcpState::Listening => return Err(ErrorCode::InvalidState.into()),
                HostTcpState::Connecting
                | HostTcpState::ConnectReady
                | HostTcpState::ListenStarted
                | HostTcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            }

            validate_unicast(&remote_address)?;
            validate_remote_address(&remote_address)?;
            validate_address_family(&socket, &remote_address)?;

            let connecter = network.0.tcp_connecter(remote_address)?;

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
                let socket = table.get_tcp_socket_mut(this)?;
                socket.tcp_state = HostTcpState::ConnectReady;
                return Ok(());
            }
            // continue in progress,
            Err(err) if err.errno() == Some(INPROGRESS) => {}
            // or fail immediately.
            Err(err) => {
                return Err(match err.errno() {
                    Some(Errno::AFNOSUPPORT) => ErrorCode::InvalidArgument.into(),
                    _ => Into::<network::Error>::into(err),
                })
            }
        }

        let socket = table.get_tcp_socket_mut(this)?;
        socket.tcp_state = HostTcpState::Connecting;

        Ok(())
    }

    fn finish_connect(
        &mut self,
        this: tcp::TcpSocket,
    ) -> Result<(InputStream, OutputStream), network::Error> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket_mut(this)?;

        match socket.tcp_state {
            HostTcpState::ConnectReady => {}
            HostTcpState::Connecting => {
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
                        socket.tcp_state = HostTcpState::ConnectFailed;
                        return Err(err.into());
                    }
                }
            }
            _ => return Err(ErrorCode::NotInProgress.into()),
        };

        socket.tcp_state = HostTcpState::Connected;
        let (input, output) = socket.as_split();
        let input_stream = self.table_mut().push_input_stream_child(input, this)?;
        let output_stream = self.table_mut().push_output_stream_child(output, this)?;

        Ok((input_stream, output_stream))
    }

    fn start_listen(&mut self, this: tcp::TcpSocket) -> Result<(), network::Error> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket_mut(this)?;

        match socket.tcp_state {
            HostTcpState::Bound => {}
            HostTcpState::Default
            | HostTcpState::Connected
            | HostTcpState::ConnectFailed
            | HostTcpState::Listening => return Err(ErrorCode::InvalidState.into()),
            HostTcpState::ListenStarted
            | HostTcpState::Connecting
            | HostTcpState::ConnectReady
            | HostTcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
        }

        socket
            .tcp_socket()
            .as_socketlike_view::<TcpListener>()
            .listen(None)?;

        socket.tcp_state = HostTcpState::ListenStarted;

        Ok(())
    }

    fn finish_listen(&mut self, this: tcp::TcpSocket) -> Result<(), network::Error> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket_mut(this)?;

        match socket.tcp_state {
            HostTcpState::ListenStarted => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        socket.tcp_state = HostTcpState::Listening;

        Ok(())
    }

    fn accept(
        &mut self,
        this: tcp::TcpSocket,
    ) -> Result<(tcp::TcpSocket, InputStream, OutputStream), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        match socket.tcp_state {
            HostTcpState::Listening => {}
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
            .map_err(|error| match error.errno() {
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
                ) => ErrorCode::ConnectionAborted.into(),

                _ => Into::<network::Error>::into(error),
            })?;
        let mut tcp_socket = HostTcpSocket::from_tcp_stream(connection, socket.family)?;

        // Mark the socket as connected so that we can exit early from methods like `start-bind`.
        tcp_socket.tcp_state = HostTcpState::Connected;

        let (input, output) = tcp_socket.as_split();

        let tcp_socket = self.table_mut().push_tcp_socket(tcp_socket)?;
        let input_stream = self
            .table_mut()
            .push_input_stream_child(input, tcp_socket)?;
        let output_stream = self
            .table_mut()
            .push_output_stream_child(output, tcp_socket)?;

        Ok((tcp_socket, input_stream, output_stream))
    }

    fn local_address(&mut self, this: tcp::TcpSocket) -> Result<IpSocketAddress, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        match socket.tcp_state {
            HostTcpState::Default => return Err(ErrorCode::InvalidState.into()),
            HostTcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => {}
        }

        let addr = socket
            .tcp_socket()
            .as_socketlike_view::<std::net::TcpStream>()
            .local_addr()?;
        Ok(addr.into())
    }

    fn remote_address(&mut self, this: tcp::TcpSocket) -> Result<IpSocketAddress, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        match socket.tcp_state {
            HostTcpState::Connected => {}
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        let addr = socket
            .tcp_socket()
            .as_socketlike_view::<std::net::TcpStream>()
            .peer_addr()?;
        Ok(addr.into())
    }

    fn address_family(&mut self, this: tcp::TcpSocket) -> Result<IpAddressFamily, anyhow::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        Ok(socket.family.into())
    }

    fn ipv6_only(&mut self, this: tcp::TcpSocket) -> Result<bool, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::get_ipv6_v6only(socket.tcp_socket())?)
    }

    fn set_ipv6_only(&mut self, this: tcp::TcpSocket, value: bool) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        match socket.tcp_state {
            HostTcpState::Default => {}
            HostTcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        Ok(sockopt::set_ipv6_v6only(socket.tcp_socket(), value)?)
    }

    fn set_listen_backlog_size(
        &mut self,
        this: tcp::TcpSocket,
        value: u64,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        match socket.tcp_state {
            HostTcpState::Listening => {}
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        let value = value.try_into().map_err(|_| ErrorCode::OutOfMemory)?;
        Ok(rustix::net::listen(socket.tcp_socket(), value)?)
    }

    fn keep_alive(&mut self, this: tcp::TcpSocket) -> Result<bool, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::get_socket_keepalive(socket.tcp_socket())?)
    }

    fn set_keep_alive(&mut self, this: tcp::TcpSocket, value: bool) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::set_socket_keepalive(socket.tcp_socket(), value)?)
    }

    fn no_delay(&mut self, this: tcp::TcpSocket) -> Result<bool, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::get_tcp_nodelay(socket.tcp_socket())?)
    }

    fn set_no_delay(&mut self, this: tcp::TcpSocket, value: bool) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::set_tcp_nodelay(socket.tcp_socket(), value)?)
    }

    fn unicast_hop_limit(&mut self, this: tcp::TcpSocket) -> Result<u8, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let ttl = match socket.family {
            AddressFamily::Ipv4 => sockopt::get_ip_ttl(socket.tcp_socket())?.try_into().unwrap(),
            AddressFamily::Ipv6 => sockopt::get_ipv6_unicast_hops(socket.tcp_socket())?,
        };

        Ok(ttl)
    }

    fn set_unicast_hop_limit(
        &mut self,
        this: tcp::TcpSocket,
        value: u8,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        if value == 0 {
            // A well-behaved IP application should never send out new packets with TTL 0.
            // We validate the value ourselves because OS'es are not consistent in this.
            // On Linux the validation is even inconsistent between their IPv4 and IPv6 implementation.
            return Err(ErrorCode::InvalidArgument.into());
        }

        match socket.family {
            AddressFamily::Ipv4 => sockopt::set_ip_ttl(socket.tcp_socket(), value.into())?,
            AddressFamily::Ipv6 => sockopt::set_ipv6_unicast_hops(socket.tcp_socket(), Some(value))?,
        }

        Ok(())
    }

    fn receive_buffer_size(&mut self, this: tcp::TcpSocket) -> Result<u64, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let value = sockopt::get_socket_recv_buffer_size(socket.tcp_socket())? as u64;
        Ok(normalize_getsockopt_buffer_size(value))
    }

    fn set_receive_buffer_size(
        &mut self,
        this: tcp::TcpSocket,
        value: u64,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        Ok(sockopt::set_socket_recv_buffer_size(
            socket.tcp_socket(),
            normalize_setsockopt_buffer_size(value),
        )?)
    }

    fn send_buffer_size(&mut self, this: tcp::TcpSocket) -> Result<u64, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let value = sockopt::get_socket_send_buffer_size(socket.tcp_socket())? as u64;
        Ok(normalize_getsockopt_buffer_size(value))
    }

    fn set_send_buffer_size(
        &mut self,
        this: tcp::TcpSocket,
        value: u64,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        Ok(sockopt::set_socket_send_buffer_size(
            socket.tcp_socket(),
            normalize_setsockopt_buffer_size(value),
        )?)
    }

    fn subscribe(&mut self, this: tcp::TcpSocket) -> anyhow::Result<Pollable> {
        fn make_tcp_socket_future<'a>(stream: &'a mut dyn Any) -> PollableFuture<'a> {
            let socket = stream
                .downcast_mut::<HostTcpSocket>()
                .expect("downcast to HostTcpSocket failed");

            // Some states are ready immediately.
            match socket.tcp_state {
                HostTcpState::BindStarted
                | HostTcpState::ListenStarted
                | HostTcpState::ConnectReady => return Box::pin(async { Ok(()) }),
                _ => {}
            }

            // FIXME: Add `Interest::ERROR` when we update to tokio 1.32.
            let join = Box::pin(async move {
                socket
                    .inner
                    .ready(Interest::READABLE | Interest::WRITABLE)
                    .await
                    .unwrap();
                Ok(())
            });

            join
        }

        let pollable = HostPollable::TableEntry {
            index: this,
            make_future: make_tcp_socket_future,
        };

        Ok(self.table_mut().push_host_pollable(pollable)?)
    }

    fn shutdown(
        &mut self,
        this: tcp::TcpSocket,
        shutdown_type: ShutdownType,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        match socket.tcp_state {
            HostTcpState::Connected => {}
            HostTcpState::Connecting | HostTcpState::ConnectReady => {
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

    fn drop_tcp_socket(&mut self, this: tcp::TcpSocket) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete_tcp_socket(this)?;

        // If we might have an `event::poll` waiting on the socket, wake it up.
        #[cfg(not(unix))]
        {
            match dropped.tcp_state {
                HostTcpState::Default
                | HostTcpState::BindStarted
                | HostTcpState::Bound
                | HostTcpState::ListenStarted
                | HostTcpState::ConnectReady => {}

                HostTcpState::Listening | HostTcpState::Connecting | HostTcpState::Connected => {
                    match rustix::net::shutdown(&dropped.inner, rustix::net::Shutdown::ReadWrite) {
                        Ok(()) | Err(Errno::NOTCONN) => {}
                        Err(err) => Err(err).unwrap(),
                    }
                }
            }
        }

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

fn validate_unicast(addr: &SocketAddr) -> Result<(), network::Error> {
    match to_canonical_compat(&addr.ip()) {
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

fn validate_remote_address(addr: &SocketAddr) -> Result<(), network::Error> {
    if to_canonical_compat(&addr.ip()).is_unspecified() {
        return Err(ErrorCode::InvalidArgument.into());
    }

    if addr.port() == 0 {
        return Err(ErrorCode::InvalidArgument.into());
    }

    Ok(())
}

fn validate_address_family(
    socket: &HostTcpSocket,
    addr: &SocketAddr,
) -> Result<(), network::Error> {
    match (socket.family, addr.ip()) {
        (AddressFamily::Ipv4, IpAddr::V4(_)) => {}
        (AddressFamily::Ipv6, IpAddr::V6(ipv6)) => {
            if let Some(_) = ipv6.to_ipv4_mapped() {
                if sockopt::get_ipv6_v6only(socket.tcp_socket())? {
                    // Address is IPv4-mapped IPv6 address, but socket is IPv6-only.
                    return Err(ErrorCode::InvalidArgument.into());
                }
            }
        }
        _ => return Err(ErrorCode::InvalidArgument.into()),
    }

    Ok(())
}

fn to_canonical_compat(addr: &IpAddr) -> IpAddr {
    match addr {
        IpAddr::V4(ipv4) => IpAddr::V4(*ipv4),
        IpAddr::V6(ipv6) => {
            if let Some(ipv4) = ipv6.to_ipv4_mapped() {
                IpAddr::V4(ipv4)
            } else if let Some(ipv4) = ipv6.to_ipv4() {
                IpAddr::V4(ipv4)
            } else {
                IpAddr::V6(*ipv6)
            }
        }
    }
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
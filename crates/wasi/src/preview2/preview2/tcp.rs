use crate::preview2::bindings::{
    io::streams::{InputStream, OutputStream},
    poll::poll::Pollable,
    sockets::network::{self, ErrorCode, IpAddressFamily, IpSocketAddress, Network},
    sockets::tcp::{self, ShutdownType},
};
use crate::preview2::network::TableNetworkExt;
use crate::preview2::poll::TablePollableExt;
use crate::preview2::stream::{InternalInputStream, InternalOutputStream, InternalTableStreamExt};
use crate::preview2::tcp::{HostTcpSocket, HostTcpState, TableTcpSocketExt};
use crate::preview2::{HostPollable, WasiView};
use cap_net_ext::{Blocking, PoolExt, TcpListenerExt};
use io_lifetimes::AsSocketlike;
use tokio::task::spawn_blocking;

impl<T: WasiView> tcp::Host for T {
    fn start_bind(
        &mut self,
        this: tcp::TcpSocket,
        network: Network,
        local_address: IpSocketAddress,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let mut tcp_state = socket.0.tcp_state.write().unwrap();
        match &*tcp_state {
            HostTcpState::Default => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        let network = table.get_network(network)?;
        let binder = network.0.tcp_binder(local_address)?;

        let clone = socket.clone();
        let future = spawn_blocking(move || {
            let result = binder.bind_existing_tcp_listener(clone.tcp_socket());
            clone.0.notify.notify_waiters();
            result
        });

        *tcp_state = HostTcpState::Bind(future);

        Ok(())
    }

    fn finish_bind(&mut self, this: tcp::TcpSocket) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let mut tcp_state = socket.0.tcp_state.write().unwrap();
        let future = match &*tcp_state {
            HostTcpState::Bind(future) => future,
            _ => return Err(ErrorCode::NotInProgress.into()),
        };

        if future.is_finished() {
            *tcp_state = HostTcpState::Bound;
            Ok(())
        } else {
            Err(ErrorCode::WouldBlock.into())
        }
    }

    fn start_connect(
        &mut self,
        this: tcp::TcpSocket,
        network: Network,
        remote_address: IpSocketAddress,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let mut tcp_state = socket.0.tcp_state.write().unwrap();
        match &*tcp_state {
            HostTcpState::Default => {}
            HostTcpState::Connected => return Err(ErrorCode::AlreadyConnected.into()),
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        let network = table.get_network(network)?;
        let connecter = network.0.tcp_connecter(remote_address)?;

        let clone = socket.clone();
        let future = spawn_blocking(move || {
            let result = connecter.connect_existing_tcp_listener(clone.tcp_socket());
            clone.0.notify.notify_waiters();
            result
        });

        *tcp_state = HostTcpState::Connect(future);

        Ok(())
    }

    fn finish_connect(
        &mut self,
        this: tcp::TcpSocket,
    ) -> Result<(InputStream, OutputStream), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let mut tcp_state = socket.0.tcp_state.write().unwrap();
        let future = match &*tcp_state {
            HostTcpState::Connect(future) => future,
            _ => return Err(ErrorCode::NotInProgress.into()),
        };

        if future.is_finished() {
            *tcp_state = HostTcpState::Connected;
            drop(tcp_state);

            let input_clone = socket.clone();
            let output_clone = socket.clone();

            let input_stream = self
                .table_mut()
                .push_internal_input_stream(InternalInputStream::Host(Box::new(input_clone)))?;
            let output_stream = self
                .table_mut()
                .push_internal_output_stream(InternalOutputStream::Host(Box::new(output_clone)))?;

            Ok((input_stream, output_stream))
        } else {
            Err(ErrorCode::WouldBlock.into())
        }
    }

    fn start_listen(
        &mut self,
        this: tcp::TcpSocket,
        _network: Network,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let mut tcp_state = socket.0.tcp_state.write().unwrap();
        match &*tcp_state {
            HostTcpState::Bound => {}
            HostTcpState::Listen(_) | HostTcpState::Listening => {
                return Err(ErrorCode::AlreadyListening.into())
            }
            HostTcpState::Connected => return Err(ErrorCode::AlreadyConnected.into()),
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        let clone = socket.clone();
        let future = spawn_blocking(move || {
            let result = clone.tcp_socket().listen(None);
            clone.0.notify.notify_waiters();
            result
        });

        *tcp_state = HostTcpState::Listen(future);

        Ok(())
    }

    fn finish_listen(&mut self, this: tcp::TcpSocket) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let mut tcp_state = socket.0.tcp_state.write().unwrap();
        let future = match &*tcp_state {
            HostTcpState::Listen(future) => future,
            _ => return Err(ErrorCode::NotInProgress.into()),
        };

        if future.is_finished() {
            *tcp_state = HostTcpState::Listening;
            Ok(())
        } else {
            Err(ErrorCode::WouldBlock.into())
        }
    }

    fn accept(
        &mut self,
        this: tcp::TcpSocket,
    ) -> Result<(tcp::TcpSocket, InputStream, OutputStream), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let tcp_state = socket.0.tcp_state.read().unwrap();
        match &*tcp_state {
            HostTcpState::Listening => {}
            HostTcpState::Connected => return Err(ErrorCode::AlreadyConnected.into()),
            _ => return Err(ErrorCode::NotInProgress.into()),
        }
        drop(tcp_state);

        let (connection, _addr) = socket.tcp_socket().accept_with(Blocking::No)?;
        let tcp_socket = HostTcpSocket::from_tcp_stream(connection)?;

        let input_clone = socket.clone();
        let output_clone = socket.clone();

        let tcp_socket = self.table_mut().push_tcp_socket(tcp_socket)?;
        let input_stream = self
            .table_mut()
            .push_internal_input_stream(InternalInputStream::Host(Box::new(input_clone)))?;
        let output_stream = self
            .table_mut()
            .push_internal_output_stream(InternalOutputStream::Host(Box::new(output_clone)))?;

        Ok((tcp_socket, input_stream, output_stream))
    }

    fn local_address(&mut self, this: tcp::TcpSocket) -> Result<IpSocketAddress, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        let addr = socket
            .0
            .tcp_socket
            .as_socketlike_view::<std::net::TcpStream>()
            .local_addr()?;
        Ok(addr.into())
    }

    fn remote_address(&mut self, this: tcp::TcpSocket) -> Result<IpSocketAddress, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        let addr = socket
            .0
            .tcp_socket
            .as_socketlike_view::<std::net::TcpStream>()
            .peer_addr()?;
        Ok(addr.into())
    }

    fn address_family(&mut self, this: tcp::TcpSocket) -> Result<IpAddressFamily, anyhow::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        // If `SO_DOMAIN` is available, use it.
        //
        // TODO: OpenBSD also supports this; upstream PRs are posted.
        #[cfg(not(any(apple, windows, target_os = "netbsd", target_os = "openbsd")))]
        {
            use rustix::net::AddressFamily;

            let family = rustix::net::sockopt::get_socket_domain(&socket.0.tcp_socket)?;
            let family = match family {
                AddressFamily::INET => IpAddressFamily::Ipv4,
                AddressFamily::INET6 => IpAddressFamily::Ipv6,
                _ => return Err(ErrorCode::NotSupported.into()),
            };
            Ok(family)
        }

        // When `SO_DOMAIN` is not available, emulate it.
        #[cfg(any(apple, windows, target_os = "netbsd", target_os = "openbsd"))]
        {
            if let Ok(_) = rustix::net::sockopt::get_ipv6_unicast_hops(&socket.0.tcp_socket) {
                return Ok(IpAddressFamily::Ipv6);
            }
            if let Ok(_) = rustix::net::sockopt::get_ip_ttl(&socket.0.tcp_socket) {
                return Ok(IpAddressFamily::Ipv4);
            }
            Err(ErrorCode::NotSupported.into())
        }
    }

    fn ipv6_only(&mut self, this: tcp::TcpSocket) -> Result<bool, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(rustix::net::sockopt::get_ipv6_v6only(&socket.0.tcp_socket)?)
    }

    fn set_ipv6_only(&mut self, this: tcp::TcpSocket, value: bool) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(rustix::net::sockopt::set_ipv6_v6only(
            &socket.0.tcp_socket,
            value,
        )?)
    }

    fn set_listen_backlog_size(
        &mut self,
        this: tcp::TcpSocket,
        value: u64,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let tcp_state = socket.0.tcp_state.read().unwrap();
        match &*tcp_state {
            HostTcpState::Listening => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        let value = value.try_into().map_err(|_| ErrorCode::OutOfMemory)?;
        Ok(rustix::net::listen(&socket.0.tcp_socket, value)?)
    }

    fn keep_alive(&mut self, this: tcp::TcpSocket) -> Result<bool, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(rustix::net::sockopt::get_socket_keepalive(
            &socket.0.tcp_socket,
        )?)
    }

    fn set_keep_alive(&mut self, this: tcp::TcpSocket, value: bool) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(rustix::net::sockopt::set_socket_keepalive(
            &socket.0.tcp_socket,
            value,
        )?)
    }

    fn no_delay(&mut self, this: tcp::TcpSocket) -> Result<bool, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(rustix::net::sockopt::get_tcp_nodelay(&socket.0.tcp_socket)?)
    }

    fn set_no_delay(&mut self, this: tcp::TcpSocket, value: bool) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(rustix::net::sockopt::set_tcp_nodelay(
            &socket.0.tcp_socket,
            value,
        )?)
    }

    fn unicast_hop_limit(&mut self, this: tcp::TcpSocket) -> Result<u8, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        match rustix::net::sockopt::get_ipv6_unicast_hops(&socket.0.tcp_socket) {
            Ok(value) => Ok(value),
            Err(rustix::io::Errno::NOPROTOOPT) => {
                let value = rustix::net::sockopt::get_ip_ttl(&socket.0.tcp_socket)?;
                let value = value.try_into().unwrap();
                Ok(value)
            }
            Err(err) => Err(err.into()),
        }
    }

    fn set_unicast_hop_limit(
        &mut self,
        this: tcp::TcpSocket,
        value: u8,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        match rustix::net::sockopt::set_ipv6_unicast_hops(&socket.0.tcp_socket, Some(value)) {
            Ok(()) => Ok(()),
            Err(rustix::io::Errno::NOPROTOOPT) => Ok(rustix::net::sockopt::set_ip_ttl(
                &socket.0.tcp_socket,
                value.into(),
            )?),
            Err(err) => Err(err.into()),
        }
    }

    fn receive_buffer_size(&mut self, this: tcp::TcpSocket) -> Result<u64, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(rustix::net::sockopt::get_socket_recv_buffer_size(&socket.0.tcp_socket)? as u64)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: tcp::TcpSocket,
        value: u64,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        let value = value.try_into().map_err(|_| ErrorCode::OutOfMemory)?;
        Ok(rustix::net::sockopt::set_socket_recv_buffer_size(
            &socket.0.tcp_socket,
            value,
        )?)
    }

    fn send_buffer_size(&mut self, this: tcp::TcpSocket) -> Result<u64, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(rustix::net::sockopt::get_socket_send_buffer_size(&socket.0.tcp_socket)? as u64)
    }

    fn set_send_buffer_size(
        &mut self,
        this: tcp::TcpSocket,
        value: u64,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        let value = value.try_into().map_err(|_| ErrorCode::OutOfMemory)?;
        Ok(rustix::net::sockopt::set_socket_send_buffer_size(
            &socket.0.tcp_socket,
            value,
        )?)
    }

    fn subscribe(&mut self, this: tcp::TcpSocket) -> Result<Pollable, anyhow::Error> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(this)?;

        let clone = socket.clone();
        let pollable = HostPollable::Closure(Box::new(move || {
            let clone = clone.clone();
            Box::pin(async move {
                let notified = clone.0.notify.notified();
                notified.await;
                Ok(())
            })
        }));

        Ok(table.push_host_pollable(pollable)?)
    }

    fn shutdown(
        &mut self,
        this: tcp::TcpSocket,
        shutdown_type: ShutdownType,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let how = match shutdown_type {
            ShutdownType::Receive => std::net::Shutdown::Read,
            ShutdownType::Send => std::net::Shutdown::Write,
            ShutdownType::Both => std::net::Shutdown::Both,
        };

        socket
            .0
            .tcp_socket
            .as_socketlike_view::<std::net::TcpStream>()
            .shutdown(how)?;
        Ok(())
    }

    fn drop_tcp_socket(&mut self, this: tcp::TcpSocket) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        table.delete_tcp_socket(this)?;

        Ok(())
    }
}

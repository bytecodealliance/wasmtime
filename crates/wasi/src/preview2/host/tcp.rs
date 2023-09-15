use crate::preview2::bindings::{
    io::poll::Pollable,
    io::streams::{InputStream, OutputStream},
    sockets::network::{self, ErrorCode, IpAddressFamily, IpSocketAddress, Network},
    sockets::tcp::{self, ShutdownType},
};
use crate::preview2::network::TableNetworkExt;
use crate::preview2::poll::TablePollableExt;
use crate::preview2::stream::TableStreamExt;
use crate::preview2::tcp::{HostTcpSocketState, HostTcpState, TableTcpSocketExt};
use crate::preview2::{HostPollable, PollableFuture, WasiView};
use cap_net_ext::{Blocking, PoolExt, TcpListenerExt};
use cap_std::net::TcpListener;
use io_lifetimes::AsSocketlike;
use rustix::io::Errno;
use rustix::net::sockopt;
use std::any::Any;
use tokio::io::Interest;
use wasmtime::component::Resource;

impl<T: WasiView> tcp::Host for T {}

impl<T: WasiView> crate::preview2::host::tcp::tcp::HostTcpSocket for T {
    fn start_bind(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> Result<(), network::Error> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(&this)?;

        match socket.tcp_state {
            HostTcpState::Default => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        let network = table.get_network(&network)?;
        let binder = network.0.tcp_binder(local_address)?;

        // Perform the OS bind call.
        binder.bind_existing_tcp_listener(
            &*socket.tcp_socket().as_socketlike_view::<TcpListener>(),
        )?;

        let socket = table.get_tcp_socket_mut(&this)?;
        socket.tcp_state = HostTcpState::BindStarted;

        Ok(())
    }

    fn finish_bind(&mut self, this: Resource<tcp::TcpSocket>) -> Result<(), network::Error> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket_mut(&this)?;

        match socket.tcp_state {
            HostTcpState::BindStarted => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        socket.tcp_state = HostTcpState::Bound;

        Ok(())
    }

    fn start_connect(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        network: Resource<Network>,
        remote_address: IpSocketAddress,
    ) -> Result<(), network::Error> {
        let table = self.table_mut();
        let r = {
            let socket = table.get_tcp_socket(&this)?;

            match socket.tcp_state {
                HostTcpState::Default => {}
                HostTcpState::Connected => return Err(ErrorCode::AlreadyConnected.into()),
                _ => return Err(ErrorCode::NotInProgress.into()),
            }

            let network = table.get_network(&network)?;
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
                let socket = table.get_tcp_socket_mut(&this)?;
                socket.tcp_state = HostTcpState::ConnectReady;
                return Ok(());
            }
            // continue in progress,
            Err(err) if err.raw_os_error() == Some(INPROGRESS.raw_os_error()) => {}
            // or fail immediately.
            Err(err) => return Err(err.into()),
        }

        let socket = table.get_tcp_socket_mut(&this)?;
        socket.tcp_state = HostTcpState::Connecting;

        Ok(())
    }

    fn finish_connect(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> Result<(Resource<InputStream>, Resource<OutputStream>), network::Error> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket_mut(&this)?;

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
                    Err(err) | Ok(Err(err)) => return Err(err.into()),
                }
            }
            _ => return Err(ErrorCode::NotInProgress.into()),
        };

        socket.tcp_state = HostTcpState::Connected;
        let (input, output) = socket.as_split();
        let input_stream = self
            .table_mut()
            .push_input_stream_child(input, Resource::<tcp::TcpSocket>::new_borrow(this.rep()))?;
        let output_stream = self
            .table_mut()
            .push_output_stream_child(output, Resource::<tcp::TcpSocket>::new_borrow(this.rep()))?;

        Ok((input_stream, output_stream))
    }

    fn start_listen(&mut self, this: Resource<tcp::TcpSocket>) -> Result<(), network::Error> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket_mut(&this)?;

        match socket.tcp_state {
            HostTcpState::Bound => {}
            HostTcpState::ListenStarted => return Err(ErrorCode::AlreadyListening.into()),
            HostTcpState::Connected => return Err(ErrorCode::AlreadyConnected.into()),
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        socket
            .tcp_socket()
            .as_socketlike_view::<TcpListener>()
            .listen(None)?;

        socket.tcp_state = HostTcpState::ListenStarted;

        Ok(())
    }

    fn finish_listen(&mut self, this: Resource<tcp::TcpSocket>) -> Result<(), network::Error> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket_mut(&this)?;

        match socket.tcp_state {
            HostTcpState::ListenStarted => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        socket.tcp_state = HostTcpState::Listening;

        Ok(())
    }

    fn accept(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> Result<
        (
            Resource<tcp::TcpSocket>,
            Resource<InputStream>,
            Resource<OutputStream>,
        ),
        network::Error,
    > {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;

        match socket.tcp_state {
            HostTcpState::Listening => {}
            HostTcpState::Connected => return Err(ErrorCode::AlreadyConnected.into()),
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        // Do the OS accept call.
        let tcp_socket = socket.tcp_socket();
        let (connection, _addr) = tcp_socket.try_io(Interest::READABLE, || {
            tcp_socket
                .as_socketlike_view::<TcpListener>()
                .accept_with(Blocking::No)
        })?;
        let mut tcp_socket = HostTcpSocketState::from_tcp_stream(connection)?;

        // Mark the socket as connected so that we can exit early from methods like `start-bind`.
        tcp_socket.tcp_state = HostTcpState::Connected;

        let (input, output) = tcp_socket.as_split();

        let tcp_socket = self.table_mut().push_tcp_socket(tcp_socket)?;
        let input_stream = self.table_mut().push_input_stream_child(
            input,
            Resource::<tcp::TcpSocket>::new_borrow(tcp_socket.rep()),
        )?;
        let output_stream = self.table_mut().push_output_stream_child(
            output,
            Resource::<tcp::TcpSocket>::new_borrow(tcp_socket.rep()),
        )?;

        Ok((tcp_socket, input_stream, output_stream))
    }

    fn local_address(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> Result<IpSocketAddress, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;
        let addr = socket
            .tcp_socket()
            .as_socketlike_view::<std::net::TcpStream>()
            .local_addr()?;
        Ok(addr.into())
    }

    fn remote_address(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> Result<IpSocketAddress, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;
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
        let socket = table.get_tcp_socket(&this)?;

        // If `SO_DOMAIN` is available, use it.
        //
        // TODO: OpenBSD also supports this; upstream PRs are posted.
        #[cfg(not(any(
            windows,
            target_os = "ios",
            target_os = "macos",
            target_os = "netbsd",
            target_os = "openbsd"
        )))]
        {
            use rustix::net::AddressFamily;

            let family = sockopt::get_socket_domain(socket.tcp_socket())?;
            let family = match family {
                AddressFamily::INET => IpAddressFamily::Ipv4,
                AddressFamily::INET6 => IpAddressFamily::Ipv6,
                _ => return Err(ErrorCode::NotSupported.into()),
            };
            Ok(family)
        }

        // When `SO_DOMAIN` is not available, emulate it.
        #[cfg(any(
            windows,
            target_os = "ios",
            target_os = "macos",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            if let Ok(_) = sockopt::get_ipv6_unicast_hops(socket.tcp_socket()) {
                return Ok(IpAddressFamily::Ipv6);
            }
            if let Ok(_) = sockopt::get_ip_ttl(socket.tcp_socket()) {
                return Ok(IpAddressFamily::Ipv4);
            }
            Err(ErrorCode::NotSupported.into())
        }
    }

    fn ipv6_only(&mut self, this: Resource<tcp::TcpSocket>) -> Result<bool, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;
        Ok(sockopt::get_ipv6_v6only(socket.tcp_socket())?)
    }

    fn set_ipv6_only(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: bool,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;
        Ok(sockopt::set_ipv6_v6only(socket.tcp_socket(), value)?)
    }

    fn set_listen_backlog_size(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;

        match socket.tcp_state {
            HostTcpState::Listening => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        let value = value.try_into().map_err(|_| ErrorCode::OutOfMemory)?;
        Ok(rustix::net::listen(socket.tcp_socket(), value)?)
    }

    fn keep_alive(&mut self, this: Resource<tcp::TcpSocket>) -> Result<bool, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;
        Ok(sockopt::get_socket_keepalive(socket.tcp_socket())?)
    }

    fn set_keep_alive(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: bool,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;
        Ok(sockopt::set_socket_keepalive(socket.tcp_socket(), value)?)
    }

    fn no_delay(&mut self, this: Resource<tcp::TcpSocket>) -> Result<bool, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;
        Ok(sockopt::get_tcp_nodelay(socket.tcp_socket())?)
    }

    fn set_no_delay(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: bool,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;
        Ok(sockopt::set_tcp_nodelay(socket.tcp_socket(), value)?)
    }

    fn unicast_hop_limit(&mut self, this: Resource<tcp::TcpSocket>) -> Result<u8, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;

        // We don't track whether the socket is IPv4 or IPv6 so try one and
        // fall back to the other.
        match sockopt::get_ipv6_unicast_hops(socket.tcp_socket()) {
            Ok(value) => Ok(value),
            Err(Errno::NOPROTOOPT) => {
                let value = sockopt::get_ip_ttl(socket.tcp_socket())?;
                let value = value.try_into().unwrap();
                Ok(value)
            }
            Err(err) => Err(err.into()),
        }
    }

    fn set_unicast_hop_limit(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u8,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;

        // We don't track whether the socket is IPv4 or IPv6 so try one and
        // fall back to the other.
        match sockopt::set_ipv6_unicast_hops(socket.tcp_socket(), Some(value)) {
            Ok(()) => Ok(()),
            Err(Errno::NOPROTOOPT) => Ok(sockopt::set_ip_ttl(socket.tcp_socket(), value.into())?),
            Err(err) => Err(err.into()),
        }
    }

    fn receive_buffer_size(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> Result<u64, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;
        Ok(sockopt::get_socket_recv_buffer_size(socket.tcp_socket())? as u64)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;
        let value = value.try_into().map_err(|_| ErrorCode::OutOfMemory)?;
        Ok(sockopt::set_socket_recv_buffer_size(
            socket.tcp_socket(),
            value,
        )?)
    }

    fn send_buffer_size(&mut self, this: Resource<tcp::TcpSocket>) -> Result<u64, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;
        Ok(sockopt::get_socket_send_buffer_size(socket.tcp_socket())? as u64)
    }

    fn set_send_buffer_size(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;
        let value = value.try_into().map_err(|_| ErrorCode::OutOfMemory)?;
        Ok(sockopt::set_socket_send_buffer_size(
            socket.tcp_socket(),
            value,
        )?)
    }

    fn subscribe(&mut self, this: Resource<tcp::TcpSocket>) -> anyhow::Result<Resource<Pollable>> {
        fn make_tcp_socket_future<'a>(stream: &'a mut dyn Any) -> PollableFuture<'a> {
            let socket = stream
                .downcast_mut::<HostTcpSocketState>()
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
            index: this.rep(),
            make_future: make_tcp_socket_future,
        };

        Ok(self.table_mut().push_host_pollable(pollable)?)
    }

    fn shutdown(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        shutdown_type: ShutdownType,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(&this)?;

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

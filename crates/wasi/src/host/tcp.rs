use crate::host::network::util;
use crate::network::SocketAddrUse;
use crate::tcp::{TcpReadStream, TcpState, TcpWriteStream};
use crate::{
    bindings::{
        io::streams::{InputStream, OutputStream},
        sockets::network::{ErrorCode, IpAddressFamily, IpSocketAddress, Network},
        sockets::tcp::{self, ShutdownType},
    },
    network::SocketAddressFamily,
};
use crate::{Pollable, SocketResult, WasiView};
use io_lifetimes::AsSocketlike;
use rustix::net::sockopt;
use std::net::SocketAddr;
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
        let network = table.get(&network)?;
        let local_address: SocketAddr = local_address.into();

        // Ensure that we're allowed to connect to this address.
        network.check_socket_addr(&local_address, SocketAddrUse::TcpBind)?;

        // Bind to the address.
        table.get_mut(&this)?.bind(local_address)?;

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
        let network = table.get(&network)?;
        let remote_address: SocketAddr = remote_address.into();

        // Ensure that we're allowed to connect to this address.
        network.check_socket_addr(&remote_address, SocketAddrUse::TcpConnect)?;

        // Start connection
        table.get_mut(&this)?.start_connect(remote_address)?;

        Ok(())
    }

    fn finish_connect(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> SocketResult<(Resource<InputStream>, Resource<OutputStream>)> {
        let table = self.table();
        let socket = table.get_mut(&this)?;

        let stream = socket.finish_connect()?;

        let input: InputStream = InputStream::Host(Box::new(TcpReadStream::new(stream.clone())));
        let output: OutputStream = Box::new(TcpWriteStream::new(stream));

        let input_stream = self.table().push_child(input, &this)?;
        let output_stream = self.table().push_child(output, &this)?;

        Ok((input_stream, output_stream))
    }

    fn start_listen(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<()> {
        self.ctx().allowed_network_uses.check_allowed_tcp()?;
        let table = self.table();
        let socket = table.get_mut(&this)?;

        socket.start_listen()
    }

    fn finish_listen(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;
        socket.finish_listen()
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

        let (tcp_socket, input, output) = socket.accept()?;

        let tcp_socket = self.table().push(tcp_socket)?;
        let input_stream = self.table().push_child(input, &tcp_socket)?;
        let output_stream = self.table().push_child(output, &tcp_socket)?;

        Ok((tcp_socket, input_stream, output_stream))
    }

    fn local_address(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?;

        socket.local_address().map(Into::into)
    }

    fn remote_address(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?;

        socket.remote_address().map(Into::into)
    }

    fn is_listening(&mut self, this: Resource<tcp::TcpSocket>) -> Result<bool, anyhow::Error> {
        let table = self.table();
        let socket = table.get(&this)?;

        Ok(socket.is_listening())
    }

    fn address_family(
        &mut self,
        this: Resource<tcp::TcpSocket>,
    ) -> Result<IpAddressFamily, anyhow::Error> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.address_family() {
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

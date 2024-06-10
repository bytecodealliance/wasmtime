use crate::network::SocketAddrUse;
use crate::{
    bindings::{
        io::streams::{InputStream, OutputStream},
        sockets::network::{IpAddressFamily, IpSocketAddress, Network},
        sockets::tcp::{self, ShutdownType},
    },
    network::SocketAddressFamily,
};
use crate::{Pollable, SocketResult, WasiImpl, WasiView};
use std::net::SocketAddr;
use std::time::Duration;
use wasmtime::component::Resource;

impl<T> tcp::Host for WasiImpl<T> where T: WasiView {}

#[async_trait::async_trait]
impl<T> crate::host::tcp::tcp::HostTcpSocket for WasiImpl<T>
where
    T: WasiView,
{
    async fn start_bind(
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
        network
            .check_socket_addr(local_address, SocketAddrUse::TcpBind)
            .await?;

        // Bind to the address.
        table.get_mut(&this)?.start_bind(local_address)?;

        Ok(())
    }

    fn finish_bind(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;

        socket.finish_bind()
    }

    async fn start_connect(
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
        network
            .check_socket_addr(remote_address, SocketAddrUse::TcpConnect)
            .await?;

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

        let (input, output) = socket.finish_connect()?;

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
        let table = self.table();
        let socket = table.get_mut(&this)?;

        // Silently clamp backlog size. This is OK for us to do, because operating systems do this too.
        let value = value.try_into().unwrap_or(u32::MAX);

        socket.set_listen_backlog_size(value)
    }

    fn keep_alive_enabled(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<bool> {
        let table = self.table();
        let socket = table.get(&this)?;
        socket.keep_alive_enabled()
    }

    fn set_keep_alive_enabled(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: bool,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        socket.set_keep_alive_enabled(value)
    }

    fn keep_alive_idle_time(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(socket.keep_alive_idle_time()?.as_nanos() as u64)
    }

    fn set_keep_alive_idle_time(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;
        let duration = Duration::from_nanos(value);
        socket.set_keep_alive_idle_time(duration)
    }

    fn keep_alive_interval(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(socket.keep_alive_interval()?.as_nanos() as u64)
    }

    fn set_keep_alive_interval(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        socket.set_keep_alive_interval(Duration::from_nanos(value))
    }

    fn keep_alive_count(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u32> {
        let table = self.table();
        let socket = table.get(&this)?;
        socket.keep_alive_count()
    }

    fn set_keep_alive_count(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u32,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        socket.set_keep_alive_count(value)
    }

    fn hop_limit(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u8> {
        let table = self.table();
        let socket = table.get(&this)?;
        socket.hop_limit()
    }

    fn set_hop_limit(&mut self, this: Resource<tcp::TcpSocket>, value: u8) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;
        socket.set_hop_limit(value)
    }

    fn receive_buffer_size(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;

        Ok(socket.receive_buffer_size()? as u64)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);
        socket.set_receive_buffer_size(value)
    }

    fn send_buffer_size(&mut self, this: Resource<tcp::TcpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;

        Ok(socket.send_buffer_size()? as u64)
    }

    fn set_send_buffer_size(
        &mut self,
        this: Resource<tcp::TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);
        socket.set_send_buffer_size(value)
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

        let how = match shutdown_type {
            ShutdownType::Receive => std::net::Shutdown::Read,
            ShutdownType::Send => std::net::Shutdown::Write,
            ShutdownType::Both => std::net::Shutdown::Both,
        };
        Ok(socket.shutdown(how)?)
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

pub mod sync {
    use wasmtime::component::Resource;

    use crate::{
        bindings::{
            sockets::{
                network::Network,
                tcp::{self as async_tcp, HostTcpSocket as AsyncHostTcpSocket},
            },
            sync::sockets::tcp::{
                self, Duration, HostTcpSocket, InputStream, IpAddressFamily, IpSocketAddress,
                OutputStream, Pollable, ShutdownType, TcpSocket,
            },
        },
        runtime::in_tokio,
        SocketError, WasiImpl, WasiView,
    };

    impl<T> tcp::Host for WasiImpl<T> where T: WasiView {}

    impl<T> HostTcpSocket for WasiImpl<T>
    where
        T: WasiView,
    {
        fn start_bind(
            &mut self,
            self_: Resource<TcpSocket>,
            network: Resource<Network>,
            local_address: IpSocketAddress,
        ) -> Result<(), SocketError> {
            in_tokio(async {
                AsyncHostTcpSocket::start_bind(self, self_, network, local_address).await
            })
        }

        fn finish_bind(&mut self, self_: Resource<TcpSocket>) -> Result<(), SocketError> {
            AsyncHostTcpSocket::finish_bind(self, self_)
        }

        fn start_connect(
            &mut self,
            self_: Resource<TcpSocket>,
            network: Resource<Network>,
            remote_address: IpSocketAddress,
        ) -> Result<(), SocketError> {
            in_tokio(async {
                AsyncHostTcpSocket::start_connect(self, self_, network, remote_address).await
            })
        }

        fn finish_connect(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> Result<(Resource<InputStream>, Resource<OutputStream>), SocketError> {
            AsyncHostTcpSocket::finish_connect(self, self_)
        }

        fn start_listen(&mut self, self_: Resource<TcpSocket>) -> Result<(), SocketError> {
            AsyncHostTcpSocket::start_listen(self, self_)
        }

        fn finish_listen(&mut self, self_: Resource<TcpSocket>) -> Result<(), SocketError> {
            AsyncHostTcpSocket::finish_listen(self, self_)
        }

        fn accept(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> Result<
            (
                Resource<TcpSocket>,
                Resource<InputStream>,
                Resource<OutputStream>,
            ),
            SocketError,
        > {
            AsyncHostTcpSocket::accept(self, self_)
        }

        fn local_address(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> Result<IpSocketAddress, SocketError> {
            AsyncHostTcpSocket::local_address(self, self_)
        }

        fn remote_address(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> Result<IpSocketAddress, SocketError> {
            AsyncHostTcpSocket::remote_address(self, self_)
        }

        fn is_listening(&mut self, self_: Resource<TcpSocket>) -> wasmtime::Result<bool> {
            AsyncHostTcpSocket::is_listening(self, self_)
        }

        fn address_family(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> wasmtime::Result<IpAddressFamily> {
            AsyncHostTcpSocket::address_family(self, self_)
        }

        fn set_listen_backlog_size(
            &mut self,
            self_: Resource<TcpSocket>,
            value: u64,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_listen_backlog_size(self, self_, value)
        }

        fn keep_alive_enabled(&mut self, self_: Resource<TcpSocket>) -> Result<bool, SocketError> {
            AsyncHostTcpSocket::keep_alive_enabled(self, self_)
        }

        fn set_keep_alive_enabled(
            &mut self,
            self_: Resource<TcpSocket>,
            value: bool,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_keep_alive_enabled(self, self_, value)
        }

        fn keep_alive_idle_time(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> Result<Duration, SocketError> {
            AsyncHostTcpSocket::keep_alive_idle_time(self, self_)
        }

        fn set_keep_alive_idle_time(
            &mut self,
            self_: Resource<TcpSocket>,
            value: Duration,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_keep_alive_idle_time(self, self_, value)
        }

        fn keep_alive_interval(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> Result<Duration, SocketError> {
            AsyncHostTcpSocket::keep_alive_interval(self, self_)
        }

        fn set_keep_alive_interval(
            &mut self,
            self_: Resource<TcpSocket>,
            value: Duration,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_keep_alive_interval(self, self_, value)
        }

        fn keep_alive_count(&mut self, self_: Resource<TcpSocket>) -> Result<u32, SocketError> {
            AsyncHostTcpSocket::keep_alive_count(self, self_)
        }

        fn set_keep_alive_count(
            &mut self,
            self_: Resource<TcpSocket>,
            value: u32,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_keep_alive_count(self, self_, value)
        }

        fn hop_limit(&mut self, self_: Resource<TcpSocket>) -> Result<u8, SocketError> {
            AsyncHostTcpSocket::hop_limit(self, self_)
        }

        fn set_hop_limit(
            &mut self,
            self_: Resource<TcpSocket>,
            value: u8,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_hop_limit(self, self_, value)
        }

        fn receive_buffer_size(&mut self, self_: Resource<TcpSocket>) -> Result<u64, SocketError> {
            AsyncHostTcpSocket::receive_buffer_size(self, self_)
        }

        fn set_receive_buffer_size(
            &mut self,
            self_: Resource<TcpSocket>,
            value: u64,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_receive_buffer_size(self, self_, value)
        }

        fn send_buffer_size(&mut self, self_: Resource<TcpSocket>) -> Result<u64, SocketError> {
            AsyncHostTcpSocket::send_buffer_size(self, self_)
        }

        fn set_send_buffer_size(
            &mut self,
            self_: Resource<TcpSocket>,
            value: u64,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_send_buffer_size(self, self_, value)
        }

        fn subscribe(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> wasmtime::Result<Resource<Pollable>> {
            AsyncHostTcpSocket::subscribe(self, self_)
        }

        fn shutdown(
            &mut self,
            self_: Resource<TcpSocket>,
            shutdown_type: ShutdownType,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::shutdown(self, self_, shutdown_type.into())
        }

        fn drop(&mut self, rep: Resource<TcpSocket>) -> wasmtime::Result<()> {
            AsyncHostTcpSocket::drop(self, rep)
        }
    }

    impl From<ShutdownType> for async_tcp::ShutdownType {
        fn from(other: ShutdownType) -> Self {
            match other {
                ShutdownType::Receive => async_tcp::ShutdownType::Receive,
                ShutdownType::Send => async_tcp::ShutdownType::Send,
                ShutdownType::Both => async_tcp::ShutdownType::Both,
            }
        }
    }
}

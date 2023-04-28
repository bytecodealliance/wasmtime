#![allow(unused_variables)]

use crate::{
    network::TableNetworkExt,
    preview2::network::convert,
    preview2::poll::PollableEntry,
    tcp_socket::TableTcpSocketExt,
    wasi::network::{
        Error, IpAddressFamily, Ipv4Address, Ipv4SocketAddress, Ipv6Address, Ipv6SocketAddress,
        Network,
    },
    wasi::poll::Pollable,
    wasi::streams::{InputStream, OutputStream},
    wasi::tcp::{self, IpSocketAddress, ShutdownType, TcpSocket},
    wasi::tcp_create_socket,
    WasiCtx, WasiTcpSocket,
};
use cap_net_ext::AddressFamily;
use cap_std::net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, SocketAddrV4, SocketAddrV6};

#[async_trait::async_trait]
impl tcp::Host for WasiCtx {
    async fn listen(
        &mut self,
        socket: TcpSocket,
        network: Network,
    ) -> anyhow::Result<Result<(), Error>> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(socket)?;
        let network = table.get_network(network)?;

        socket.listen(network).await?;

        Ok(Ok(()))
    }

    async fn accept(
        &mut self,
        socket: TcpSocket,
    ) -> anyhow::Result<Result<(TcpSocket, InputStream, OutputStream), Error>> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(socket)?;

        let (connection, input_stream, output_stream, _addr) = socket.accept(false).await?;

        let connection = table.push(Box::new(connection)).map_err(convert)?;
        let input_stream = table.push(Box::new(input_stream)).map_err(convert)?;
        let output_stream = table.push(Box::new(output_stream)).map_err(convert)?;

        Ok(Ok((connection, input_stream, output_stream)))
    }

    async fn connect(
        &mut self,
        socket: TcpSocket,
        network: Network,
        remote_address: IpSocketAddress,
    ) -> anyhow::Result<Result<(InputStream, OutputStream), Error>> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(socket)?;
        let network = table.get_network(network)?;

        let (input_stream, output_stream) = socket.connect(network, remote_address.into()).await?;

        let input_stream = table.push(Box::new(input_stream)).map_err(convert)?;
        let output_stream = table.push(Box::new(output_stream)).map_err(convert)?;

        Ok(Ok((input_stream, output_stream)))
    }

    async fn receive_buffer_size(
        &mut self,
        socket: TcpSocket,
    ) -> anyhow::Result<Result<u64, Error>> {
        todo!()
    }

    async fn set_receive_buffer_size(
        &mut self,
        socket: TcpSocket,
        value: u64,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn send_buffer_size(&mut self, socket: TcpSocket) -> anyhow::Result<Result<u64, Error>> {
        todo!()
    }

    async fn set_send_buffer_size(
        &mut self,
        socket: TcpSocket,
        value: u64,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn bind(
        &mut self,
        this: TcpSocket,
        network: Network,
        local_address: IpSocketAddress,
    ) -> anyhow::Result<Result<(), Error>> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(this)?;
        let network = table.get_network(network)?;

        socket.bind(network, local_address.into()).await?;

        Ok(Ok(()))
    }

    async fn shutdown(
        &mut self,
        this: TcpSocket,
        shutdown_type: ShutdownType,
    ) -> anyhow::Result<Result<(), Error>> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(this)?;

        let how = match shutdown_type {
            ShutdownType::Receive => Shutdown::Read,
            ShutdownType::Send => Shutdown::Write,
            ShutdownType::Both => Shutdown::Both,
        };

        let addr = socket.shutdown(how).await?;

        Ok(Ok(()))
    }

    async fn local_address(
        &mut self,
        this: TcpSocket,
    ) -> anyhow::Result<Result<IpSocketAddress, Error>> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(this)?;

        let addr = socket.local_address()?;

        Ok(Ok(addr.into()))
    }

    async fn remote_address(
        &mut self,
        this: TcpSocket,
    ) -> anyhow::Result<Result<IpSocketAddress, Error>> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(this)?;

        let addr = socket.remote_address()?;

        Ok(Ok(addr.into()))
    }

    async fn keep_alive(&mut self, this: TcpSocket) -> anyhow::Result<Result<bool, Error>> {
        todo!()
    }

    async fn set_keep_alive(
        &mut self,
        this: TcpSocket,
        value: bool,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn no_delay(&mut self, this: TcpSocket) -> anyhow::Result<Result<bool, Error>> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(this)?;

        let value = socket.nodelay()?;

        Ok(Ok(value))
    }

    async fn set_no_delay(
        &mut self,
        this: TcpSocket,
        value: bool,
    ) -> anyhow::Result<Result<(), Error>> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(this)?;

        socket.set_nodelay(value)?;

        Ok(Ok(()))
    }

    async fn address_family(
        &mut self,
        this: TcpSocket,
    ) -> anyhow::Result<Result<IpAddressFamily, Error>> {
        todo!()
    }

    async fn unicast_hop_limit(&mut self, this: TcpSocket) -> anyhow::Result<Result<u8, Error>> {
        todo!()
    }

    async fn set_unicast_hop_limit(
        &mut self,
        this: TcpSocket,
        value: u8,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn set_listen_backlog_size(
        &mut self,
        this: TcpSocket,
        value: u64,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn ipv6_only(&mut self, this: TcpSocket) -> anyhow::Result<Result<bool, Error>> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(this)?;

        let value = socket.v6_only()?;

        Ok(Ok(value))
    }

    async fn set_ipv6_only(
        &mut self,
        this: TcpSocket,
        value: bool,
    ) -> anyhow::Result<Result<(), Error>> {
        let table = self.table_mut();
        let socket = table.get_tcp_socket(this)?;

        socket.set_v6_only(value)?;

        Ok(Ok(()))
    }

    async fn non_blocking(&mut self, this: TcpSocket) -> anyhow::Result<Result<bool, Error>> {
        todo!()
    }

    async fn set_non_blocking(
        &mut self,
        this: TcpSocket,
        value: bool,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn subscribe(&mut self, this: TcpSocket) -> anyhow::Result<Pollable> {
        Ok(self
            .table_mut()
            .push(Box::new(PollableEntry::TcpSocket(this)))?)
    }

    async fn drop_tcp_socket(&mut self, this: TcpSocket) -> anyhow::Result<()> {
        let table = self.table_mut();
        if !table.delete::<Box<dyn WasiTcpSocket>>(this).is_ok() {
            anyhow::bail!("{this} is not a socket");
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl tcp_create_socket::Host for WasiCtx {
    async fn create_tcp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> anyhow::Result<Result<TcpSocket, Error>> {
        let socket = (self.tcp_socket_creator)(address_family.into())?;
        let table = self.table_mut();
        let socket = table.push(Box::new(socket)).map_err(convert)?;
        Ok(Ok(socket))
    }
}

impl From<IpSocketAddress> for SocketAddr {
    fn from(addr: IpSocketAddress) -> Self {
        match addr {
            IpSocketAddress::Ipv4(v4) => SocketAddr::V4(v4.into()),
            IpSocketAddress::Ipv6(v6) => SocketAddr::V6(v6.into()),
        }
    }
}

impl From<Ipv4SocketAddress> for SocketAddrV4 {
    fn from(addr: Ipv4SocketAddress) -> Self {
        SocketAddrV4::new(convert_ipv4_addr(addr.address), addr.port)
    }
}

impl From<Ipv6SocketAddress> for SocketAddrV6 {
    fn from(addr: Ipv6SocketAddress) -> Self {
        SocketAddrV6::new(
            convert_ipv6_addr(addr.address),
            addr.port,
            addr.flow_info,
            addr.scope_id,
        )
    }
}

fn convert_ipv4_addr(addr: Ipv4Address) -> Ipv4Addr {
    Ipv4Addr::new(addr.0, addr.1, addr.2, addr.3)
}

fn convert_ipv6_addr(addr: Ipv6Address) -> Ipv6Addr {
    Ipv6Addr::new(
        addr.0, addr.1, addr.2, addr.3, addr.4, addr.5, addr.6, addr.7,
    )
}

impl From<IpAddressFamily> for AddressFamily {
    fn from(family: IpAddressFamily) -> Self {
        match family {
            IpAddressFamily::Ipv4 => AddressFamily::Ipv4,
            IpAddressFamily::Ipv6 => AddressFamily::Ipv6,
        }
    }
}

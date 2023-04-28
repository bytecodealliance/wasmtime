#![allow(unused_variables)]

use crate::{
    udp_socket::TableUdpSocketExt,
    wasi::network::{Error, IpAddressFamily, Network},
    wasi::poll::Pollable,
    wasi::udp::{self, Datagram, IpSocketAddress, UdpSocket},
    wasi::udp_create_socket,
    WasiCtx,
};

#[async_trait::async_trait]
impl udp::Host for WasiCtx {
    async fn connect(
        &mut self,
        udp_socket: UdpSocket,
        network: Network,
        remote_address: IpSocketAddress,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn send(
        &mut self,
        socket: UdpSocket,
        datagram: Datagram,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn receive(&mut self, socket: UdpSocket) -> anyhow::Result<Result<Datagram, Error>> {
        todo!()
    }

    async fn receive_buffer_size(
        &mut self,
        socket: UdpSocket,
    ) -> anyhow::Result<Result<u64, Error>> {
        todo!()
    }

    async fn set_receive_buffer_size(
        &mut self,
        socket: UdpSocket,
        value: u64,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn send_buffer_size(&mut self, socket: UdpSocket) -> anyhow::Result<Result<u64, Error>> {
        todo!()
    }

    async fn set_send_buffer_size(
        &mut self,
        socket: UdpSocket,
        value: u64,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn bind(
        &mut self,
        this: UdpSocket,
        network: Network,
        local_address: IpSocketAddress,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn local_address(
        &mut self,
        this: UdpSocket,
    ) -> anyhow::Result<Result<IpSocketAddress, Error>> {
        todo!()
    }

    async fn remote_address(
        &mut self,
        this: UdpSocket,
    ) -> anyhow::Result<Result<IpSocketAddress, Error>> {
        todo!()
    }

    async fn address_family(
        &mut self,
        this: UdpSocket,
    ) -> anyhow::Result<Result<IpAddressFamily, Error>> {
        todo!()
    }

    async fn unicast_hop_limit(&mut self, this: UdpSocket) -> anyhow::Result<Result<u8, Error>> {
        todo!()
    }

    async fn set_unicast_hop_limit(
        &mut self,
        this: UdpSocket,
        value: u8,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn ipv6_only(&mut self, this: UdpSocket) -> anyhow::Result<Result<bool, Error>> {
        todo!()
    }

    async fn set_ipv6_only(
        &mut self,
        this: UdpSocket,
        value: bool,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn non_blocking(&mut self, this: UdpSocket) -> anyhow::Result<Result<bool, Error>> {
        todo!()
    }

    async fn set_non_blocking(
        &mut self,
        this: UdpSocket,
        value: bool,
    ) -> anyhow::Result<Result<(), Error>> {
        let this = self.table.get_udp_socket_mut(this)?;
        this.set_nonblocking(value)?;
        Ok(Ok(()))
    }

    async fn subscribe(&mut self, this: UdpSocket) -> anyhow::Result<Pollable> {
        todo!()
    }

    /* TODO: Revisit after https://github.com/WebAssembly/wasi-sockets/issues/17
    async fn bytes_readable(&mut self, socket: UdpSocket) -> anyhow::Result<Result<(u64, bool), Error>> {
        drop(socket);
        todo!()
    }

    async fn bytes_writable(&mut self, socket: UdpSocket) -> anyhow::Result<Result<(u64, bool), Error>> {
        drop(socket);
        todo!()
    }
    */

    async fn drop_udp_socket(&mut self, socket: UdpSocket) -> anyhow::Result<()> {
        drop(socket);
        todo!()
    }
}

#[async_trait::async_trait]
impl udp_create_socket::Host for WasiCtx {
    async fn create_udp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> anyhow::Result<Result<UdpSocket, Error>> {
        todo!()
    }
}

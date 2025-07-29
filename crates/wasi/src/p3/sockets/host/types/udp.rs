use core::net::SocketAddr;

use anyhow::Context as _;
use wasmtime::component::{Accessor, Resource, ResourceTable};

use crate::p3::bindings::sockets::types::{
    ErrorCode, HostUdpSocket, HostUdpSocketWithStore, IpAddressFamily, IpSocketAddress,
};
use crate::p3::sockets::WasiSockets;
use crate::p3::sockets::udp::UdpSocket;
use crate::sockets::{MAX_UDP_DATAGRAM_SIZE, SocketAddrUse, WasiSocketsCtxView};

use super::is_addr_allowed;

fn is_udp_allowed<T>(store: &Accessor<T, WasiSockets>) -> bool {
    store.with(|mut view| view.get().ctx.allowed_network_uses.udp)
}

fn get_socket<'a>(
    table: &'a ResourceTable,
    socket: &'a Resource<UdpSocket>,
) -> wasmtime::Result<&'a UdpSocket> {
    table
        .get(socket)
        .context("failed to get socket resource from table")
}

fn get_socket_mut<'a>(
    table: &'a mut ResourceTable,
    socket: &'a Resource<UdpSocket>,
) -> wasmtime::Result<&'a mut UdpSocket> {
    table
        .get_mut(socket)
        .context("failed to get socket resource from table")
}

impl HostUdpSocketWithStore for WasiSockets {
    async fn bind<T>(
        store: &Accessor<T, Self>,
        socket: Resource<UdpSocket>,
        local_address: IpSocketAddress,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let local_address = SocketAddr::from(local_address);
        if !is_udp_allowed(store)
            || !is_addr_allowed(store, local_address, SocketAddrUse::UdpBind).await
        {
            return Ok(Err(ErrorCode::AccessDenied));
        }
        store.with(|mut view| {
            let socket = get_socket_mut(view.get().table, &socket)?;
            Ok(socket.bind(local_address))
        })
    }

    async fn connect<T>(
        store: &Accessor<T, Self>,
        socket: Resource<UdpSocket>,
        remote_address: IpSocketAddress,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let remote_address = SocketAddr::from(remote_address);
        if !is_udp_allowed(store)
            || !is_addr_allowed(store, remote_address, SocketAddrUse::UdpConnect).await
        {
            return Ok(Err(ErrorCode::AccessDenied));
        }
        store.with(|mut view| {
            let socket = get_socket_mut(view.get().table, &socket)?;
            Ok(socket.connect(remote_address))
        })
    }

    async fn send<T>(
        store: &Accessor<T, Self>,
        socket: Resource<UdpSocket>,
        data: Vec<u8>,
        remote_address: Option<IpSocketAddress>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        if data.len() > MAX_UDP_DATAGRAM_SIZE {
            return Ok(Err(ErrorCode::DatagramTooLarge));
        }
        if !is_udp_allowed(store) {
            return Ok(Err(ErrorCode::AccessDenied));
        }
        if let Some(addr) = remote_address {
            let addr = SocketAddr::from(addr);
            if !is_addr_allowed(store, addr, SocketAddrUse::UdpOutgoingDatagram).await {
                return Ok(Err(ErrorCode::AccessDenied));
            }
            let fut = store.with(|mut view| {
                get_socket(view.get().table, &socket).map(|sock| sock.send_to(data, addr))
            })?;
            Ok(fut.await)
        } else {
            let fut = store.with(|mut view| {
                get_socket(view.get().table, &socket).map(|sock| sock.send(data))
            })?;
            Ok(fut.await)
        }
    }

    async fn receive<T>(
        store: &Accessor<T, Self>,
        socket: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<(Vec<u8>, IpSocketAddress), ErrorCode>> {
        if !is_udp_allowed(store) {
            return Ok(Err(ErrorCode::AccessDenied));
        }
        let fut = store
            .with(|mut view| get_socket(view.get().table, &socket).map(|sock| sock.receive()))?;
        Ok(fut.await)
    }
}

impl HostUdpSocket for WasiSocketsCtxView<'_> {
    fn new(&mut self, address_family: IpAddressFamily) -> wasmtime::Result<Resource<UdpSocket>> {
        let socket = UdpSocket::new(address_family.into()).context("failed to create socket")?;
        self.table
            .push(socket)
            .context("failed to push socket resource to table")
    }

    fn disconnect(
        &mut self,
        socket: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let socket = get_socket_mut(self.table, &socket)?;
        Ok(socket.disconnect())
    }

    fn local_address(
        &mut self,
        socket: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<IpSocketAddress, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.local_address())
    }

    fn remote_address(
        &mut self,
        socket: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<IpSocketAddress, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.remote_address())
    }

    fn address_family(&mut self, socket: Resource<UdpSocket>) -> wasmtime::Result<IpAddressFamily> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.address_family())
    }

    fn unicast_hop_limit(
        &mut self,
        socket: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<u8, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.unicast_hop_limit())
    }

    fn set_unicast_hop_limit(
        &mut self,
        socket: Resource<UdpSocket>,
        value: u8,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.set_unicast_hop_limit(value))
    }

    fn receive_buffer_size(
        &mut self,
        socket: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<u64, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.receive_buffer_size())
    }

    fn set_receive_buffer_size(
        &mut self,
        socket: Resource<UdpSocket>,
        value: u64,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.set_receive_buffer_size(value))
    }

    fn send_buffer_size(
        &mut self,
        socket: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<u64, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.send_buffer_size())
    }

    fn set_send_buffer_size(
        &mut self,
        socket: Resource<UdpSocket>,
        value: u64,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.set_send_buffer_size(value))
    }

    fn drop(&mut self, sock: Resource<UdpSocket>) -> wasmtime::Result<()> {
        self.table
            .delete(sock)
            .context("failed to delete socket resource from table")?;
        Ok(())
    }
}

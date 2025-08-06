use super::is_addr_allowed;
use crate::TrappableError;
use crate::p3::bindings::sockets::types::{
    ErrorCode, HostUdpSocket, HostUdpSocketWithStore, IpAddressFamily, IpSocketAddress,
};
use crate::p3::sockets::{SocketResult, WasiSockets};
use crate::sockets::{MAX_UDP_DATAGRAM_SIZE, SocketAddrUse, UdpSocket, WasiSocketsCtxView};
use anyhow::Context;
use std::net::SocketAddr;
use wasmtime::component::{Accessor, Resource, ResourceTable};

fn get_socket<'a>(
    table: &'a ResourceTable,
    socket: &'a Resource<UdpSocket>,
) -> SocketResult<&'a UdpSocket> {
    table
        .get(socket)
        .context("failed to get socket resource from table")
        .map_err(TrappableError::trap)
}

fn get_socket_mut<'a>(
    table: &'a mut ResourceTable,
    socket: &'a Resource<UdpSocket>,
) -> SocketResult<&'a mut UdpSocket> {
    table
        .get_mut(socket)
        .context("failed to get socket resource from table")
        .map_err(TrappableError::trap)
}

impl HostUdpSocketWithStore for WasiSockets {
    async fn bind<T>(
        store: &Accessor<T, Self>,
        socket: Resource<UdpSocket>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let local_address = SocketAddr::from(local_address);
        if !is_addr_allowed(store, local_address, SocketAddrUse::UdpBind).await {
            return Err(ErrorCode::AccessDenied.into());
        }
        store.with(|mut view| {
            let socket = get_socket_mut(view.get().table, &socket)?;
            socket.bind(local_address)?;
            socket.finish_bind()?;
            Ok(())
        })
    }

    async fn connect<T>(
        store: &Accessor<T, Self>,
        socket: Resource<UdpSocket>,
        remote_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let remote_address = SocketAddr::from(remote_address);
        if !is_addr_allowed(store, remote_address, SocketAddrUse::UdpConnect).await {
            return Err(ErrorCode::AccessDenied.into());
        }
        store.with(|mut view| {
            let socket = get_socket_mut(view.get().table, &socket)?;
            socket.connect(remote_address)?;
            Ok(())
        })
    }

    async fn send<T>(
        store: &Accessor<T, Self>,
        socket: Resource<UdpSocket>,
        data: Vec<u8>,
        remote_address: Option<IpSocketAddress>,
    ) -> SocketResult<()> {
        if data.len() > MAX_UDP_DATAGRAM_SIZE {
            return Err(ErrorCode::DatagramTooLarge.into());
        }
        if let Some(addr) = remote_address {
            let addr = SocketAddr::from(addr);
            if !is_addr_allowed(store, addr, SocketAddrUse::UdpOutgoingDatagram).await {
                return Err(ErrorCode::AccessDenied.into());
            }
            let fut = store.with(|mut view| {
                get_socket(view.get().table, &socket).map(|sock| sock.send_to(data, addr))
            })?;
            fut.await?;
            Ok(())
        } else {
            let fut = store.with(|mut view| {
                get_socket(view.get().table, &socket).map(|sock| sock.send(data))
            })?;
            fut.await?;
            Ok(())
        }
    }

    async fn receive<T>(
        store: &Accessor<T, Self>,
        socket: Resource<UdpSocket>,
    ) -> SocketResult<(Vec<u8>, IpSocketAddress)> {
        let fut = store
            .with(|mut view| get_socket(view.get().table, &socket).map(|sock| sock.receive()))?;
        let (result, addr) = fut.await?;
        Ok((result, addr.into()))
    }
}

impl HostUdpSocket for WasiSocketsCtxView<'_> {
    fn new(&mut self, address_family: IpAddressFamily) -> wasmtime::Result<Resource<UdpSocket>> {
        let socket = UdpSocket::new(self.ctx, address_family.into())?;
        self.table
            .push(socket)
            .context("failed to push socket resource to table")
    }

    fn disconnect(&mut self, socket: Resource<UdpSocket>) -> SocketResult<()> {
        let socket = get_socket_mut(self.table, &socket)?;
        socket.disconnect()?;
        Ok(())
    }

    fn local_address(&mut self, socket: Resource<UdpSocket>) -> SocketResult<IpSocketAddress> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.local_address()?.into())
    }

    fn remote_address(&mut self, socket: Resource<UdpSocket>) -> SocketResult<IpSocketAddress> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.remote_address()?.into())
    }

    fn address_family(&mut self, socket: Resource<UdpSocket>) -> wasmtime::Result<IpAddressFamily> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.address_family().into())
    }

    fn unicast_hop_limit(&mut self, socket: Resource<UdpSocket>) -> SocketResult<u8> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.unicast_hop_limit()?)
    }

    fn set_unicast_hop_limit(
        &mut self,
        socket: Resource<UdpSocket>,
        value: u8,
    ) -> SocketResult<()> {
        let sock = get_socket(self.table, &socket)?;
        sock.set_unicast_hop_limit(value)?;
        Ok(())
    }

    fn receive_buffer_size(&mut self, socket: Resource<UdpSocket>) -> SocketResult<u64> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.receive_buffer_size()?)
    }

    fn set_receive_buffer_size(
        &mut self,
        socket: Resource<UdpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let sock = get_socket(self.table, &socket)?;
        sock.set_receive_buffer_size(value)?;
        Ok(())
    }

    fn send_buffer_size(&mut self, socket: Resource<UdpSocket>) -> SocketResult<u64> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.send_buffer_size()?)
    }

    fn set_send_buffer_size(
        &mut self,
        socket: Resource<UdpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let sock = get_socket(self.table, &socket)?;
        sock.set_send_buffer_size(value)?;
        Ok(())
    }

    fn drop(&mut self, sock: Resource<UdpSocket>) -> wasmtime::Result<()> {
        self.table
            .delete(sock)
            .context("failed to delete socket resource from table")?;
        Ok(())
    }
}

use super::is_addr_allowed;
use crate::TrappableError;
use crate::p3::bindings::sockets::types::{
    ErrorCode, HostUdpSocket, HostUdpSocketWithStore, IpAddressFamily, IpSocketAddress,
};
use crate::p3::sockets::{SocketResult, WasiSockets};
use crate::sockets::{MAX_UDP_DATAGRAM_SIZE, SocketAddrUse, UdpSocket, WasiSocketsCtxView};
use std::net::SocketAddr;
use wasmtime::component::{Accessor, Resource, ResourceTable};
use wasmtime::error::Context as _;

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

impl<T> HostUdpSocketWithStore<T> for WasiSockets {
    async fn send(
        store: &Accessor<T, Self>,
        socket: Resource<UdpSocket>,
        data: Vec<u8>,
        remote_address: Option<IpSocketAddress>,
    ) -> SocketResult<()> {
        if data.len() > MAX_UDP_DATAGRAM_SIZE {
            return Err(ErrorCode::DatagramTooLarge.into());
        }
        let remote_address = remote_address.map(SocketAddr::from);

        if let Some(addr) = remote_address {
            if !is_addr_allowed(store, addr, SocketAddrUse::UdpOutgoingDatagram).await {
                return Err(ErrorCode::AccessDenied.into());
            }
        }

        // If this socket is not yet explicitly bound then part of this
        // operation is performing an implicit bind. This is done for a few
        // reasons:
        //
        // * The host's `socket_addr_check` callback, if any, gets an
        //   opportunity to have an opinion about the implicit use of this
        //   address.
        // * On the first send the OS will automatically assign a free local
        //   port. However, if the `send` syscall failed, we can't reliably know
        //   which state the socket is in at the kernel level and our own
        //   state bookkeeping may have become out-of-sync.
        //
        // To avoid that, we perform the implicit bind ourselves here. This way,
        // we always leave the socket in a consistent state: Bound.
        //
        // Note that implicit binding only happens when the socket isn't bound
        // and the remote address is `Some`. If the remote address is `None` and
        // we're unbound then that's an invalid state that `send_p3` below will
        // reject.
        let (needs_implicit_bind, family) = store.with(|mut view| -> SocketResult<_> {
            let socket = get_socket_mut(view.get().table, &socket)?;
            let needs_implicit_bind = !socket.is_bound() && remote_address.is_some();
            Ok((needs_implicit_bind, socket.address_family()))
        })?;
        if needs_implicit_bind {
            let implicit_addr = crate::sockets::util::implicit_bind_addr(family);
            if !is_addr_allowed(store, implicit_addr, SocketAddrUse::UdpBind).await {
                return Err(ErrorCode::AccessDenied.into());
            }
            store.with(|mut view| -> SocketResult<_> {
                let socket = get_socket_mut(view.get().table, &socket)?;
                socket.bind(implicit_addr)?;
                socket.finish_bind()?;
                Ok(())
            })?;
        }

        let fut = store.with(|mut view| {
            get_socket_mut(view.get().table, &socket).map(|sock| sock.send_p3(data, remote_address))
        })?;
        fut.await?;
        Ok(())
    }

    async fn receive(
        store: &Accessor<T, Self>,
        socket: Resource<UdpSocket>,
    ) -> SocketResult<(Vec<u8>, IpSocketAddress)> {
        let fut = store
            .with(|mut view| get_socket(view.get().table, &socket).map(|sock| sock.receive_p3()))?;
        let (result, addr) = fut.await?;
        Ok((result, addr.into()))
    }
}

impl HostUdpSocket for WasiSocketsCtxView<'_> {
    async fn bind(
        &mut self,
        socket: Resource<UdpSocket>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let local_address = SocketAddr::from(local_address);
        if !(self.ctx.socket_addr_check)(local_address, SocketAddrUse::UdpBind).await {
            return Err(ErrorCode::AccessDenied.into());
        }
        let socket = get_socket_mut(self.table, &socket)?;
        socket.bind(local_address)?;
        socket.finish_bind()?;
        Ok(())
    }

    async fn connect(
        &mut self,
        socket: Resource<UdpSocket>,
        remote_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let remote_address = SocketAddr::from(remote_address);
        if !(self.ctx.socket_addr_check)(remote_address, SocketAddrUse::UdpConnect).await {
            return Err(ErrorCode::AccessDenied.into());
        }
        let socket = get_socket_mut(self.table, &socket)?;
        socket.connect_p3(remote_address)?;
        Ok(())
    }

    fn create(&mut self, address_family: IpAddressFamily) -> SocketResult<Resource<UdpSocket>> {
        let socket = UdpSocket::new(self.ctx, address_family.into())?;
        self.table
            .push(socket)
            .context("failed to push socket resource to table")
            .map_err(TrappableError::trap)
    }

    fn disconnect(&mut self, socket: Resource<UdpSocket>) -> SocketResult<()> {
        let socket = get_socket_mut(self.table, &socket)?;
        socket.disconnect()?;
        Ok(())
    }

    fn get_local_address(&mut self, socket: Resource<UdpSocket>) -> SocketResult<IpSocketAddress> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.local_address()?.into())
    }

    fn get_remote_address(&mut self, socket: Resource<UdpSocket>) -> SocketResult<IpSocketAddress> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.remote_address()?.into())
    }

    fn get_address_family(
        &mut self,
        socket: Resource<UdpSocket>,
    ) -> wasmtime::Result<IpAddressFamily> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.address_family().into())
    }

    fn get_unicast_hop_limit(&mut self, socket: Resource<UdpSocket>) -> SocketResult<u8> {
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

    fn get_receive_buffer_size(&mut self, socket: Resource<UdpSocket>) -> SocketResult<u64> {
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

    fn get_send_buffer_size(&mut self, socket: Resource<UdpSocket>) -> SocketResult<u64> {
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

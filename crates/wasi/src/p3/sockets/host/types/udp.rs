use crate::TrappableError;
use crate::p3::bindings::sockets::types::{
    HostUdpSocket, HostUdpSocketWithStore, IpAddressFamily, IpSocketAddress,
};
use crate::p3::sockets::{SocketResult, WasiSockets};
use crate::sockets::{UdpSocket, WasiSocketsCtxView};
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
        store
            .with(|mut view| view.get().send(&socket, data, remote_address))
            .await
    }

    async fn receive(
        store: &Accessor<T, Self>,
        socket: Resource<UdpSocket>,
    ) -> SocketResult<(Vec<u8>, IpSocketAddress)> {
        store.with(|mut view| view.get().receive(&socket)).await
    }
}

impl WasiSocketsCtxView<'_> {
    fn send(
        &mut self,
        socket: &Resource<UdpSocket>,
        data: Vec<u8>,
        remote_address: Option<IpSocketAddress>,
    ) -> impl Future<Output = SocketResult<()>> + use<> {
        let socket = get_socket_mut(self.table, socket);
        let fut = socket.map(|s| s.send(data, remote_address.map(SocketAddr::from)));
        async move {
            fut?.await?;
            Ok(())
        }
    }

    fn receive(
        &mut self,
        socket: &Resource<UdpSocket>,
    ) -> impl Future<Output = SocketResult<(Vec<u8>, IpSocketAddress)>> + use<> {
        let fut = get_socket_mut(self.table, socket).map(|s| s.recv());
        async move {
            let (data, addr) = fut?.await?;
            Ok((data, addr.into()))
        }
    }
}

impl HostUdpSocket for WasiSocketsCtxView<'_> {
    async fn bind(
        &mut self,
        socket: Resource<UdpSocket>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let local_address = SocketAddr::from(local_address);
        let socket = get_socket_mut(self.table, &socket)?;
        socket.bind(local_address).await?;
        Ok(())
    }

    async fn connect(
        &mut self,
        socket: Resource<UdpSocket>,
        remote_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let remote_address = SocketAddr::from(remote_address);
        let socket = get_socket_mut(self.table, &socket)?;
        socket.connect(remote_address).await?;
        Ok(())
    }

    async fn create(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<UdpSocket>> {
        let socket = UdpSocket::new(self.ctx, address_family.into()).await?;
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
        let sock = get_socket_mut(self.table, &socket)?;
        Ok(sock.local_address()?.into())
    }

    fn get_remote_address(&mut self, socket: Resource<UdpSocket>) -> SocketResult<IpSocketAddress> {
        let sock = get_socket_mut(self.table, &socket)?;
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

mod named {
    use crate::p3::bindings::named_imports::wasi::sockets::types::{
        HostUdpSocket, HostUdpSocketWithStore,
    };
    use crate::p3::bindings::sockets::types::{IpAddressFamily, IpSocketAddress};
    use crate::p3::sockets::SocketResult;
    use crate::sockets::{UdpSocket, WasiSocketsNamed, WasiSocketsNamedView};
    use crate::{NamedId, WasiCtxNamedView};
    use wasmtime::component::{Accessor, Resource};

    impl<T, U> HostUdpSocketWithStore<U> for WasiSocketsNamed<T>
    where
        T: WasiSocketsNamedView,
    {
        async fn send(
            store: &Accessor<U, Self>,
            id: NamedId,
            socket: Resource<UdpSocket>,
            data: Vec<u8>,
            remote_address: Option<IpSocketAddress>,
        ) -> SocketResult<()> {
            store
                .with(|mut view| view.get().0.sockets(id).send(&socket, data, remote_address))
                .await
        }

        async fn receive(
            store: &Accessor<U, Self>,
            id: NamedId,
            socket: Resource<UdpSocket>,
        ) -> SocketResult<(Vec<u8>, IpSocketAddress)> {
            store
                .with(|mut view| view.get().0.sockets(id).receive(&socket))
                .await
        }
    }

    impl<T> HostUdpSocket for WasiCtxNamedView<'_, T>
    where
        T: WasiSocketsNamedView,
    {
        async fn bind(
            &mut self,
            id: NamedId,
            socket: Resource<UdpSocket>,
            local_address: IpSocketAddress,
        ) -> SocketResult<()> {
            super::HostUdpSocket::bind(&mut self.0.sockets(id), socket, local_address).await
        }

        async fn connect(
            &mut self,
            id: NamedId,
            socket: Resource<UdpSocket>,
            remote_address: IpSocketAddress,
        ) -> SocketResult<()> {
            super::HostUdpSocket::connect(&mut self.0.sockets(id), socket, remote_address).await
        }

        async fn create(
            &mut self,
            id: NamedId,
            address_family: IpAddressFamily,
        ) -> SocketResult<Resource<UdpSocket>> {
            super::HostUdpSocket::create(&mut self.0.sockets(id), address_family).await
        }

        fn disconnect(&mut self, id: NamedId, socket: Resource<UdpSocket>) -> SocketResult<()> {
            super::HostUdpSocket::disconnect(&mut self.0.sockets(id), socket)
        }

        fn get_local_address(
            &mut self,
            id: NamedId,
            socket: Resource<UdpSocket>,
        ) -> SocketResult<IpSocketAddress> {
            super::HostUdpSocket::get_local_address(&mut self.0.sockets(id), socket)
        }

        fn get_remote_address(
            &mut self,
            id: NamedId,
            socket: Resource<UdpSocket>,
        ) -> SocketResult<IpSocketAddress> {
            super::HostUdpSocket::get_remote_address(&mut self.0.sockets(id), socket)
        }

        fn get_address_family(
            &mut self,
            id: NamedId,
            socket: Resource<UdpSocket>,
        ) -> wasmtime::Result<IpAddressFamily> {
            super::HostUdpSocket::get_address_family(&mut self.0.sockets(id), socket)
        }

        fn get_unicast_hop_limit(
            &mut self,
            id: NamedId,
            socket: Resource<UdpSocket>,
        ) -> SocketResult<u8> {
            super::HostUdpSocket::get_unicast_hop_limit(&mut self.0.sockets(id), socket)
        }

        fn set_unicast_hop_limit(
            &mut self,
            id: NamedId,
            socket: Resource<UdpSocket>,
            value: u8,
        ) -> SocketResult<()> {
            super::HostUdpSocket::set_unicast_hop_limit(&mut self.0.sockets(id), socket, value)
        }

        fn get_receive_buffer_size(
            &mut self,
            id: NamedId,
            socket: Resource<UdpSocket>,
        ) -> SocketResult<u64> {
            super::HostUdpSocket::get_receive_buffer_size(&mut self.0.sockets(id), socket)
        }

        fn set_receive_buffer_size(
            &mut self,
            id: NamedId,
            socket: Resource<UdpSocket>,
            value: u64,
        ) -> SocketResult<()> {
            super::HostUdpSocket::set_receive_buffer_size(&mut self.0.sockets(id), socket, value)
        }

        fn get_send_buffer_size(
            &mut self,
            id: NamedId,
            socket: Resource<UdpSocket>,
        ) -> SocketResult<u64> {
            super::HostUdpSocket::get_send_buffer_size(&mut self.0.sockets(id), socket)
        }

        fn set_send_buffer_size(
            &mut self,
            id: NamedId,
            socket: Resource<UdpSocket>,
            value: u64,
        ) -> SocketResult<()> {
            super::HostUdpSocket::set_send_buffer_size(&mut self.0.sockets(id), socket, value)
        }

        fn drop(&mut self, id: NamedId, sock: Resource<UdpSocket>) -> wasmtime::Result<()> {
            super::HostUdpSocket::drop(&mut self.0.sockets(id), sock)
        }
    }
}

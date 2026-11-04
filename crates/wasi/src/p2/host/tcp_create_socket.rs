use crate::WasiPermitType;
use crate::p2::bindings::sockets::{network::IpAddressFamily, tcp::ErrorCode, tcp_create_socket};
use crate::p2::{SocketResult, TcpSocket};
use crate::sockets::{SocketAddressFamily, TcpSocket as P3Socket, WasiSocketsCtxView};
use wasmtime::component::Resource;

impl tcp_create_socket::Host for WasiSocketsCtxView<'_> {
    async fn create_tcp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<TcpSocket>> {
        let permit = self
            .hooks
            .acquire(WasiPermitType::Socket)
            .await
            .map_err(|_| ErrorCode::NewSocketLimit)?;
        let socket = P3Socket::new(self.ctx, address_family.into(), permit, self.hooks.owned())?;
        let socket = self.table.push(TcpSocket::new(socket))?;
        Ok(socket)
    }
}

impl From<IpAddressFamily> for SocketAddressFamily {
    fn from(family: IpAddressFamily) -> SocketAddressFamily {
        match family {
            IpAddressFamily::Ipv4 => Self::Ipv4,
            IpAddressFamily::Ipv6 => Self::Ipv6,
        }
    }
}

pub mod sync {
    use wasmtime::component::Resource;

    use crate::p2::{
        SocketResult, TcpSocket,
        bindings::sockets::tcp_create_socket::Host as AsyncHost,
        bindings::sync::sockets::{tcp::IpAddressFamily, tcp_create_socket},
    };
    use crate::runtime::in_tokio;
    use crate::sockets::WasiSocketsCtxView;

    impl tcp_create_socket::Host for WasiSocketsCtxView<'_> {
        fn create_tcp_socket(
            &mut self,
            address_family: IpAddressFamily,
        ) -> SocketResult<Resource<TcpSocket>> {
            in_tokio(async { AsyncHost::create_tcp_socket(self, address_family).await })
        }
    }
}

mod named {
    use crate::p2::SocketError;
    use crate::p2::bindings::named_imports::wasi::sockets::tcp_create_socket;
    use crate::p2::bindings::sockets::network::ErrorCode;
    use crate::p2::bindings::sockets::network::IpAddressFamily;
    use crate::p2::{SocketResult, TcpSocket};
    use crate::sockets::WasiSocketsNamedView;
    use crate::{NamedId, WasiCtxNamedView};
    use wasmtime::component::Resource;

    impl<T> tcp_create_socket::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiSocketsNamedView,
    {
        fn convert_error_code(&mut self, err: SocketError) -> wasmtime::Result<ErrorCode> {
            err.downcast()
        }

        async fn create_tcp_socket(
            &mut self,
            id: NamedId,
            address_family: IpAddressFamily,
        ) -> SocketResult<Resource<TcpSocket>> {
            super::tcp_create_socket::Host::create_tcp_socket(
                &mut self.0.sockets(id),
                address_family,
            )
            .await
        }
    }
}

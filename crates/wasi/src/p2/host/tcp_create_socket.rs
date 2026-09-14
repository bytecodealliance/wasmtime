use crate::p2::bindings::{sockets::network::IpAddressFamily, sockets::tcp_create_socket};
use crate::p2::{SocketResult, TcpSocket};
use crate::sockets::{SocketAddressFamily, TcpSocket as P3Socket, WasiSocketsCtxView};
use wasmtime::component::Resource;

impl tcp_create_socket::Host for WasiSocketsCtxView<'_> {
    fn create_tcp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<TcpSocket>> {
        let socket = P3Socket::new(self.ctx, address_family.into())?;
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

        fn create_tcp_socket(
            &mut self,
            id: NamedId,
            address_family: IpAddressFamily,
        ) -> SocketResult<Resource<TcpSocket>> {
            super::tcp_create_socket::Host::create_tcp_socket(
                &mut self.0.sockets(id),
                address_family,
            )
        }
    }
}

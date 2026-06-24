use crate::p2::bindings::{sockets::network::IpAddressFamily, sockets::udp_create_socket};
use crate::p2::{SocketResult, UdpSocket};
use crate::sockets::UdpSocket as P3Socket;
use crate::sockets::WasiSocketsCtxView;
use wasmtime::component::Resource;

impl udp_create_socket::Host for WasiSocketsCtxView<'_> {
    async fn create_udp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<UdpSocket>> {
        let inner = P3Socket::new(self.ctx, address_family.into()).await?;
        let socket = self.table.push(UdpSocket::new(inner))?;
        Ok(socket)
    }
}

pub mod sync {
    use wasmtime::component::Resource;

    use crate::p2::{
        SocketResult, UdpSocket,
        bindings::sockets::udp_create_socket::Host as AsyncHost,
        bindings::sync::sockets::{udp::IpAddressFamily, udp_create_socket},
    };
    use crate::runtime::in_tokio;
    use crate::sockets::WasiSocketsCtxView;

    impl udp_create_socket::Host for WasiSocketsCtxView<'_> {
        fn create_udp_socket(
            &mut self,
            address_family: IpAddressFamily,
        ) -> SocketResult<Resource<UdpSocket>> {
            in_tokio(async { AsyncHost::create_udp_socket(self, address_family).await })
        }
    }
}

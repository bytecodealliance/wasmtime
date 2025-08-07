use crate::p2::SocketResult;
use crate::p2::bindings::{sockets::network::IpAddressFamily, sockets::tcp_create_socket};
use crate::sockets::{SocketAddressFamily, TcpSocket, WasiSocketsCtxView};
use wasmtime::component::Resource;

impl tcp_create_socket::Host for WasiSocketsCtxView<'_> {
    fn create_tcp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<TcpSocket>> {
        let socket = TcpSocket::new(self.ctx, address_family.into())?;
        let socket = self.table.push(socket)?;
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

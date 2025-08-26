use crate::p2::SocketResult;
use crate::p2::bindings::{sockets::network::IpAddressFamily, sockets::udp_create_socket};
use crate::sockets::UdpSocket;
use crate::sockets::WasiSocketsCtxView;
use wasmtime::component::Resource;

impl udp_create_socket::Host for WasiSocketsCtxView<'_> {
    fn create_udp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<UdpSocket>> {
        let socket = UdpSocket::new(self.ctx, address_family.into())?;
        let socket = self.table.push(socket)?;
        Ok(socket)
    }
}

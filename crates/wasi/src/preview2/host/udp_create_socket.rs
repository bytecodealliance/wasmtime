use crate::preview2::bindings::{sockets::network::IpAddressFamily, sockets::udp_create_socket};
use crate::preview2::udp::UdpSocket;
use crate::preview2::{SocketResult, WasiView};
use wasmtime::component::Resource;

impl<T: WasiView> udp_create_socket::Host for T {
    fn create_udp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<UdpSocket>> {
        let socket = UdpSocket::new(address_family.into(), self.ctx().udp_addr_check.clone())?;
        let socket = self.table_mut().push(socket)?;
        Ok(socket)
    }
}

use crate::p2::bindings::{sockets::network::IpAddressFamily, sockets::udp_create_socket};
use crate::p2::udp::UdpSocket;
use crate::p2::{IoView, SocketResult, WasiImpl, WasiView};
use wasmtime::component::Resource;

impl<T> udp_create_socket::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn create_udp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<UdpSocket>> {
        let socket = UdpSocket::new(address_family.into())?;
        let socket = self.table().push(socket)?;
        Ok(socket)
    }
}

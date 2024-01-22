use crate::preview2::bindings::{sockets::network::IpAddressFamily, sockets::tcp_create_socket};
use crate::preview2::tcp::TcpSocket;
use crate::preview2::{SocketResult, WasiView};
use wasmtime::component::Resource;

impl<T: WasiView> tcp_create_socket::Host for T {
    fn create_tcp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<TcpSocket>> {
        let socket = TcpSocket::new(address_family.into())?;
        let socket = self.table().push(socket)?;
        Ok(socket)
    }
}

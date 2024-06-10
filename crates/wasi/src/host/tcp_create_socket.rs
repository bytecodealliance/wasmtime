use crate::bindings::{sockets::network::IpAddressFamily, sockets::tcp_create_socket};
use crate::tcp::TcpSocket;
use crate::{SocketResult, WasiView};
use wasmtime::component::Resource;

impl tcp_create_socket::Host for dyn WasiView + '_ {
    fn create_tcp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<TcpSocket>> {
        let socket = TcpSocket::new(address_family.into())?;
        let socket = self.table().push(socket)?;
        Ok(socket)
    }
}

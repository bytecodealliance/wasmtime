use crate::preview2::bindings::{
    sockets::network::{self, IpAddressFamily},
    sockets::tcp_create_socket,
};
use crate::preview2::tcp::TcpSocket;
use crate::preview2::WasiView;
use wasmtime::component::Resource;

impl<T: WasiView> tcp_create_socket::Host for T {
    fn create_tcp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> Result<Resource<TcpSocket>, network::Error> {
        let socket = TcpSocket::new(address_family.into())?;
        let socket = self.table_mut().push_resource(socket)?;
        Ok(socket)
    }
}

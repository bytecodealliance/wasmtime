use crate::p2::SocketResult;
use crate::p2::bindings::{sockets::network::IpAddressFamily, sockets::tcp_create_socket};
use crate::p2::tcp::TcpSocket;
use crate::sockets::WasiSocketsCtxView;
use wasmtime::component::Resource;

impl tcp_create_socket::Host for WasiSocketsCtxView<'_> {
    fn create_tcp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<TcpSocket>> {
        let socket = TcpSocket::new(address_family.into())?;
        let socket = self.table.push(socket)?;
        Ok(socket)
    }
}

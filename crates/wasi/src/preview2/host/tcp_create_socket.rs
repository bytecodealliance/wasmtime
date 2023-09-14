use crate::preview2::bindings::{
    sockets::network::{self, IpAddressFamily},
    sockets::tcp::TcpSocket,
    sockets::tcp_create_socket,
};
use crate::preview2::tcp::{HostTcpSocketState, TableTcpSocketExt};
use crate::preview2::WasiView;

impl<T: WasiView> tcp_create_socket::Host for T {
    fn create_tcp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> Result<TcpSocket, network::Error> {
        let socket = HostTcpSocketState::new(address_family.into())?;
        let socket = self.table_mut().push_tcp_socket(socket)?;
        Ok(socket)
    }
}

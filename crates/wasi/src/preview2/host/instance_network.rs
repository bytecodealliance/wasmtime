use crate::preview2::bindings::sockets::instance_network;
use crate::preview2::network::NetworkHandle;
use crate::preview2::WasiView;
use wasmtime::component::Resource;

impl<T: WasiView> instance_network::Host for T {
    fn instance_network(&mut self) -> Result<Resource<NetworkHandle>, anyhow::Error> {
        let network = NetworkHandle {
            socket_addr_check: self.ctx().socket_addr_check.clone(),
        };
        let network = self.table_mut().push(network)?;
        Ok(network)
    }
}

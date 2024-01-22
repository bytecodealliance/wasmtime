use crate::preview2::bindings::sockets::instance_network;
use crate::preview2::network::NetworkResource;
use crate::preview2::WasiView;
use wasmtime::component::Resource;

impl<T: WasiView> instance_network::Host for T {
    fn instance_network(&mut self) -> Result<Resource<NetworkResource>, anyhow::Error> {
        let network = NetworkResource::new();
        let network = self.table_mut().push(network)?;
        Ok(network)
    }
}

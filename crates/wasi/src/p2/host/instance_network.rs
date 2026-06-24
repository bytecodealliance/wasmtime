use crate::p2::bindings::sockets::instance_network;
use crate::p2::network::Network;
use crate::sockets::WasiSocketsCtxView;
use wasmtime::component::Resource;

impl instance_network::Host for WasiSocketsCtxView<'_> {
    fn instance_network(&mut self) -> Result<Resource<Network>, wasmtime::Error> {
        let network = Network { _priv: () };
        let network = self.table.push(network)?;
        Ok(network)
    }
}

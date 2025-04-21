use crate::net::Network;
use crate::p2::bindings::sockets::instance_network;
use crate::p2::{IoView, WasiImpl, WasiView};
use wasmtime::component::Resource;

impl<T> instance_network::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn instance_network(&mut self) -> Result<Resource<Network>, anyhow::Error> {
        let network = Network {
            socket_addr_check: self.ctx().socket_addr_check.clone(),
            allow_ip_name_lookup: self.ctx().allowed_network_uses.ip_name_lookup,
        };
        let network = self.table().push(network)?;
        Ok(network)
    }
}

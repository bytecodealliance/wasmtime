use crate::p2::bindings::sockets::instance_network;
use crate::p2::network::Network;
use crate::sockets::WasiSocketsCtxView;
use wasmtime::component::Resource;

impl instance_network::Host for WasiSocketsCtxView<'_> {
    fn instance_network(&mut self) -> Result<Resource<Network>, anyhow::Error> {
        let network = Network {
            socket_addr_check: self.ctx.socket_addr_check.clone(),
            allow_ip_name_lookup: self.ctx.allowed_network_uses.ip_name_lookup,
        };
        let network = self.table.push(network)?;
        Ok(network)
    }
}

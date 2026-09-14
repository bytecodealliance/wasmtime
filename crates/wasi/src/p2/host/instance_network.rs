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

mod named {
    use crate::p2::bindings::named_imports::wasi::sockets::instance_network;
    use crate::p2::network::Network;
    use crate::sockets::WasiSocketsNamedView;
    use crate::{NamedId, WasiCtxNamedView};
    use wasmtime::component::Resource;

    impl<T> instance_network::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiSocketsNamedView,
    {
        fn instance_network(&mut self, id: NamedId) -> Result<Resource<Network>, wasmtime::Error> {
            super::instance_network::Host::instance_network(&mut self.0.sockets(id))
        }
    }
}

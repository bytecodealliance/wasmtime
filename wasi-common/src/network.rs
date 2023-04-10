//! IP Networks.

use crate::Error;
use std::any::Any;

/// An IP network.
#[async_trait::async_trait]
pub trait WasiNetwork: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    fn pool(&self) -> &cap_std::net::Pool;
}

pub trait TableNetworkExt {
    fn get_network(&self, fd: u32) -> Result<&dyn WasiNetwork, Error>;
    fn get_network_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiNetwork>, Error>;
}
impl TableNetworkExt for crate::table::Table {
    fn get_network(&self, fd: u32) -> Result<&dyn WasiNetwork, Error> {
        self.get::<Box<dyn WasiNetwork>>(fd).map(|f| f.as_ref())
    }
    fn get_network_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiNetwork>, Error> {
        self.get_mut::<Box<dyn WasiNetwork>>(fd)
    }
}

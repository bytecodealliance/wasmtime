use crate::preview2::{Table, TableError};
use cap_std::net::Pool;

pub(crate) struct HostNetworkState(pub(crate) Pool);

impl HostNetworkState {
    pub fn new(pool: Pool) -> Self {
        Self(pool)
    }
}

pub(crate) trait TableNetworkExt {
    fn push_network(&mut self, network: HostNetworkState) -> Result<u32, TableError>;
    fn delete_network(&mut self, fd: u32) -> Result<HostNetworkState, TableError>;
    fn is_network(&self, fd: u32) -> bool;
    fn get_network(&self, fd: u32) -> Result<&HostNetworkState, TableError>;
}

impl TableNetworkExt for Table {
    fn push_network(&mut self, network: HostNetworkState) -> Result<u32, TableError> {
        self.push(Box::new(network))
    }
    fn delete_network(&mut self, fd: u32) -> Result<HostNetworkState, TableError> {
        self.delete(fd)
    }
    fn is_network(&self, fd: u32) -> bool {
        self.is::<HostNetworkState>(fd)
    }
    fn get_network(&self, fd: u32) -> Result<&HostNetworkState, TableError> {
        self.get(fd)
    }
}

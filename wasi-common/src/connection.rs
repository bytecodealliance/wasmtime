//! Socket connections.

use crate::Error;
use bitflags::bitflags;
use std::any::Any;

/// A socket connection.
#[async_trait::async_trait]
pub trait WasiConnection: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    async fn sock_recv<'a>(
        &mut self,
        _ri_data: &mut [std::io::IoSliceMut<'a>],
        _ri_flags: RiFlags,
    ) -> Result<(u64, RoFlags), Error>;

    async fn sock_send<'a>(
        &mut self,
        _si_data: &[std::io::IoSlice<'a>],
        _si_flags: SiFlags,
    ) -> Result<u64, Error>;

    async fn sock_shutdown(&mut self, _how: SdFlags) -> Result<(), Error>;

    fn get_nonblocking(&mut self) -> Result<bool, Error>;

    fn set_nonblocking(&mut self, _flag: bool) -> Result<(), Error>;

    async fn readable(&self) -> Result<(), Error>;

    async fn writable(&self) -> Result<(), Error>;
}

bitflags! {
    pub struct SdFlags: u32 {
        const RD = 0b1;
        const WR = 0b10;
    }
}

bitflags! {
    pub struct SiFlags: u32 {
    }
}

bitflags! {
    pub struct RiFlags: u32 {
        const RECV_PEEK    = 0b1;
        const RECV_WAITALL = 0b10;
    }
}

bitflags! {
    pub struct RoFlags: u32 {
        const RECV_DATA_TRUNCATED = 0b1;
    }
}

pub trait TableConnectionExt {
    fn get_connection(&self, fd: u32) -> Result<&dyn WasiConnection, Error>;
    fn get_connection_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiConnection>, Error>;
}
impl TableConnectionExt for crate::table::Table {
    fn get_connection(&self, fd: u32) -> Result<&dyn WasiConnection, Error> {
        self.get::<Box<dyn WasiConnection>>(fd).map(|f| f.as_ref())
    }
    fn get_connection_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiConnection>, Error> {
        self.get_mut::<Box<dyn WasiConnection>>(fd)
    }
}

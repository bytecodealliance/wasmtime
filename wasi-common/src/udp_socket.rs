//! UDP sockets.

use crate::Error;
use bitflags::bitflags;
use std::any::Any;

/// A UDP socket.
#[async_trait::async_trait]
pub trait WasiUdpSocket: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    async fn sock_recv<'a>(
        &mut self,
        ri_data: &mut [std::io::IoSliceMut<'a>],
        ri_flags: RiFlags,
    ) -> Result<(u64, RoFlags), Error>;

    async fn sock_send<'a>(&mut self, si_data: &[std::io::IoSlice<'a>]) -> Result<u64, Error>;

    fn set_nonblocking(&mut self, flag: bool) -> Result<(), Error>;

    async fn readable(&self) -> Result<(), Error>;

    async fn writable(&self) -> Result<(), Error>;
}

bitflags! {
    pub struct RoFlags: u32 {
        const RECV_DATA_TRUNCATED = 0b1;
    }
}

bitflags! {
    pub struct RiFlags: u32 {
        const RECV_PEEK    = 0b1;
        const RECV_WAITALL = 0b10;
    }
}

pub trait TableUdpSocketExt {
    fn get_udp_socket(&self, fd: u32) -> Result<&dyn WasiUdpSocket, Error>;
    fn get_udp_socket_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiUdpSocket>, Error>;
}
impl TableUdpSocketExt for crate::table::Table {
    fn get_udp_socket(&self, fd: u32) -> Result<&dyn WasiUdpSocket, Error> {
        self.get::<Box<dyn WasiUdpSocket>>(fd).map(|f| f.as_ref())
    }
    fn get_udp_socket_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiUdpSocket>, Error> {
        self.get_mut::<Box<dyn WasiUdpSocket>>(fd)
    }
}

//! TCP sockets.

use crate::Error;
use crate::{InputStream, OutputStream};
use bitflags::bitflags;
use std::any::Any;
use std::net::SocketAddr;

/// A TCP socket.
#[async_trait::async_trait]
pub trait WasiTcpSocket: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    async fn accept(
        &mut self,
        nonblocking: bool,
    ) -> Result<
        (
            Box<dyn WasiTcpSocket>,
            Box<dyn InputStream>,
            Box<dyn OutputStream>,
            SocketAddr,
        ),
        Error,
    >;

    async fn sock_shutdown(&mut self, how: SdFlags) -> Result<(), Error>;

    fn set_nonblocking(&mut self, flag: bool) -> Result<(), Error>;

    async fn readable(&self) -> Result<(), Error>;

    async fn writable(&self) -> Result<(), Error>;
}

bitflags! {
    pub struct SdFlags: u32 {
        const READ = 0b1;
        const WRITE = 0b10;
    }
}

pub trait TableTcpSocketExt {
    fn get_tcp_socket(&self, fd: u32) -> Result<&dyn WasiTcpSocket, Error>;
    fn get_tcp_socket_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiTcpSocket>, Error>;
}
impl TableTcpSocketExt for crate::table::Table {
    fn get_tcp_socket(&self, fd: u32) -> Result<&dyn WasiTcpSocket, Error> {
        self.get::<Box<dyn WasiTcpSocket>>(fd).map(|f| f.as_ref())
    }
    fn get_tcp_socket_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiTcpSocket>, Error> {
        self.get_mut::<Box<dyn WasiTcpSocket>>(fd)
    }
}

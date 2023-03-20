//! TCP sockets.

use crate::Error;
use crate::{InputStream, OutputStream, WasiNetwork};
use cap_std::net::{Shutdown, SocketAddr};
use std::any::Any;

/// A TCP socket.
#[async_trait::async_trait]
pub trait WasiTcpSocket: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    /// Return the host file descriptor so that it can be polled with a host poll.
    fn pollable(&self) -> rustix::fd::BorrowedFd;

    async fn listen(&self, network: &dyn WasiNetwork) -> Result<(), Error>;

    async fn accept(
        &self,
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

    async fn connect(
        &self,
        network: &dyn WasiNetwork,
        remote_address: SocketAddr,
    ) -> Result<(Box<dyn InputStream>, Box<dyn OutputStream>), Error>;

    async fn bind(&self, network: &dyn WasiNetwork, local_address: SocketAddr)
        -> Result<(), Error>;

    async fn shutdown(&self, how: Shutdown) -> Result<(), Error>;

    fn local_address(&self) -> Result<SocketAddr, Error>;
    fn remote_address(&self) -> Result<SocketAddr, Error>;

    fn nodelay(&self) -> Result<bool, Error>;
    fn set_nodelay(&self, value: bool) -> Result<(), Error>;
    fn v6_only(&self) -> Result<bool, Error>;
    fn set_v6_only(&self, value: bool) -> Result<(), Error>;

    fn set_nonblocking(&mut self, flag: bool) -> Result<(), Error>;

    async fn readable(&self) -> Result<(), Error>;

    async fn writable(&self) -> Result<(), Error>;
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

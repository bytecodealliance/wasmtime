//! TCP socket listeners.

use crate::connection::WasiConnection;
use crate::Error;
use crate::WasiListener;
use crate::{InputStream, OutputStream};
use std::any::Any;
use std::net::SocketAddr;

/// A TCP socket listener.
#[async_trait::async_trait]
pub trait WasiTcpListener: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    async fn accept(
        &mut self,
        nonblocking: bool,
    ) -> Result<
        (
            Box<dyn WasiConnection>,
            Box<dyn InputStream>,
            Box<dyn OutputStream>,
            SocketAddr,
        ),
        Error,
    >;

    fn set_nonblocking(&mut self, flag: bool) -> Result<(), Error>;

    fn into_listener(self) -> Box<dyn WasiListener>;
}

pub trait TableTcpListenerExt {
    fn get_tcp_listener(&self, fd: u32) -> Result<&dyn WasiTcpListener, Error>;
    fn get_tcp_listener_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiTcpListener>, Error>;
}
impl TableTcpListenerExt for crate::table::Table {
    fn get_tcp_listener(&self, fd: u32) -> Result<&dyn WasiTcpListener, Error> {
        self.get::<Box<dyn WasiTcpListener>>(fd).map(|f| f.as_ref())
    }
    fn get_tcp_listener_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiTcpListener>, Error> {
        self.get_mut::<Box<dyn WasiTcpListener>>(fd)
    }
}

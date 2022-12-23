use crate::connection::WasiConnection;
/// Socket listeners.
use crate::Error;
use std::any::Any;

/// A socket listener.
#[async_trait::async_trait]
pub trait WasiListener: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    async fn sock_accept(&mut self, nonblocking: bool) -> Result<Box<dyn WasiConnection>, Error>;

    fn get_nonblocking(&mut self) -> Result<bool, Error>;

    fn set_nonblocking(&mut self, _flag: bool) -> Result<(), Error>;
}

pub trait TableListenerExt {
    fn get_listener(&self, fd: u32) -> Result<&dyn WasiListener, Error>;
    fn get_listener_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiListener>, Error>;
}
impl TableListenerExt for crate::table::Table {
    fn get_listener(&self, fd: u32) -> Result<&dyn WasiListener, Error> {
        self.get::<Box<dyn WasiListener>>(fd).map(|f| f.as_ref())
    }
    fn get_listener_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiListener>, Error> {
        self.get_mut::<Box<dyn WasiListener>>(fd)
    }
}

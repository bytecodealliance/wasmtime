//! Socket listeners.

use crate::connection::WasiConnection;
use crate::Error;
use crate::{InputStream, OutputStream};
use std::any::Any;

/// A socket listener.
#[async_trait::async_trait]
pub trait WasiListener: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    async fn accept(
        &mut self,
        nonblocking: bool,
    ) -> Result<
        (
            Box<dyn WasiConnection>,
            Box<dyn InputStream>,
            Box<dyn OutputStream>,
        ),
        Error,
    >;

    fn set_nonblocking(&mut self, flag: bool) -> Result<(), Error>;
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

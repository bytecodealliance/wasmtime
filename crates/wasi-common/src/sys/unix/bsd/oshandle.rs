use crate::sys::sys_impl::oshandle::OsHandle;
use crate::wasi::Result;
use std::cell::{RefCell, RefMut};
use std::io;
use std::ops::Deref;
use yanix::dir::Dir;

#[derive(Debug)]
pub(crate) struct OsDirHandle {
    handle: OsHandle,
    dir: RefCell<Dir>,
}

impl OsDirHandle {
    /// Consumes the spcified `OsHandle` and initialises the
    /// underlying `Dir` stream pointer.
    pub(crate) fn new(handle: OsHandle) -> io::Result<Self> {
        let dir = Dir::from(handle.try_clone()?)?;
        let dir = RefCell::new(dir);
        Ok(Self { handle, dir })
    }
    /// Tries clone `self`.
    ///
    /// Note that the `Dir` stream pointer will be reset
    /// to start.
    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        let handle = self.handle.try_clone()?;
        Self::new(handle)
    }
    /// Gets mutable reference to the current dir stream pointer.
    pub(crate) fn dir_stream(&self) -> Result<RefMut<Dir>> {
        Ok(self.dir.borrow_mut())
    }
}

impl Deref for OsDirHandle {
    type Target = OsHandle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

use crate::sys::AsFile;
use std::cell::Cell;
use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::os::unix::prelude::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

pub(crate) use super::sys_impl::oshandle::*;

#[derive(Debug)]
pub(crate) struct OsHandle(Cell<RawFd>);

impl OsHandle {
    /// Tries clone `self`.
    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        let fd = self.as_file().try_clone()?;
        Ok(Self(Cell::new(fd.into_raw_fd())))
    }
    /// Consumes `other` taking the ownership of the underlying
    /// `RawFd` file descriptor.
    ///
    /// Note that the state of `Dir` stream pointer *will* not be carried
    /// across from `other` to `self`.
    pub(crate) fn update_from(&self, other: Self) {
        let new_fd = other.into_raw_fd();
        let old_fd = self.0.get();
        self.0.set(new_fd);
        // We need to remember to close the old_fd.
        unsafe {
            File::from_raw_fd(old_fd);
        }
    }
}

impl Drop for OsHandle {
    fn drop(&mut self) {
        unsafe {
            File::from_raw_fd(self.as_raw_fd());
        }
    }
}

impl AsRawFd for OsHandle {
    fn as_raw_fd(&self) -> RawFd {
        self.0.get()
    }
}

impl FromRawFd for OsHandle {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(Cell::new(fd))
    }
}

impl IntoRawFd for OsHandle {
    fn into_raw_fd(self) -> RawFd {
        // We need to prevent dropping of self
        let wrapped = ManuallyDrop::new(self);
        wrapped.0.get()
    }
}

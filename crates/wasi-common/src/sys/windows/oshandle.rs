use crate::sys::AsFile;
use std::cell::Cell;
use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle, IntoRawHandle, RawHandle};

#[derive(Debug)]
pub(crate) struct RawOsHandle(Cell<RawHandle>);

impl RawOsHandle {
    /// Tries cloning `self`.
    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        let handle = self.as_file()?.try_clone()?;
        Ok(Self(Cell::new(handle.into_raw_handle())))
    }
    /// Consumes `other` taking the ownership of the underlying
    /// `RawHandle` file handle.
    pub(crate) fn update_from(&self, other: Self) {
        let new_handle = other.into_raw_handle();
        let old_handle = self.0.get();
        self.0.set(new_handle);
        // We need to remember to close the old_handle.
        unsafe {
            File::from_raw_handle(old_handle);
        }
    }
}

impl Drop for RawOsHandle {
    fn drop(&mut self) {
        unsafe {
            File::from_raw_handle(self.as_raw_handle());
        }
    }
}

impl AsRawHandle for RawOsHandle {
    fn as_raw_handle(&self) -> RawHandle {
        self.0.get()
    }
}

impl FromRawHandle for RawOsHandle {
    unsafe fn from_raw_handle(handle: RawHandle) -> Self {
        Self(Cell::new(handle))
    }
}

impl IntoRawHandle for RawOsHandle {
    fn into_raw_handle(self) -> RawHandle {
        // We need to prevent dropping of the OsFile
        let wrapped = ManuallyDrop::new(self);
        wrapped.0.get()
    }
}

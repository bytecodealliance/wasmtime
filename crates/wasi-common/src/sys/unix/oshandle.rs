use std::fs::File;
use std::io;
use std::os::unix::prelude::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

#[derive(Debug)]
pub(crate) struct RawOsHandle(File);

impl RawOsHandle {
    /// Tries clone `self`.
    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        let fd = self.0.try_clone()?;
        Ok(unsafe { Self::from_raw_fd(fd.into_raw_fd()) })
    }
    /// Consumes `other` taking the ownership of the underlying
    /// `RawFd` file descriptor.
    ///
    /// Note that the state of `Dir` stream pointer *will* not be carried
    /// across from `other` to `self`.
    pub(crate) fn update_from(&self, _other: Self) {
        panic!("RawOsHandle::update_from should never be issued on Unix!")
    }
}

impl AsRawFd for RawOsHandle {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl IntoRawFd for RawOsHandle {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

impl FromRawFd for RawOsHandle {
    unsafe fn from_raw_fd(raw: RawFd) -> Self {
        Self(File::from_raw_fd(raw))
    }
}

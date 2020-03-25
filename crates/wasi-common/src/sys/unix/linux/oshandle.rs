use std::fs;
use std::ops::Deref;
use std::os::unix::prelude::{AsRawFd, RawFd};

#[derive(Debug)]
pub(crate) struct OsHandle(fs::File);

impl From<fs::File> for OsHandle {
    fn from(file: fs::File) -> Self {
        Self(file)
    }
}

impl AsRawFd for OsHandle {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl Deref for OsHandle {
    type Target = fs::File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

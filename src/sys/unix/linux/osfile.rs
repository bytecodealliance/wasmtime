use std::fs;
use std::ops::{Deref, DerefMut};
use std::os::unix::prelude::{AsRawFd, RawFd};

#[derive(Debug)]
pub(crate) struct OsFile(fs::File);

impl From<fs::File> for OsFile {
    fn from(file: fs::File) -> Self {
        Self(file)
    }
}

impl AsRawFd for OsFile {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl Deref for OsFile {
    type Target = fs::File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OsFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

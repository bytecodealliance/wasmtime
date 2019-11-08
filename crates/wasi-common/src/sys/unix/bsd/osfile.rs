use std::fs;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::os::unix::prelude::{AsRawFd, RawFd};
use std::sync::Mutex;

#[derive(Debug)]
pub(crate) struct DirStream {
    pub(crate) file: ManuallyDrop<fs::File>,
    pub(crate) dir_ptr: *mut libc::DIR,
}

impl Drop for DirStream {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.dir_ptr) };
    }
}

#[derive(Debug)]
pub(crate) struct OsFile {
    pub(crate) file: fs::File,
    pub(crate) dir_stream: Option<Mutex<DirStream>>,
}

impl From<fs::File> for OsFile {
    fn from(file: fs::File) -> Self {
        Self {
            file,
            dir_stream: None,
        }
    }
}

impl AsRawFd for OsFile {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl Deref for OsFile {
    type Target = fs::File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl DerefMut for OsFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}

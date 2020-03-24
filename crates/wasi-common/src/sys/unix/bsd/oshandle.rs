use std::cell::RefCell;
use std::fs;
use std::ops::Deref;
use std::os::unix::prelude::{AsRawFd, RawFd};
use yanix::dir::Dir;

#[derive(Debug)]
pub(crate) struct OsHandle {
    pub(crate) file: fs::File,
    // In case that this `OsHandle` actually refers to a directory,
    // when the client makes a `fd_readdir` syscall on this descriptor,
    // we will need to cache the `libc::DIR` pointer manually in order
    // to be able to seek on it later. While on Linux, this is handled
    // by the OS, BSD Unixes require the client to do this caching.
    //
    // This comes directly from the BSD man pages on `readdir`:
    //   > Values returned by telldir() are good only for the lifetime
    //   > of the DIR pointer, dirp, from which they are derived.
    //   > If the directory is closed and then reopened, prior values
    //   > returned by telldir() will no longer be valid.
    pub(crate) dir: RefCell<Option<Dir>>,
}

impl From<fs::File> for OsHandle {
    fn from(file: fs::File) -> Self {
        Self {
            file,
            dir: RefCell::new(None),
        }
    }
}

impl AsRawFd for OsHandle {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl Deref for OsHandle {
    type Target = fs::File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

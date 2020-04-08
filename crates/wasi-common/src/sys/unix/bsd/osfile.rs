use crate::sys::oshandle::AsFile;
use crate::wasi::Result;
use std::cell::{Cell, RefCell, RefMut};
use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::os::unix::prelude::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use yanix::dir::Dir;

#[derive(Debug)]
pub(crate) struct OsFile {
    fd: Cell<RawFd>,
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
    dir: RefCell<Option<Dir>>,
}

impl OsFile {
    /// Consumes `other` taking the ownership of the underlying
    /// `RawFd` file descriptor.
    ///
    /// Note that the state of `Dir` stream pointer *will* not be carried
    /// across from `other` to `self`.
    pub(crate) fn update_from(&self, other: Self) {
        let new_fd = other.into_raw_fd();
        let old_fd = self.fd.get();
        self.fd.set(new_fd);
        // We need to remember to close the old_fd.
        unsafe {
            File::from_raw_fd(old_fd);
        }
    }
    /// Clones `self` uninitializing the `Dir` stream pointer
    /// (if any).
    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        let fd = self.as_file().try_clone()?;
        Ok(Self {
            fd: Cell::new(fd.into_raw_fd()),
            dir: RefCell::new(None),
        })
    }
    /// Returns the `Dir` stream pointer associated with
    /// this instance.
    ///
    /// Initializes the `Dir` stream pointer if `None`.
    pub(crate) fn dir_stream(&self) -> Result<RefMut<Dir>> {
        if self.dir.borrow().is_none() {
            // We need to duplicate the fd, because `opendir(3)`:
            //     Upon successful return from fdopendir(), the file descriptor is under
            //     control of the system, and if any attempt is made to close the file
            //     descriptor, or to modify the state of the associated description other
            //     than by means of closedir(), readdir(), readdir_r(), or rewinddir(),
            //     the behaviour is undefined.
            let file = self.try_clone()?;
            let d = Dir::from(file)?;
            *self.dir.borrow_mut() = Some(d);
        }
        Ok(RefMut::map(self.dir.borrow_mut(), |dir| {
            dir.as_mut().unwrap()
        }))
    }
}

impl Drop for OsFile {
    fn drop(&mut self) {
        unsafe {
            File::from_raw_fd(self.as_raw_fd());
        }
    }
}

impl AsRawFd for OsFile {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.get()
    }
}

impl FromRawFd for OsFile {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self {
            fd: Cell::new(fd),
            dir: RefCell::new(None),
        }
    }
}

impl IntoRawFd for OsFile {
    fn into_raw_fd(self) -> RawFd {
        // We need to prevent dropping of the OsFile
        let wrapped = ManuallyDrop::new(self);
        wrapped.fd.get()
    }
}

impl AsFile for OsFile {
    fn as_file(&self) -> ManuallyDrop<File> {
        let file = unsafe { File::from_raw_fd(self.fd.get()) };
        ManuallyDrop::new(file)
    }
}

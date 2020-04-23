use crate::sys::oshandle::AsFile;
use crate::wasi::Result;
use std::cell::Cell;
use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::os::unix::prelude::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use yanix::dir::Dir;

#[derive(Debug)]
pub(crate) struct OsFile(Cell<RawFd>);

impl OsFile {
    /// Consumes `other` taking the ownership of the underlying
    /// `RawFd` file descriptor.
    pub(crate) fn update_from(&self, other: Self) {
        let new_fd = other.into_raw_fd();
        let old_fd = self.0.get();
        self.0.set(new_fd);
        // We need to remember to close the old_fd.
        unsafe {
            File::from_raw_fd(old_fd);
        }
    }
    /// Clones `self`.
    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        let fd = self.as_file().try_clone()?;
        Ok(Self(Cell::new(fd.into_raw_fd())))
    }
    /// Returns the `Dir` stream pointer associated with
    /// this instance.
    pub(crate) fn dir_stream(&self) -> Result<Box<Dir>> {
        // We need to duplicate the fd, because `opendir(3)`:
        //     After a successful call to fdopendir(), fd is used internally by the implementation,
        //     and should not otherwise be used by the application.
        // `opendir(3p)` also says that it's undefined behavior to
        // modify the state of the fd in a different way than by accessing DIR*.
        //
        // Still, rewinddir will be needed because the two file descriptors
        // share progress. But we can safely execute closedir now.
        let file = self.try_clone()?;
        // TODO This doesn't look very clean. Can we do something about it?
        // Boxing is needed here in order to satisfy `yanix`'s trait requirement for the `DirIter`
        // where `T: Deref<Target = Dir>`.
        Ok(Box::new(Dir::from(file)?))
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
        self.0.get()
    }
}

impl FromRawFd for OsFile {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(Cell::new(fd))
    }
}

impl IntoRawFd for OsFile {
    fn into_raw_fd(self) -> RawFd {
        // We need to prevent dropping of the OsFile
        let wrapped = ManuallyDrop::new(self);
        wrapped.0.get()
    }
}

impl AsFile for OsFile {
    fn as_file(&self) -> ManuallyDrop<File> {
        let file = unsafe { File::from_raw_fd(self.0.get()) };
        ManuallyDrop::new(file)
    }
}

use crate::wasi::Result;
use std::fs;
use std::ops::Deref;
use std::os::unix::prelude::{AsRawFd, RawFd};
use yanix::dir::Dir;

#[derive(Debug)]
pub(crate) struct OsHandle(fs::File);

impl OsHandle {
    pub(crate) fn dir_stream(&self) -> Result<Box<Dir>> {
        // We need to duplicate the fd, because `opendir(3)`:
        //     After a successful call to fdopendir(), fd is used internally by the implementation,
        //     and should not otherwise be used by the application.
        // `opendir(3p)` also says that it's undefined behavior to
        // modify the state of the fd in a different way than by accessing DIR*.
        //
        // Still, rewinddir will be needed because the two file descriptors
        // share progress. But we can safely execute closedir now.
        let fd = self.0.try_clone()?;
        // TODO This doesn't look very clean. Can we do something about it?
        // Boxing is needed here in order to satisfy `yanix`'s trait requirement for the `DirIter`
        // where `T: Deref<Target = Dir>`.
        Ok(Box::new(Dir::from(fd)?))
    }
}

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

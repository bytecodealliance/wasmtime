use crate::sys::sys_impl::oshandle::OsHandle;
use crate::wasi::Result;
use std::io;
use std::ops::Deref;
use yanix::dir::Dir;

#[derive(Debug)]
pub(crate) struct OsDirHandle(OsHandle);

impl OsDirHandle {
    /// Consumes the spcified `OsHandle`.
    pub(crate) fn new(handle: OsHandle) -> io::Result<Self> {
        Ok(Self(handle))
    }
    /// Tries clone `self`.
    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        let handle = self.0.try_clone()?;
        Self::new(handle)
    }
    /// Gets current dir stream pointer.
    pub(crate) fn dir_stream(&self) -> Result<Box<Dir>> {
        // We need to duplicate the fd, because `opendir(3)`:
        //     After a successful call to fdopendir(), fd is used internally by the implementation,
        //     and should not otherwise be used by the application.
        // `opendir(3p)` also says that it's undefined behavior to
        // modify the state of the fd in a different way than by accessing DIR*.
        //
        // Still, rewinddir will be needed because the two file descriptors
        // share progress. But we can safely execute closedir now.
        let file = self.0.try_clone()?;
        // TODO This doesn't look very clean. Can we do something about it?
        // Boxing is needed here in order to satisfy `yanix`'s trait requirement for the `DirIter`
        // where `T: Deref<Target = Dir>`.
        Ok(Box::new(Dir::from(file)?))
    }
}

impl Deref for OsDirHandle {
    type Target = OsHandle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

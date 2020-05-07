use crate::handle::HandleRights;
use crate::sys::sys_impl::oshandle::RawOsHandle;
use crate::wasi::Result;
use std::cell::Cell;
use std::io;
use yanix::dir::Dir;

#[derive(Debug)]
pub(crate) struct OsDir {
    pub(crate) rights: Cell<HandleRights>,
    pub(crate) handle: RawOsHandle,
}

impl OsDir {
    pub(crate) fn new(rights: HandleRights, handle: RawOsHandle) -> io::Result<Self> {
        let rights = Cell::new(rights);
        Ok(Self { rights, handle })
    }
    /// Returns the `Dir` stream pointer associated with this `OsDir`.
    pub(crate) fn stream_ptr(&self) -> Result<Box<Dir>> {
        // We need to duplicate the handle, because `opendir(3)`:
        //     After a successful call to fdopendir(), fd is used internally by the implementation,
        //     and should not otherwise be used by the application.
        // `opendir(3p)` also says that it's undefined behavior to
        // modify the state of the fd in a different way than by accessing DIR*.
        //
        // Still, rewinddir will be needed because the two file descriptors
        // share progress. But we can safely execute closedir now.
        let file = self.handle.try_clone()?;
        // TODO This doesn't look very clean. Can we do something about it?
        // Boxing is needed here in order to satisfy `yanix`'s trait requirement for the `DirIter`
        // where `T: Deref<Target = Dir>`.
        Ok(Box::new(Dir::from(file)?))
    }
}

use crate::handle::HandleRights;
use crate::sys::sys_impl::oshandle::RawOsHandle;
use crate::Result;
use std::cell::Cell;
use std::io;
use yanix::dir::Dir;

#[derive(Debug)]
/// A directory in the operating system's file system. Its impl of `Handle` is
/// in `sys::osdir`. This type is exposed to all other modules as
/// `sys::osdir::OsDir` when configured.
///
/// # Constructing `OsDir`
///
/// `OsDir` can currently only be constructed from `std::fs::File` using
/// the `std::convert::TryFrom` trait:
///
/// ```rust,no_run
/// use std::fs::OpenOptions;
/// use std::convert::TryFrom;
/// use wasi_common::OsDir;
///
/// let dir = OpenOptions::new().read(true).open("some_dir").unwrap();
/// let os_dir = OsDir::try_from(dir).unwrap();
/// ```
pub struct OsDir {
    pub(crate) rights: Cell<HandleRights>,
    pub(crate) handle: RawOsHandle,
}

impl OsDir {
    pub(crate) fn new(rights: HandleRights, handle: RawOsHandle) -> io::Result<Self> {
        let rights = Cell::new(rights);
        Ok(Self { rights, handle })
    }
    /// Returns the `Dir` stream pointer associated with this `OsDir`. Duck typing:
    /// sys::unix::fd::readdir expects the configured OsDir to have this method.
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

use crate::handle::HandleRights;
use crate::sys::sys_impl::oshandle::RawOsHandle;
use crate::wasi::Result;
use std::cell::{Cell, RefCell, RefMut};
use std::io;
use yanix::dir::Dir;

#[derive(Debug)]
pub(crate) struct OsDir {
    pub(crate) rights: Cell<HandleRights>,
    pub(crate) handle: RawOsHandle,
    // When the client makes a `fd_readdir` syscall on this descriptor,
    // we will need to cache the `libc::DIR` pointer manually in order
    // to be able to seek on it later. While on Linux, this is handled
    // by the OS, BSD Unixes require the client to do this caching.
    //
    // This comes directly from the BSD man pages on `readdir`:
    //   > Values returned by telldir() are good only for the lifetime
    //   > of the DIR pointer, dirp, from which they are derived.
    //   > If the directory is closed and then reopened, prior values
    //   > returned by telldir() will no longer be valid.
    stream_ptr: RefCell<Dir>,
}

impl OsDir {
    pub(crate) fn new(rights: HandleRights, handle: RawOsHandle) -> io::Result<Self> {
        let rights = Cell::new(rights);
        // We need to duplicate the handle, because `opendir(3)`:
        //     Upon successful return from fdopendir(), the file descriptor is under
        //     control of the system, and if any attempt is made to close the file
        //     descriptor, or to modify the state of the associated description other
        //     than by means of closedir(), readdir(), readdir_r(), or rewinddir(),
        //     the behaviour is undefined.
        let stream_ptr = Dir::from(handle.try_clone()?)?;
        let stream_ptr = RefCell::new(stream_ptr);
        Ok(Self {
            rights,
            handle,
            stream_ptr,
        })
    }
    /// Returns the `Dir` stream pointer associated with this `OsDir`.
    pub(crate) fn stream_ptr(&self) -> Result<RefMut<Dir>> {
        Ok(self.stream_ptr.borrow_mut())
    }
}

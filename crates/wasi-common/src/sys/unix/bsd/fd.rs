use crate::sys::entry::OsHandle;
use crate::wasi::Result;
use std::sync::{Mutex, MutexGuard};
use yanix::dir::Dir;

pub(crate) fn get_dir_from_os_handle<'a>(
    os_handle: &'a mut OsHandle,
) -> Result<MutexGuard<'a, Dir>> {
    let dir = match os_handle.dir {
        Some(ref mut dir) => dir,
        None => {
            // We need to duplicate the fd, because `opendir(3)`:
            //     Upon successful return from fdopendir(), the file descriptor is under
            //     control of the system, and if any attempt is made to close the file
            //     descriptor, or to modify the state of the associated description other
            //     than by means of closedir(), readdir(), readdir_r(), or rewinddir(),
            //     the behaviour is undefined.
            let fd = (*os_handle).try_clone()?;
            let dir = Dir::from(fd)?;
            os_handle.dir.get_or_insert(Mutex::new(dir))
        }
    };
    // Note that from this point on, until the end of the parent scope (i.e., enclosing this
    // function), we're locking the `Dir` member of this `OsHandle`.
    Ok(dir.lock().unwrap())
}

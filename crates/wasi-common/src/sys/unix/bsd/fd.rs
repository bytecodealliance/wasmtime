use crate::sys::entry::OsHandle;
use crate::wasi::Result;
use std::cell::RefMut;
use yanix::dir::Dir;

pub(crate) fn get_dir_from_os_handle(os_handle: &OsHandle) -> Result<RefMut<Dir>> {
    if os_handle.dir.borrow().is_none() {
        // We need to duplicate the fd, because `opendir(3)`:
        //     Upon successful return from fdopendir(), the file descriptor is under
        //     control of the system, and if any attempt is made to close the file
        //     descriptor, or to modify the state of the associated description other
        //     than by means of closedir(), readdir(), readdir_r(), or rewinddir(),
        //     the behaviour is undefined.
        let fd = (*os_handle).try_clone()?;
        let d = Dir::from(fd)?;
        *os_handle.dir.borrow_mut() = Some(d);
    }
    Ok(RefMut::map(os_handle.dir.borrow_mut(), |dir| {
        dir.as_mut().unwrap()
    }))
}

use crate::sys::entry::OsHandle;
use crate::wasi::Result;
use yanix::dir::Dir;

pub(crate) fn get_dir_from_os_handle(os_handle: &mut OsHandle) -> Result<Box<Dir>> {
    // We need to duplicate the fd, because `opendir(3)`:
    //     After a successful call to fdopendir(), fd is used internally by the implementation,
    //     and should not otherwise be used by the application.
    // `opendir(3p)` also says that it's undefined behavior to
    // modify the state of the fd in a different way than by accessing DIR*.
    //
    // Still, rewinddir will be needed because the two file descriptors
    // share progress. But we can safely execute closedir now.
    let fd = os_handle.try_clone()?;
    // TODO This doesn't look very clean. Can we do something about it?
    // Boxing is needed here in order to satisfy `yanix`'s trait requirement for the `DirIter`
    // where `T: Deref<Target = Dir>`.
    Ok(Box::new(Dir::from(fd)?))
}

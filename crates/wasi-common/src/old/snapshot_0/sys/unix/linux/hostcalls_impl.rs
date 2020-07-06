use crate::old::snapshot_0::hostcalls_impl::PathGet;
use crate::old::snapshot_0::wasi::WasiResult;
use std::os::unix::prelude::AsRawFd;

pub(crate) fn path_unlink_file(resolved: PathGet) -> WasiResult<()> {
    use yanix::file::{unlinkat, AtFlags};
    unsafe {
        unlinkat(
            resolved.dirfd().as_raw_fd(),
            resolved.path(),
            AtFlags::empty(),
        )
    }
    .map_err(Into::into)
}

pub(crate) fn path_symlink(old_path: &str, resolved: PathGet) -> WasiResult<()> {
    use yanix::file::symlinkat;

    log::debug!("path_symlink old_path = {:?}", old_path);
    log::debug!("path_symlink resolved = {:?}", resolved);

    unsafe { symlinkat(old_path, resolved.dirfd().as_raw_fd(), resolved.path()) }
        .map_err(Into::into)
}

pub(crate) fn path_rename(resolved_old: PathGet, resolved_new: PathGet) -> WasiResult<()> {
    use yanix::file::renameat;
    unsafe {
        renameat(
            resolved_old.dirfd().as_raw_fd(),
            resolved_old.path(),
            resolved_new.dirfd().as_raw_fd(),
            resolved_new.path(),
        )
    }
    .map_err(Into::into)
}

pub(crate) mod fd_readdir_impl {
    use crate::old::snapshot_0::sys::entry_impl::OsHandle;
    use crate::old::snapshot_0::wasi::WasiResult;
    use yanix::dir::Dir;

    pub(crate) fn get_dir_from_os_handle(os_handle: &mut OsHandle) -> WasiResult<Box<Dir>> {
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
}

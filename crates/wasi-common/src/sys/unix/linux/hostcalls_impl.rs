use crate::hostcalls_impl::PathGet;
use crate::{wasi, Error, Result};
use std::convert::TryInto;
use std::fs::File;
use std::os::unix::prelude::AsRawFd;

pub(crate) fn path_unlink_file(resolved: PathGet) -> Result<()> {
    use yanix::file::{unlinkat, AtFlag};
    unlinkat(
        resolved.dirfd().as_raw_fd(),
        resolved.path(),
        AtFlag::empty(),
    )
    .map_err(Into::into)
}

pub(crate) fn path_symlink(old_path: &str, resolved: PathGet) -> Result<()> {
    use yanix::file::symlinkat;

    log::debug!("path_symlink old_path = {:?}", old_path);
    log::debug!("path_symlink resolved = {:?}", resolved);

    symlinkat(old_path, resolved.dirfd().as_raw_fd(), resolved.path()).map_err(Into::into)
}

pub(crate) fn path_rename(resolved_old: PathGet, resolved_new: PathGet) -> Result<()> {
    use yanix::file::renameat;
    renameat(
        resolved_old.dirfd().as_raw_fd(),
        resolved_old.path(),
        resolved_new.dirfd().as_raw_fd(),
        resolved_new.path(),
    )
    .map_err(Into::into)
}

pub(crate) fn fd_advise(
    file: &File,
    advice: wasi::__wasi_advice_t,
    offset: wasi::__wasi_filesize_t,
    len: wasi::__wasi_filesize_t,
) -> Result<()> {
    use yanix::sys::{posix_fadvise, PosixFadviseAdvice};
    let offset = offset.try_into()?;
    let len = len.try_into()?;
    let host_advice = match advice {
        wasi::__WASI_ADVICE_DONTNEED => PosixFadviseAdvice::DontNeed,
        wasi::__WASI_ADVICE_SEQUENTIAL => PosixFadviseAdvice::Sequential,
        wasi::__WASI_ADVICE_WILLNEED => PosixFadviseAdvice::WillNeed,
        wasi::__WASI_ADVICE_NOREUSE => PosixFadviseAdvice::NoReuse,
        wasi::__WASI_ADVICE_RANDOM => PosixFadviseAdvice::Random,
        wasi::__WASI_ADVICE_NORMAL => PosixFadviseAdvice::Normal,
        _ => return Err(Error::EINVAL),
    };
    posix_fadvise(file.as_raw_fd(), offset, len, host_advice).map_err(Into::into)
}

pub(crate) mod fd_readdir_impl {
    use crate::sys::fdentry_impl::OsHandle;
    use crate::Result;
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
}

use crate::host::Dirent;
use crate::hostcalls_impl::PathGet;
use crate::{wasi, Error, Result};
use log::trace;
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

pub(crate) fn fd_readdir(
    fd: &File,
    cookie: wasi::__wasi_dircookie_t,
) -> Result<impl Iterator<Item = Result<Dirent>>> {
    use yanix::dir::{Dir, DirIter, Entry, SeekLoc};

    // We need to duplicate the fd, because `opendir(3)`:
    //     After a successful call to fdopendir(), fd is used internally by the implementation,
    //     and should not otherwise be used by the application.
    // `opendir(3p)` also says that it's undefined behavior to
    // modify the state of the fd in a different way than by accessing DIR*.
    //
    // Still, rewinddir will be needed because the two file descriptors
    // share progress. But we can safely execute closedir now.
    let fd = fd.try_clone()?;
    let mut dir = Box::new(Dir::from(fd)?);

    // Seek if needed. Unless cookie is wasi::__WASI_DIRCOOKIE_START,
    // new items may not be returned to the caller.
    //
    // According to `opendir(3p)`:
    //     If a file is removed from or added to the directory after the most recent call
    //     to opendir() or rewinddir(), whether a subsequent call to readdir() returns an entry
    //     for that file is unspecified.
    if cookie == wasi::__WASI_DIRCOOKIE_START {
        trace!("     | fd_readdir: doing rewinddir");
        dir.rewind();
    } else {
        trace!("     | fd_readdir: doing seekdir to {}", cookie);
        let loc = unsafe { SeekLoc::from_raw(cookie as i64) };
        dir.seek(loc);
    }

    Ok(DirIter::new(dir).map(|entry| {
        let entry: Entry = entry?;
        Ok(Dirent {
            name: entry // TODO can we reuse path_from_host for CStr?
                .file_name()
                .to_str()?
                .to_owned(),
            ino: entry.ino(),
            ftype: entry.file_type().into(),
            cookie: entry.seek_loc().to_raw().try_into()?,
        })
    }))
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

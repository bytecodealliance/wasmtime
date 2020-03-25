use super::sys_impl::fd::get_dir_from_os_handle;
use crate::sys::entry::OsHandle;
use crate::wasi::{self, types, Result};
use std::convert::TryInto;
use std::fs::File;
use std::os::unix::prelude::AsRawFd;

pub(crate) fn fdstat_get(fd: &File) -> Result<types::Fdflags> {
    let fdflags = unsafe { yanix::fcntl::get_status_flags(fd.as_raw_fd())? };
    Ok(fdflags.into())
}

pub(crate) fn fdstat_set_flags(fd: &File, fdflags: types::Fdflags) -> Result<Option<OsHandle>> {
    unsafe { yanix::fcntl::set_status_flags(fd.as_raw_fd(), fdflags.into())? };
    // TODO why are we returning Ok(None) here?
    Ok(None)
}

pub(crate) fn advise(
    file: &File,
    advice: types::Advice,
    offset: types::Filesize,
    len: types::Filesize,
) -> Result<()> {
    use yanix::fadvise::{posix_fadvise, PosixFadviseAdvice};
    let offset = offset.try_into()?;
    let len = len.try_into()?;
    let host_advice = match advice {
        types::Advice::Dontneed => PosixFadviseAdvice::DontNeed,
        types::Advice::Sequential => PosixFadviseAdvice::Sequential,
        types::Advice::Willneed => PosixFadviseAdvice::WillNeed,
        types::Advice::Noreuse => PosixFadviseAdvice::NoReuse,
        types::Advice::Random => PosixFadviseAdvice::Random,
        types::Advice::Normal => PosixFadviseAdvice::Normal,
    };
    unsafe { posix_fadvise(file.as_raw_fd(), offset, len, host_advice)? };
    Ok(())
}

pub(crate) fn filestat_get(file: &std::fs::File) -> Result<types::Filestat> {
    use yanix::file::fstat;
    let stat = unsafe { fstat(file.as_raw_fd())? };
    Ok(stat.try_into()?)
}

pub(crate) fn readdir<'a>(
    os_handle: &'a OsHandle,
    cookie: types::Dircookie,
) -> Result<impl Iterator<Item = Result<(types::Dirent, String)>> + 'a> {
    use yanix::dir::{DirIter, Entry, EntryExt, SeekLoc};

    // Get an instance of `Dir`; this is host-specific due to intricasies
    // of managing a dir stream between Linux and BSD *nixes
    let mut dir = get_dir_from_os_handle(os_handle)?;

    // Seek if needed. Unless cookie is wasi::__WASI_DIRCOOKIE_START,
    // new items may not be returned to the caller.
    if cookie == wasi::DIRCOOKIE_START {
        log::trace!("     | fd_readdir: doing rewinddir");
        dir.rewind();
    } else {
        log::trace!("     | fd_readdir: doing seekdir to {}", cookie);
        let loc = unsafe { SeekLoc::from_raw(cookie as i64)? };
        dir.seek(loc);
    }

    Ok(DirIter::new(dir).map(|entry| {
        let entry: Entry = entry?;
        let name = entry.file_name().to_str()?.to_owned();
        let dirent = types::Dirent {
            d_next: entry.seek_loc()?.to_raw().try_into()?,
            d_ino: entry.ino(),
            d_namlen: name.len().try_into()?,
            d_type: entry.file_type().into(),
        };
        Ok((dirent, name))
    }))
}

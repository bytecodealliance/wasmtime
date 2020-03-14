#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use crate::entry::Descriptor;
use crate::host::Dirent;
use crate::hostcalls_impl::PathGet;
use crate::sys::entry_impl::OsHandle;
use crate::sys::{host_impl, unix::sys_impl};
use crate::wasi::{self, WasiError, WasiResult};
use std::convert::TryInto;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::os::unix::prelude::{AsRawFd, FromRawFd};

pub(crate) use sys_impl::hostcalls_impl::*;

pub(crate) fn fd_pread(
    file: &File,
    buf: &mut [u8],
    offset: wasi::__wasi_filesize_t,
) -> WasiResult<usize> {
    file.read_at(buf, offset).map_err(Into::into)
}

pub(crate) fn fd_pwrite(
    file: &File,
    buf: &[u8],
    offset: wasi::__wasi_filesize_t,
) -> WasiResult<usize> {
    file.write_at(buf, offset).map_err(Into::into)
}

pub(crate) fn fd_fdstat_get(fd: &File) -> WasiResult<wasi::__wasi_fdflags_t> {
    unsafe { yanix::fcntl::get_status_flags(fd.as_raw_fd()) }
        .map(host_impl::fdflags_from_nix)
        .map_err(Into::into)
}

pub(crate) fn fd_fdstat_set_flags(
    fd: &File,
    fdflags: wasi::__wasi_fdflags_t,
) -> WasiResult<Option<OsHandle>> {
    let nix_flags = host_impl::nix_from_fdflags(fdflags);
    unsafe { yanix::fcntl::set_status_flags(fd.as_raw_fd(), nix_flags) }
        .map(|_| None)
        .map_err(Into::into)
}

pub(crate) fn fd_advise(
    file: &File,
    advice: wasi::__wasi_advice_t,
    offset: wasi::__wasi_filesize_t,
    len: wasi::__wasi_filesize_t,
) -> WasiResult<()> {
    use yanix::fadvise::{posix_fadvise, PosixFadviseAdvice};
    let offset = offset.try_into()?;
    let len = len.try_into()?;
    let host_advice = match advice {
        wasi::__WASI_ADVICE_DONTNEED => PosixFadviseAdvice::DontNeed,
        wasi::__WASI_ADVICE_SEQUENTIAL => PosixFadviseAdvice::Sequential,
        wasi::__WASI_ADVICE_WILLNEED => PosixFadviseAdvice::WillNeed,
        wasi::__WASI_ADVICE_NOREUSE => PosixFadviseAdvice::NoReuse,
        wasi::__WASI_ADVICE_RANDOM => PosixFadviseAdvice::Random,
        wasi::__WASI_ADVICE_NORMAL => PosixFadviseAdvice::Normal,
        _ => return Err(WasiError::EINVAL),
    };
    unsafe { posix_fadvise(file.as_raw_fd(), offset, len, host_advice) }.map_err(Into::into)
}

pub(crate) fn path_create_directory(base: &File, path: &str) -> WasiResult<()> {
    use yanix::file::{mkdirat, Mode};
    unsafe { mkdirat(base.as_raw_fd(), path, Mode::from_bits_truncate(0o777)) }.map_err(Into::into)
}

pub(crate) fn path_link(resolved_old: PathGet, resolved_new: PathGet) -> WasiResult<()> {
    use yanix::file::{linkat, AtFlag};
    unsafe {
        linkat(
            resolved_old.dirfd().as_raw_fd(),
            resolved_old.path(),
            resolved_new.dirfd().as_raw_fd(),
            resolved_new.path(),
            AtFlag::SYMLINK_FOLLOW,
        )
    }
    .map_err(Into::into)
}

pub(crate) fn path_open(
    resolved: PathGet,
    read: bool,
    write: bool,
    oflags: wasi::__wasi_oflags_t,
    fs_flags: wasi::__wasi_fdflags_t,
) -> WasiResult<Descriptor> {
    use yanix::file::{fstatat, openat, AtFlag, FileType, Mode, OFlag};

    let mut nix_all_oflags = if read && write {
        OFlag::RDWR
    } else if write {
        OFlag::WRONLY
    } else {
        OFlag::RDONLY
    };

    // on non-Capsicum systems, we always want nofollow
    nix_all_oflags.insert(OFlag::NOFOLLOW);

    // convert open flags
    nix_all_oflags.insert(host_impl::nix_from_oflags(oflags));

    // convert file descriptor flags
    nix_all_oflags.insert(host_impl::nix_from_fdflags(fs_flags));

    // Call openat. Use mode 0o666 so that we follow whatever the user's
    // umask is, but don't set the executable flag, because it isn't yet
    // meaningful for WASI programs to create executable files.

    log::debug!("path_open resolved = {:?}", resolved);
    log::debug!("path_open oflags = {:?}", nix_all_oflags);

    let fd_no = unsafe {
        openat(
            resolved.dirfd().as_raw_fd(),
            resolved.path(),
            nix_all_oflags,
            Mode::from_bits_truncate(0o666),
        )
    };
    let new_fd = match fd_no {
        Ok(fd) => fd,
        Err(e) => {
            match e.raw_os_error().unwrap() {
                // Linux returns ENXIO instead of EOPNOTSUPP when opening a socket
                libc::ENXIO => {
                    match unsafe {
                        fstatat(
                            resolved.dirfd().as_raw_fd(),
                            resolved.path(),
                            AtFlag::SYMLINK_NOFOLLOW,
                        )
                    } {
                        Ok(stat) => {
                            if FileType::from_stat_st_mode(stat.st_mode) == FileType::Socket {
                                return Err(WasiError::ENOTSUP);
                            }
                        }
                        Err(err) => {
                            log::debug!("path_open fstatat error: {:?}", err);
                        }
                    }
                }
                // Linux returns ENOTDIR instead of ELOOP when using O_NOFOLLOW|O_DIRECTORY
                // on a symlink.
                libc::ENOTDIR
                    if !(nix_all_oflags & (OFlag::NOFOLLOW | OFlag::DIRECTORY)).is_empty() =>
                {
                    match unsafe {
                        fstatat(
                            resolved.dirfd().as_raw_fd(),
                            resolved.path(),
                            AtFlag::SYMLINK_NOFOLLOW,
                        )
                    } {
                        Ok(stat) => {
                            if FileType::from_stat_st_mode(stat.st_mode) == FileType::Symlink {
                                return Err(WasiError::ELOOP);
                            }
                        }
                        Err(err) => {
                            log::debug!("path_open fstatat error: {:?}", err);
                        }
                    }
                }
                // FreeBSD returns EMLINK instead of ELOOP when using O_NOFOLLOW on
                // a symlink.
                libc::EMLINK if !(nix_all_oflags & OFlag::NOFOLLOW).is_empty() => {
                    return Err(WasiError::ELOOP);
                }
                _ => {}
            }

            return Err(e.into());
        }
    };

    log::debug!("path_open (host) new_fd = {:?}", new_fd);

    // Determine the type of the new file descriptor and which rights contradict with this type
    Ok(OsHandle::from(unsafe { File::from_raw_fd(new_fd) }).into())
}

pub(crate) fn path_readlink(resolved: PathGet, buf: &mut [u8]) -> WasiResult<usize> {
    use std::cmp::min;
    use yanix::file::readlinkat;
    let read_link = unsafe { readlinkat(resolved.dirfd().as_raw_fd(), resolved.path()) }
        .map_err(Into::into)
        .and_then(host_impl::path_from_host)?;
    let copy_len = min(read_link.len(), buf.len());
    if copy_len > 0 {
        buf[..copy_len].copy_from_slice(&read_link.as_bytes()[..copy_len]);
    }
    Ok(copy_len)
}

pub(crate) fn fd_filestat_get(file: &std::fs::File) -> WasiResult<wasi::__wasi_filestat_t> {
    use yanix::file::fstat;
    unsafe { fstat(file.as_raw_fd()) }
        .map_err(Into::into)
        .and_then(host_impl::filestat_from_nix)
}

pub(crate) fn path_filestat_get(
    resolved: PathGet,
    dirflags: wasi::__wasi_lookupflags_t,
) -> WasiResult<wasi::__wasi_filestat_t> {
    use yanix::file::{fstatat, AtFlag};
    let atflags = match dirflags {
        0 => AtFlag::empty(),
        _ => AtFlag::SYMLINK_NOFOLLOW,
    };
    unsafe { fstatat(resolved.dirfd().as_raw_fd(), resolved.path(), atflags) }
        .map_err(Into::into)
        .and_then(host_impl::filestat_from_nix)
}

pub(crate) fn path_filestat_set_times(
    resolved: PathGet,
    dirflags: wasi::__wasi_lookupflags_t,
    st_atim: wasi::__wasi_timestamp_t,
    st_mtim: wasi::__wasi_timestamp_t,
    fst_flags: wasi::__wasi_fstflags_t,
) -> WasiResult<()> {
    use std::time::{Duration, UNIX_EPOCH};
    use yanix::filetime::*;

    let set_atim = fst_flags & wasi::__WASI_FSTFLAGS_ATIM != 0;
    let set_atim_now = fst_flags & wasi::__WASI_FSTFLAGS_ATIM_NOW != 0;
    let set_mtim = fst_flags & wasi::__WASI_FSTFLAGS_MTIM != 0;
    let set_mtim_now = fst_flags & wasi::__WASI_FSTFLAGS_MTIM_NOW != 0;

    if (set_atim && set_atim_now) || (set_mtim && set_mtim_now) {
        return Err(WasiError::EINVAL);
    }

    let symlink_nofollow = wasi::__WASI_LOOKUPFLAGS_SYMLINK_FOLLOW != dirflags;
    let atim = if set_atim {
        let time = UNIX_EPOCH + Duration::from_nanos(st_atim);
        FileTime::FileTime(filetime::FileTime::from_system_time(time))
    } else if set_atim_now {
        FileTime::Now
    } else {
        FileTime::Omit
    };
    let mtim = if set_mtim {
        let time = UNIX_EPOCH + Duration::from_nanos(st_mtim);
        FileTime::FileTime(filetime::FileTime::from_system_time(time))
    } else if set_mtim_now {
        FileTime::Now
    } else {
        FileTime::Omit
    };

    utimensat(
        &resolved.dirfd().as_os_handle(),
        resolved.path(),
        atim,
        mtim,
        symlink_nofollow,
    )
    .map_err(Into::into)
}

pub(crate) fn path_remove_directory(resolved: PathGet) -> WasiResult<()> {
    use yanix::file::{unlinkat, AtFlag};

    unsafe {
        unlinkat(
            resolved.dirfd().as_raw_fd(),
            resolved.path(),
            AtFlag::REMOVEDIR,
        )
    }
    .map_err(Into::into)
}

pub(crate) fn fd_readdir<'a>(
    os_handle: &'a mut OsHandle,
    cookie: wasi::__wasi_dircookie_t,
) -> WasiResult<impl Iterator<Item = WasiResult<Dirent>> + 'a> {
    use yanix::dir::{DirIter, Entry, EntryExt, SeekLoc};

    // Get an instance of `Dir`; this is host-specific due to intricasies
    // of managing a dir stream between Linux and BSD *nixes
    let mut dir = fd_readdir_impl::get_dir_from_os_handle(os_handle)?;

    // Seek if needed. Unless cookie is wasi::__WASI_DIRCOOKIE_START,
    // new items may not be returned to the caller.
    if cookie == wasi::__WASI_DIRCOOKIE_START {
        log::trace!("     | fd_readdir: doing rewinddir");
        dir.rewind();
    } else {
        log::trace!("     | fd_readdir: doing seekdir to {}", cookie);
        let loc = unsafe { SeekLoc::from_raw(cookie as i64)? };
        dir.seek(loc);
    }

    Ok(DirIter::new(dir).map(|entry| {
        let entry: Entry = entry?;
        Ok(Dirent {
            name: entry
                // TODO can we reuse path_from_host for CStr?
                .file_name()
                .to_str()?
                .to_owned(),
            ino: entry.ino(),
            ftype: entry.file_type().into(),
            cookie: entry.seek_loc()?.to_raw().try_into()?,
        })
    }))
}

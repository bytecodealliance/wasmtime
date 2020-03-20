use crate::entry::Descriptor;
use crate::path::PathGet;
use crate::sys::entry::OsHandle;
use crate::sys::unix::sys_impl;
use crate::wasi::{types, Errno, Result};
use std::convert::TryInto;
use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::prelude::{AsRawFd, FromRawFd, OsStrExt};
use std::str;
use yanix::file::OFlag;

pub(crate) use sys_impl::path::*;

/// Creates owned WASI path from OS string.
///
/// NB WASI spec requires OS string to be valid UTF-8. Otherwise,
/// `__WASI_ERRNO_ILSEQ` error is returned.
pub(crate) fn from_host<S: AsRef<OsStr>>(s: S) -> Result<String> {
    let s = str::from_utf8(s.as_ref().as_bytes())?;
    Ok(s.to_owned())
}

pub(crate) fn open_rights(
    rights_base: types::Rights,
    rights_inheriting: types::Rights,
    oflags: types::Oflags,
    fs_flags: types::Fdflags,
) -> (types::Rights, types::Rights) {
    // which rights are needed on the dirfd?
    let mut needed_base = types::Rights::PATH_OPEN;
    let mut needed_inheriting = rights_base | rights_inheriting;

    // convert open flags
    let oflags: OFlag = oflags.into();
    if oflags.contains(OFlag::CREAT) {
        needed_base |= types::Rights::PATH_CREATE_FILE;
    }
    if oflags.contains(OFlag::TRUNC) {
        needed_base |= types::Rights::PATH_FILESTAT_SET_SIZE;
    }

    // convert file descriptor flags
    let fdflags: OFlag = fs_flags.into();
    if fdflags.contains(OFlag::DSYNC) {
        needed_inheriting |= types::Rights::FD_DATASYNC;
    }
    if fdflags.intersects(sys_impl::O_RSYNC | OFlag::SYNC) {
        needed_inheriting |= types::Rights::FD_SYNC;
    }

    (needed_base, needed_inheriting)
}

pub(crate) fn openat(dirfd: &File, path: &str) -> Result<File> {
    use std::os::unix::prelude::{AsRawFd, FromRawFd};
    use yanix::file::{openat, Mode};

    log::debug!("path_get openat path = {:?}", path);

    let raw_fd = unsafe {
        openat(
            dirfd.as_raw_fd(),
            path,
            OFlag::RDONLY | OFlag::DIRECTORY | OFlag::NOFOLLOW,
            Mode::empty(),
        )?
    };
    let file = unsafe { File::from_raw_fd(raw_fd) };
    Ok(file)
}

pub(crate) fn readlinkat(dirfd: &File, path: &str) -> Result<String> {
    use std::os::unix::prelude::AsRawFd;
    use yanix::file::readlinkat;

    log::debug!("path_get readlinkat path = {:?}", path);

    let path = unsafe { readlinkat(dirfd.as_raw_fd(), path)? };
    let path = from_host(path)?;
    Ok(path)
}

pub(crate) fn create_directory(base: &File, path: &str) -> Result<()> {
    use yanix::file::{mkdirat, Mode};
    unsafe { mkdirat(base.as_raw_fd(), path, Mode::from_bits_truncate(0o777))? };
    Ok(())
}

pub(crate) fn link(
    resolved_old: PathGet,
    resolved_new: PathGet,
    follow_symlinks: bool,
) -> Result<()> {
    use yanix::file::{linkat, AtFlag};
    let flags = if follow_symlinks {
        AtFlag::SYMLINK_FOLLOW
    } else {
        AtFlag::empty()
    };
    unsafe {
        linkat(
            resolved_old.dirfd().as_raw_fd(),
            resolved_old.path(),
            resolved_new.dirfd().as_raw_fd(),
            resolved_new.path(),
            flags,
        )?
    };
    Ok(())
}

pub(crate) fn open(
    resolved: PathGet,
    read: bool,
    write: bool,
    oflags: types::Oflags,
    fs_flags: types::Fdflags,
) -> Result<Descriptor> {
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
    nix_all_oflags.insert(oflags.into());

    // convert file descriptor flags
    nix_all_oflags.insert(fs_flags.into());

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
                                return Err(Errno::Notsup);
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
                                return Err(Errno::Loop);
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
                    return Err(Errno::Loop);
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

pub(crate) fn readlink(resolved: PathGet, buf: &mut [u8]) -> Result<usize> {
    use std::cmp::min;
    use yanix::file::readlinkat;
    let read_link = unsafe { readlinkat(resolved.dirfd().as_raw_fd(), resolved.path())? };
    let read_link = from_host(read_link)?;
    let copy_len = min(read_link.len(), buf.len());
    if copy_len > 0 {
        buf[..copy_len].copy_from_slice(&read_link.as_bytes()[..copy_len]);
    }
    Ok(copy_len)
}

pub(crate) fn filestat_get(
    resolved: PathGet,
    dirflags: types::Lookupflags,
) -> Result<types::Filestat> {
    use yanix::file::fstatat;
    let atflags = dirflags.into();
    let filestat = unsafe { fstatat(resolved.dirfd().as_raw_fd(), resolved.path(), atflags)? };
    let filestat = filestat.try_into()?;
    Ok(filestat)
}

pub(crate) fn filestat_set_times(
    resolved: PathGet,
    dirflags: types::Lookupflags,
    st_atim: types::Timestamp,
    st_mtim: types::Timestamp,
    fst_flags: types::Fstflags,
) -> Result<()> {
    use std::time::{Duration, UNIX_EPOCH};
    use yanix::filetime::*;

    let set_atim = fst_flags.contains(&types::Fstflags::ATIM);
    let set_atim_now = fst_flags.contains(&types::Fstflags::ATIM_NOW);
    let set_mtim = fst_flags.contains(&types::Fstflags::MTIM);
    let set_mtim_now = fst_flags.contains(&types::Fstflags::MTIM_NOW);

    if (set_atim && set_atim_now) || (set_mtim && set_mtim_now) {
        return Err(Errno::Inval);
    }

    let symlink_nofollow = types::Lookupflags::SYMLINK_FOLLOW != dirflags;
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
    )?;
    Ok(())
}

pub(crate) fn remove_directory(resolved: PathGet) -> Result<()> {
    use yanix::file::{unlinkat, AtFlag};

    unsafe {
        unlinkat(
            resolved.dirfd().as_raw_fd(),
            resolved.path(),
            AtFlag::REMOVEDIR,
        )?
    };
    Ok(())
}

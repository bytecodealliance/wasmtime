use super::oshandle::OsFile;
use crate::entry::EntryRights;
use crate::sys::oshandle::OsHandle;
use crate::wasi::{types, Errno, Result};
use std::ffi::OsStr;
use std::os::unix::prelude::{AsRawFd, FromRawFd, OsStrExt};
use std::str;
use yanix::file::OFlag;

pub(crate) use super::sys_impl::path::*;

/// Creates owned WASI path from OS string.
///
/// NB WASI spec requires OS string to be valid UTF-8. Otherwise,
/// `__WASI_ERRNO_ILSEQ` error is returned.
pub(crate) fn from_host<S: AsRef<OsStr>>(s: S) -> Result<String> {
    let s = str::from_utf8(s.as_ref().as_bytes())?;
    Ok(s.to_owned())
}

pub(crate) fn open_rights(
    input_rights: &EntryRights,
    oflags: types::Oflags,
    fs_flags: types::Fdflags,
) -> EntryRights {
    // which rights are needed on the dirfd?
    let mut needed_base = types::Rights::PATH_OPEN;
    let mut needed_inheriting = input_rights.base | input_rights.inheriting;

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
    if fdflags.intersects(super::O_RSYNC | OFlag::SYNC) {
        needed_inheriting |= types::Rights::FD_SYNC;
    }

    EntryRights::new(needed_base, needed_inheriting)
}

pub(crate) fn readlinkat(dirfd: &OsFile, path: &str) -> Result<String> {
    use std::os::unix::prelude::AsRawFd;
    use yanix::file::readlinkat;

    log::debug!("path_get readlinkat path = {:?}", path);

    let path = unsafe { readlinkat(dirfd.as_raw_fd(), path)? };
    let path = from_host(path)?;
    Ok(path)
}

pub(crate) fn create_directory(base: &OsFile, path: &str) -> Result<()> {
    use yanix::file::{mkdirat, Mode};
    unsafe { mkdirat(base.as_raw_fd(), path, Mode::from_bits_truncate(0o777))? };
    Ok(())
}

pub(crate) fn link(
    old_dirfd: &OsFile,
    old_path: &str,
    new_dirfd: &OsFile,
    new_path: &str,
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
            old_dirfd.as_raw_fd(),
            old_path,
            new_dirfd.as_raw_fd(),
            new_path,
            flags,
        )?
    };
    Ok(())
}

pub(crate) fn open(
    dirfd: &OsFile,
    path: &str,
    read: bool,
    write: bool,
    oflags: types::Oflags,
    fs_flags: types::Fdflags,
) -> Result<OsHandle> {
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

    log::debug!("path_open dirfd = {:?}", dirfd);
    log::debug!("path_open path = {:?}", path);
    log::debug!("path_open oflags = {:?}", nix_all_oflags);

    let fd_no = unsafe {
        openat(
            dirfd.as_raw_fd(),
            path,
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
                    match unsafe { fstatat(dirfd.as_raw_fd(), path, AtFlag::SYMLINK_NOFOLLOW) } {
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
                    match unsafe { fstatat(dirfd.as_raw_fd(), path, AtFlag::SYMLINK_NOFOLLOW) } {
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
    Ok(OsHandle::from(unsafe { OsFile::from_raw_fd(new_fd) }))
}

pub(crate) fn readlink(dirfd: &OsFile, path: &str, buf: &mut [u8]) -> Result<usize> {
    use std::cmp::min;
    use yanix::file::readlinkat;
    let read_link = unsafe { readlinkat(dirfd.as_raw_fd(), path)? };
    let read_link = from_host(read_link)?;
    let copy_len = min(read_link.len(), buf.len());
    if copy_len > 0 {
        buf[..copy_len].copy_from_slice(&read_link.as_bytes()[..copy_len]);
    }
    Ok(copy_len)
}

pub(crate) fn remove_directory(dirfd: &OsFile, path: &str) -> Result<()> {
    use yanix::file::{unlinkat, AtFlag};
    unsafe { unlinkat(dirfd.as_raw_fd(), path, AtFlag::REMOVEDIR)? };
    Ok(())
}

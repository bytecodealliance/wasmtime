use crate::handle::{Handle, HandleRights};
use crate::sys::osdir::OsDir;
use crate::sys::AsFile;
use crate::wasi::types;
use crate::{Error, Result};
use std::convert::{TryFrom, TryInto};
use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::prelude::{AsRawFd, FromRawFd, OsStrExt};
use std::str;
use yanix::file::OFlags;

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
    input_rights: &HandleRights,
    oflags: types::Oflags,
    fs_flags: types::Fdflags,
) -> HandleRights {
    // which rights are needed on the dirfd?
    let mut needed_base = types::Rights::PATH_OPEN;
    let mut needed_inheriting = input_rights.base | input_rights.inheriting;

    // convert open flags
    let oflags: OFlags = oflags.into();
    if oflags.contains(OFlags::CREAT) {
        needed_base |= types::Rights::PATH_CREATE_FILE;
    }
    if oflags.contains(OFlags::TRUNC) {
        needed_base |= types::Rights::PATH_FILESTAT_SET_SIZE;
    }

    // convert file descriptor flags
    let fdflags: OFlags = fs_flags.into();
    if fdflags.contains(OFlags::DSYNC) {
        needed_inheriting |= types::Rights::FD_DATASYNC;
    }
    if fdflags.intersects(super::O_RSYNC | OFlags::SYNC) {
        needed_inheriting |= types::Rights::FD_SYNC;
    }

    HandleRights::new(needed_base, needed_inheriting)
}

pub(crate) fn readlinkat(dirfd: &OsDir, path: &str) -> Result<String> {
    use std::os::unix::prelude::AsRawFd;
    use yanix::file::readlinkat;

    log::debug!("path_get readlinkat path = {:?}", path);

    let path = unsafe { readlinkat(dirfd.as_raw_fd(), path)? };
    let path = from_host(path)?;
    Ok(path)
}

pub(crate) fn create_directory(base: &OsDir, path: &str) -> Result<()> {
    use yanix::file::{mkdirat, Mode};
    unsafe { mkdirat(base.as_raw_fd(), path, Mode::from_bits_truncate(0o777))? };
    Ok(())
}

pub(crate) fn link(
    old_dirfd: &OsDir,
    old_path: &str,
    new_dirfd: &OsDir,
    new_path: &str,
    follow_symlinks: bool,
) -> Result<()> {
    use yanix::file::{linkat, AtFlags};
    let flags = if follow_symlinks {
        AtFlags::SYMLINK_FOLLOW
    } else {
        AtFlags::empty()
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
    dirfd: &OsDir,
    path: &str,
    read: bool,
    write: bool,
    oflags: types::Oflags,
    fs_flags: types::Fdflags,
) -> Result<Box<dyn Handle>> {
    use yanix::file::{fstatat, openat, AtFlags, FileType, Mode, OFlags};

    let mut nix_all_oflags = if read && write {
        OFlags::RDWR
    } else if write {
        OFlags::WRONLY
    } else {
        OFlags::RDONLY
    };

    // on non-Capsicum systems, we always want nofollow
    nix_all_oflags.insert(OFlags::NOFOLLOW);

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
                    match unsafe { fstatat(dirfd.as_raw_fd(), path, AtFlags::SYMLINK_NOFOLLOW) } {
                        Ok(stat) => {
                            if FileType::from_stat_st_mode(stat.st_mode) == FileType::Socket {
                                return Err(Error::Notsup);
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
                    if !(nix_all_oflags & (OFlags::NOFOLLOW | OFlags::DIRECTORY)).is_empty() =>
                {
                    match unsafe { fstatat(dirfd.as_raw_fd(), path, AtFlags::SYMLINK_NOFOLLOW) } {
                        Ok(stat) => {
                            if FileType::from_stat_st_mode(stat.st_mode) == FileType::Symlink {
                                return Err(Error::Loop);
                            }
                        }
                        Err(err) => {
                            log::debug!("path_open fstatat error: {:?}", err);
                        }
                    }
                }
                // FreeBSD returns EMLINK instead of ELOOP when using O_NOFOLLOW on
                // a symlink.
                libc::EMLINK if !(nix_all_oflags & OFlags::NOFOLLOW).is_empty() => {
                    return Err(Error::Loop);
                }
                _ => {}
            }

            return Err(e.into());
        }
    };

    log::debug!("path_open (host) new_fd = {:?}", new_fd);

    // Determine the type of the new file descriptor and which rights contradict with this type
    let file = unsafe { File::from_raw_fd(new_fd) };
    let handle = <Box<dyn Handle>>::try_from(file)?;
    Ok(handle)
}

pub(crate) fn readlink(dirfd: &OsDir, path: &str, buf: &mut [u8]) -> Result<usize> {
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

pub(crate) fn remove_directory(dirfd: &OsDir, path: &str) -> Result<()> {
    use yanix::file::{unlinkat, AtFlags};
    unsafe { unlinkat(dirfd.as_raw_fd(), path, AtFlags::REMOVEDIR)? };
    Ok(())
}

pub(crate) fn filestat_get_at(dirfd: &OsDir, path: &str, follow: bool) -> Result<types::Filestat> {
    use yanix::file::{fstatat, AtFlags};
    let flags = if follow {
        AtFlags::empty()
    } else {
        AtFlags::SYMLINK_NOFOLLOW
    };
    let stat = unsafe { fstatat(dirfd.as_raw_fd(), path, flags)? };
    let stat = stat.try_into()?;
    Ok(stat)
}

pub(crate) fn filestat_set_times_at(
    dirfd: &OsDir,
    path: &str,
    atim: types::Timestamp,
    mtim: types::Timestamp,
    fst_flags: types::Fstflags,
    follow: bool,
) -> Result<()> {
    use std::time::{Duration, UNIX_EPOCH};
    use yanix::filetime::*;

    let set_atim = fst_flags.contains(&types::Fstflags::ATIM);
    let set_atim_now = fst_flags.contains(&types::Fstflags::ATIM_NOW);
    let set_mtim = fst_flags.contains(&types::Fstflags::MTIM);
    let set_mtim_now = fst_flags.contains(&types::Fstflags::MTIM_NOW);

    if (set_atim && set_atim_now) || (set_mtim && set_mtim_now) {
        return Err(Error::Inval);
    }

    let atim = if set_atim {
        let time = UNIX_EPOCH + Duration::from_nanos(atim);
        FileTime::FileTime(filetime::FileTime::from_system_time(time))
    } else if set_atim_now {
        FileTime::Now
    } else {
        FileTime::Omit
    };
    let mtim = if set_mtim {
        let time = UNIX_EPOCH + Duration::from_nanos(mtim);
        FileTime::FileTime(filetime::FileTime::from_system_time(time))
    } else if set_mtim_now {
        FileTime::Now
    } else {
        FileTime::Omit
    };

    utimensat(&*dirfd.as_file()?, path, atim, mtim, !follow)?;

    Ok(())
}

#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use crate::helpers::systemtime_to_timestamp;
use crate::hostcalls_impl::{FileType, PathGet};
use crate::sys::host_impl;
use crate::sys::unix::str_to_cstring;
use crate::{wasi, Error, Result};
use nix::libc;
use std::convert::TryInto;
use std::fs::{File, Metadata};
use std::os::unix::fs::FileExt;
use std::os::unix::prelude::{AsRawFd, FromRawFd};

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        pub(crate) use super::super::linux::hostcalls_impl::*;
    } else if #[cfg(any(
            target_os = "macos",
            target_os = "netbsd",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "ios",
            target_os = "dragonfly"
    ))] {
        pub(crate) use super::super::bsd::hostcalls_impl::*;
    }
}

pub(crate) fn fd_pread(
    file: &File,
    buf: &mut [u8],
    offset: wasi::__wasi_filesize_t,
) -> Result<usize> {
    file.read_at(buf, offset).map_err(Into::into)
}

pub(crate) fn fd_pwrite(file: &File, buf: &[u8], offset: wasi::__wasi_filesize_t) -> Result<usize> {
    file.write_at(buf, offset).map_err(Into::into)
}

pub(crate) fn fd_fdstat_get(fd: &File) -> Result<wasi::__wasi_fdflags_t> {
    use nix::fcntl::{fcntl, OFlag, F_GETFL};
    match fcntl(fd.as_raw_fd(), F_GETFL).map(OFlag::from_bits_truncate) {
        Ok(flags) => Ok(host_impl::fdflags_from_nix(flags)),
        Err(e) => Err(host_impl::errno_from_nix(e.as_errno().unwrap())),
    }
}

pub(crate) fn fd_fdstat_set_flags(fd: &File, fdflags: wasi::__wasi_fdflags_t) -> Result<()> {
    use nix::fcntl::{fcntl, F_SETFL};
    let nix_flags = host_impl::nix_from_fdflags(fdflags);
    match fcntl(fd.as_raw_fd(), F_SETFL(nix_flags)) {
        Ok(_) => Ok(()),
        Err(e) => Err(host_impl::errno_from_nix(e.as_errno().unwrap())),
    }
}

pub(crate) fn path_create_directory(resolved: PathGet) -> Result<()> {
    use nix::libc::mkdirat;
    let path_cstr = str_to_cstring(resolved.path())?;
    // nix doesn't expose mkdirat() yet
    match unsafe { mkdirat(resolved.dirfd().as_raw_fd(), path_cstr.as_ptr(), 0o777) } {
        0 => Ok(()),
        _ => Err(host_impl::errno_from_nix(nix::errno::Errno::last())),
    }
}

pub(crate) fn path_link(resolved_old: PathGet, resolved_new: PathGet) -> Result<()> {
    use nix::libc::linkat;
    let old_path_cstr = str_to_cstring(resolved_old.path())?;
    let new_path_cstr = str_to_cstring(resolved_new.path())?;

    // Not setting AT_SYMLINK_FOLLOW fails on most filesystems
    let atflags = libc::AT_SYMLINK_FOLLOW;
    let res = unsafe {
        linkat(
            resolved_old.dirfd().as_raw_fd(),
            old_path_cstr.as_ptr(),
            resolved_new.dirfd().as_raw_fd(),
            new_path_cstr.as_ptr(),
            atflags,
        )
    };
    if res != 0 {
        Err(host_impl::errno_from_nix(nix::errno::Errno::last()))
    } else {
        Ok(())
    }
}

pub(crate) fn path_open(
    resolved: PathGet,
    read: bool,
    write: bool,
    oflags: wasi::__wasi_oflags_t,
    fs_flags: wasi::__wasi_fdflags_t,
) -> Result<File> {
    use nix::errno::Errno;
    use nix::fcntl::{openat, AtFlags, OFlag};
    use nix::sys::stat::{fstatat, Mode, SFlag};

    let mut nix_all_oflags = if read && write {
        OFlag::O_RDWR
    } else if write {
        OFlag::O_WRONLY
    } else {
        OFlag::O_RDONLY
    };

    // on non-Capsicum systems, we always want nofollow
    nix_all_oflags.insert(OFlag::O_NOFOLLOW);

    // convert open flags
    nix_all_oflags.insert(host_impl::nix_from_oflags(oflags));

    // convert file descriptor flags
    nix_all_oflags.insert(host_impl::nix_from_fdflags(fs_flags));

    // Call openat. Use mode 0o666 so that we follow whatever the user's
    // umask is, but don't set the executable flag, because it isn't yet
    // meaningful for WASI programs to create executable files.

    log::debug!("path_open resolved = {:?}", resolved);
    log::debug!("path_open oflags = {:?}", nix_all_oflags);

    let new_fd = match openat(
        resolved.dirfd().as_raw_fd(),
        resolved.path(),
        nix_all_oflags,
        Mode::from_bits_truncate(0o666),
    ) {
        Ok(fd) => fd,
        Err(e) => {
            match e.as_errno() {
                // Linux returns ENXIO instead of EOPNOTSUPP when opening a socket
                Some(Errno::ENXIO) => {
                    if let Ok(stat) = fstatat(
                        resolved.dirfd().as_raw_fd(),
                        resolved.path(),
                        AtFlags::AT_SYMLINK_NOFOLLOW,
                    ) {
                        if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFSOCK) {
                            return Err(Error::ENOTSUP);
                        } else {
                            return Err(Error::ENXIO);
                        }
                    } else {
                        return Err(Error::ENXIO);
                    }
                }
                // Linux returns ENOTDIR instead of ELOOP when using O_NOFOLLOW|O_DIRECTORY
                // on a symlink.
                Some(Errno::ENOTDIR)
                    if !(nix_all_oflags & (OFlag::O_NOFOLLOW | OFlag::O_DIRECTORY)).is_empty() =>
                {
                    if let Ok(stat) = fstatat(
                        resolved.dirfd().as_raw_fd(),
                        resolved.path(),
                        AtFlags::AT_SYMLINK_NOFOLLOW,
                    ) {
                        if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFLNK) {
                            return Err(Error::ELOOP);
                        }
                    }
                    return Err(Error::ENOTDIR);
                }
                // FreeBSD returns EMLINK instead of ELOOP when using O_NOFOLLOW on
                // a symlink.
                Some(Errno::EMLINK) if !(nix_all_oflags & OFlag::O_NOFOLLOW).is_empty() => {
                    return Err(Error::ELOOP);
                }
                Some(e) => return Err(host_impl::errno_from_nix(e)),
                None => return Err(Error::ENOSYS),
            }
        }
    };

    log::debug!("path_open (host) new_fd = {:?}", new_fd);

    // Determine the type of the new file descriptor and which rights contradict with this type
    Ok(unsafe { File::from_raw_fd(new_fd) })
}

pub(crate) fn path_readlink(resolved: PathGet, buf: &mut [u8]) -> Result<usize> {
    use nix::errno::Errno;
    let path_cstr = str_to_cstring(resolved.path())?;

    // Linux requires that the buffer size is positive, whereas POSIX does not.
    // Use a fake buffer to store the results if the size is zero.
    // TODO: instead of using raw libc::readlinkat call here, this should really
    // be fixed in `nix` crate
    let fakebuf: &mut [u8] = &mut [0];
    let buf_len = buf.len();
    let len = unsafe {
        libc::readlinkat(
            resolved.dirfd().as_raw_fd(),
            path_cstr.as_ptr() as *const libc::c_char,
            if buf_len == 0 {
                fakebuf.as_mut_ptr()
            } else {
                buf.as_mut_ptr()
            } as *mut libc::c_char,
            if buf_len == 0 { fakebuf.len() } else { buf_len },
        )
    };

    if len < 0 {
        Err(host_impl::errno_from_nix(Errno::last()))
    } else {
        let len = len as usize;
        Ok(if len < buf_len { len } else { buf_len })
    }
}

pub(crate) fn fd_filestat_get_impl(file: &std::fs::File) -> Result<wasi::__wasi_filestat_t> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata()?;
    Ok(wasi::__wasi_filestat_t {
        dev: metadata.dev(),
        ino: metadata.ino(),
        nlink: metadata.nlink().try_into()?, // u64 doesn't fit into u32
        size: metadata.len(),
        atim: systemtime_to_timestamp(metadata.accessed()?)?,
        ctim: metadata.ctime().try_into()?, // i64 doesn't fit into u64
        mtim: systemtime_to_timestamp(metadata.modified()?)?,
        filetype: filetype(file, &metadata)?.to_wasi(),
    })
}

fn filetype(file: &File, metadata: &Metadata) -> Result<FileType> {
    use nix::sys::socket::{self, SockType};
    use std::os::unix::fs::FileTypeExt;
    let ftype = metadata.file_type();
    if ftype.is_file() {
        Ok(FileType::RegularFile)
    } else if ftype.is_dir() {
        Ok(FileType::Directory)
    } else if ftype.is_symlink() {
        Ok(FileType::Symlink)
    } else if ftype.is_char_device() {
        Ok(FileType::CharacterDevice)
    } else if ftype.is_block_device() {
        Ok(FileType::BlockDevice)
    } else if ftype.is_socket() {
        match socket::getsockopt(file.as_raw_fd(), socket::sockopt::SockType)
            .map_err(|err| err.as_errno().unwrap())
            .map_err(host_impl::errno_from_nix)?
        {
            SockType::Datagram => Ok(FileType::SocketDgram),
            SockType::Stream => Ok(FileType::SocketStream),
            _ => Ok(FileType::Unknown),
        }
    } else {
        Ok(FileType::Unknown)
    }
}

pub(crate) fn path_filestat_get(
    resolved: PathGet,
    dirflags: wasi::__wasi_lookupflags_t,
) -> Result<wasi::__wasi_filestat_t> {
    use nix::fcntl::AtFlags;
    use nix::sys::stat::fstatat;

    let atflags = match dirflags {
        0 => AtFlags::empty(),
        _ => AtFlags::AT_SYMLINK_NOFOLLOW,
    };

    let filestat = fstatat(resolved.dirfd().as_raw_fd(), resolved.path(), atflags)
        .map_err(|err| host_impl::errno_from_nix(err.as_errno().unwrap()))?;
    host_impl::filestat_from_nix(filestat)
}

pub(crate) fn path_filestat_set_times(
    resolved: PathGet,
    dirflags: wasi::__wasi_lookupflags_t,
    st_atim: wasi::__wasi_timestamp_t,
    st_mtim: wasi::__wasi_timestamp_t,
    fst_flags: wasi::__wasi_fstflags_t,
) -> Result<()> {
    use super::super::filetime::*;
    use std::time::{Duration, UNIX_EPOCH};

    let set_atim = fst_flags & wasi::__WASI_FSTFLAGS_ATIM != 0;
    let set_atim_now = fst_flags & wasi::__WASI_FSTFLAGS_ATIM_NOW != 0;
    let set_mtim = fst_flags & wasi::__WASI_FSTFLAGS_MTIM != 0;
    let set_mtim_now = fst_flags & wasi::__WASI_FSTFLAGS_MTIM_NOW != 0;

    if (set_atim && set_atim_now) || (set_mtim && set_mtim_now) {
        return Err(Error::EINVAL);
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
        resolved.dirfd(),
        resolved.path(),
        atim,
        mtim,
        symlink_nofollow,
    )
    .map_err(Into::into)
}

pub(crate) fn path_remove_directory(resolved: PathGet) -> Result<()> {
    use nix::errno;
    use nix::libc::{unlinkat, AT_REMOVEDIR};

    let path_cstr = str_to_cstring(resolved.path())?;

    // nix doesn't expose unlinkat() yet
    match unsafe {
        unlinkat(
            resolved.dirfd().as_raw_fd(),
            path_cstr.as_ptr(),
            AT_REMOVEDIR,
        )
    } {
        0 => Ok(()),
        _ => Err(host_impl::errno_from_nix(errno::Errno::last())),
    }
}

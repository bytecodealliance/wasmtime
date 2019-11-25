//! WASI host types specific to *nix host.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::hostcalls_impl::FileType;
use crate::{helpers, wasi, Error, Result};
use log::warn;
use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        pub(crate) use super::linux::host_impl::*;
    } else if #[cfg(any(
            target_os = "macos",
            target_os = "netbsd",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "ios",
            target_os = "dragonfly"
    ))] {
        pub(crate) use super::bsd::host_impl::*;
    }
}

pub(crate) fn errno_from_nix(errno: nix::errno::Errno) -> Error {
    match errno {
        nix::errno::Errno::EPERM => Error::EPERM,
        nix::errno::Errno::ENOENT => Error::ENOENT,
        nix::errno::Errno::ESRCH => Error::ESRCH,
        nix::errno::Errno::EINTR => Error::EINTR,
        nix::errno::Errno::EIO => Error::EIO,
        nix::errno::Errno::ENXIO => Error::ENXIO,
        nix::errno::Errno::E2BIG => Error::E2BIG,
        nix::errno::Errno::ENOEXEC => Error::ENOEXEC,
        nix::errno::Errno::EBADF => Error::EBADF,
        nix::errno::Errno::ECHILD => Error::ECHILD,
        nix::errno::Errno::EAGAIN => Error::EAGAIN,
        nix::errno::Errno::ENOMEM => Error::ENOMEM,
        nix::errno::Errno::EACCES => Error::EACCES,
        nix::errno::Errno::EFAULT => Error::EFAULT,
        nix::errno::Errno::EBUSY => Error::EBUSY,
        nix::errno::Errno::EEXIST => Error::EEXIST,
        nix::errno::Errno::EXDEV => Error::EXDEV,
        nix::errno::Errno::ENODEV => Error::ENODEV,
        nix::errno::Errno::ENOTDIR => Error::ENOTDIR,
        nix::errno::Errno::EISDIR => Error::EISDIR,
        nix::errno::Errno::EINVAL => Error::EINVAL,
        nix::errno::Errno::ENFILE => Error::ENFILE,
        nix::errno::Errno::EMFILE => Error::EMFILE,
        nix::errno::Errno::ENOTTY => Error::ENOTTY,
        nix::errno::Errno::ETXTBSY => Error::ETXTBSY,
        nix::errno::Errno::EFBIG => Error::EFBIG,
        nix::errno::Errno::ENOSPC => Error::ENOSPC,
        nix::errno::Errno::ESPIPE => Error::ESPIPE,
        nix::errno::Errno::EROFS => Error::EROFS,
        nix::errno::Errno::EMLINK => Error::EMLINK,
        nix::errno::Errno::EPIPE => Error::EPIPE,
        nix::errno::Errno::EDOM => Error::EDOM,
        nix::errno::Errno::ERANGE => Error::ERANGE,
        nix::errno::Errno::EDEADLK => Error::EDEADLK,
        nix::errno::Errno::ENAMETOOLONG => Error::ENAMETOOLONG,
        nix::errno::Errno::ENOLCK => Error::ENOLCK,
        nix::errno::Errno::ENOSYS => Error::ENOSYS,
        nix::errno::Errno::ENOTEMPTY => Error::ENOTEMPTY,
        nix::errno::Errno::ELOOP => Error::ELOOP,
        nix::errno::Errno::ENOMSG => Error::ENOMSG,
        nix::errno::Errno::EIDRM => Error::EIDRM,
        nix::errno::Errno::ENOLINK => Error::ENOLINK,
        nix::errno::Errno::EPROTO => Error::EPROTO,
        nix::errno::Errno::EMULTIHOP => Error::EMULTIHOP,
        nix::errno::Errno::EBADMSG => Error::EBADMSG,
        nix::errno::Errno::EOVERFLOW => Error::EOVERFLOW,
        nix::errno::Errno::EILSEQ => Error::EILSEQ,
        nix::errno::Errno::ENOTSOCK => Error::ENOTSOCK,
        nix::errno::Errno::EDESTADDRREQ => Error::EDESTADDRREQ,
        nix::errno::Errno::EMSGSIZE => Error::EMSGSIZE,
        nix::errno::Errno::EPROTOTYPE => Error::EPROTOTYPE,
        nix::errno::Errno::ENOPROTOOPT => Error::ENOPROTOOPT,
        nix::errno::Errno::EPROTONOSUPPORT => Error::EPROTONOSUPPORT,
        nix::errno::Errno::EAFNOSUPPORT => Error::EAFNOSUPPORT,
        nix::errno::Errno::EADDRINUSE => Error::EADDRINUSE,
        nix::errno::Errno::EADDRNOTAVAIL => Error::EADDRNOTAVAIL,
        nix::errno::Errno::ENETDOWN => Error::ENETDOWN,
        nix::errno::Errno::ENETUNREACH => Error::ENETUNREACH,
        nix::errno::Errno::ENETRESET => Error::ENETRESET,
        nix::errno::Errno::ECONNABORTED => Error::ECONNABORTED,
        nix::errno::Errno::ECONNRESET => Error::ECONNRESET,
        nix::errno::Errno::ENOBUFS => Error::ENOBUFS,
        nix::errno::Errno::EISCONN => Error::EISCONN,
        nix::errno::Errno::ENOTCONN => Error::ENOTCONN,
        nix::errno::Errno::ETIMEDOUT => Error::ETIMEDOUT,
        nix::errno::Errno::ECONNREFUSED => Error::ECONNREFUSED,
        nix::errno::Errno::EHOSTUNREACH => Error::EHOSTUNREACH,
        nix::errno::Errno::EALREADY => Error::EALREADY,
        nix::errno::Errno::EINPROGRESS => Error::EINPROGRESS,
        nix::errno::Errno::ESTALE => Error::ESTALE,
        nix::errno::Errno::EDQUOT => Error::EDQUOT,
        nix::errno::Errno::ECANCELED => Error::ECANCELED,
        nix::errno::Errno::EOWNERDEAD => Error::EOWNERDEAD,
        nix::errno::Errno::ENOTRECOVERABLE => Error::ENOTRECOVERABLE,
        other => {
            warn!("Unknown error from nix: {}", other);
            Error::ENOSYS
        }
    }
}

pub(crate) fn nix_from_fdflags(fdflags: wasi::__wasi_fdflags_t) -> nix::fcntl::OFlag {
    use nix::fcntl::OFlag;
    let mut nix_flags = OFlag::empty();
    if fdflags & wasi::__WASI_FDFLAGS_APPEND != 0 {
        nix_flags.insert(OFlag::O_APPEND);
    }
    if fdflags & wasi::__WASI_FDFLAGS_DSYNC != 0 {
        nix_flags.insert(OFlag::O_DSYNC);
    }
    if fdflags & wasi::__WASI_FDFLAGS_NONBLOCK != 0 {
        nix_flags.insert(OFlag::O_NONBLOCK);
    }
    if fdflags & wasi::__WASI_FDFLAGS_RSYNC != 0 {
        nix_flags.insert(O_RSYNC);
    }
    if fdflags & wasi::__WASI_FDFLAGS_SYNC != 0 {
        nix_flags.insert(OFlag::O_SYNC);
    }
    nix_flags
}

pub(crate) fn fdflags_from_nix(oflags: nix::fcntl::OFlag) -> wasi::__wasi_fdflags_t {
    use nix::fcntl::OFlag;
    let mut fdflags = 0;
    if oflags.contains(OFlag::O_APPEND) {
        fdflags |= wasi::__WASI_FDFLAGS_APPEND;
    }
    if oflags.contains(OFlag::O_DSYNC) {
        fdflags |= wasi::__WASI_FDFLAGS_DSYNC;
    }
    if oflags.contains(OFlag::O_NONBLOCK) {
        fdflags |= wasi::__WASI_FDFLAGS_NONBLOCK;
    }
    if oflags.contains(O_RSYNC) {
        fdflags |= wasi::__WASI_FDFLAGS_RSYNC;
    }
    if oflags.contains(OFlag::O_SYNC) {
        fdflags |= wasi::__WASI_FDFLAGS_SYNC;
    }
    fdflags
}

pub(crate) fn nix_from_oflags(oflags: wasi::__wasi_oflags_t) -> nix::fcntl::OFlag {
    use nix::fcntl::OFlag;
    let mut nix_flags = OFlag::empty();
    if oflags & wasi::__WASI_OFLAGS_CREAT != 0 {
        nix_flags.insert(OFlag::O_CREAT);
    }
    if oflags & wasi::__WASI_OFLAGS_DIRECTORY != 0 {
        nix_flags.insert(OFlag::O_DIRECTORY);
    }
    if oflags & wasi::__WASI_OFLAGS_EXCL != 0 {
        nix_flags.insert(OFlag::O_EXCL);
    }
    if oflags & wasi::__WASI_OFLAGS_TRUNC != 0 {
        nix_flags.insert(OFlag::O_TRUNC);
    }
    nix_flags
}

pub(crate) fn filetype_from_nix(sflags: nix::sys::stat::SFlag) -> FileType {
    use nix::sys::stat::SFlag;
    if sflags.contains(SFlag::S_IFCHR) {
        FileType::CharacterDevice
    } else if sflags.contains(SFlag::S_IFBLK) {
        FileType::BlockDevice
    } else if sflags.contains(SFlag::S_IFSOCK) {
        FileType::SocketStream
    } else if sflags.contains(SFlag::S_IFDIR) {
        FileType::Directory
    } else if sflags.contains(SFlag::S_IFREG) {
        FileType::RegularFile
    } else if sflags.contains(SFlag::S_IFLNK) {
        FileType::Symlink
    } else {
        FileType::Unknown
    }
}

pub(crate) fn filestat_from_nix(
    filestat: nix::sys::stat::FileStat,
) -> Result<wasi::__wasi_filestat_t> {
    fn filestat_to_timestamp(secs: u64, nsecs: u64) -> Result<wasi::__wasi_timestamp_t> {
        secs.checked_mul(1_000_000_000)
            .and_then(|sec_nsec| sec_nsec.checked_add(nsecs))
            .ok_or(Error::EOVERFLOW)
    }

    let filetype = nix::sys::stat::SFlag::from_bits_truncate(filestat.st_mode);
    let dev = stdev_from_nix(filestat.st_dev)?;
    let ino = stino_from_nix(filestat.st_ino)?;
    let atim = filestat_to_timestamp(filestat.st_atime as u64, filestat.st_atime_nsec as u64)?;
    let ctim = filestat_to_timestamp(filestat.st_ctime as u64, filestat.st_ctime_nsec as u64)?;
    let mtim = filestat_to_timestamp(filestat.st_mtime as u64, filestat.st_mtime_nsec as u64)?;

    Ok(wasi::__wasi_filestat_t {
        dev,
        ino,
        nlink: wasi::__wasi_linkcount_t::from(filestat.st_nlink),
        size: filestat.st_size as wasi::__wasi_filesize_t,
        atim,
        ctim,
        mtim,
        filetype: filetype_from_nix(filetype).to_wasi(),
    })
}

pub(crate) fn dirent_filetype_from_host(
    host_entry: &nix::libc::dirent,
) -> Result<wasi::__wasi_filetype_t> {
    match host_entry.d_type {
        libc::DT_FIFO => Ok(wasi::__WASI_FILETYPE_UNKNOWN),
        libc::DT_CHR => Ok(wasi::__WASI_FILETYPE_CHARACTER_DEVICE),
        libc::DT_DIR => Ok(wasi::__WASI_FILETYPE_DIRECTORY),
        libc::DT_BLK => Ok(wasi::__WASI_FILETYPE_BLOCK_DEVICE),
        libc::DT_REG => Ok(wasi::__WASI_FILETYPE_REGULAR_FILE),
        libc::DT_LNK => Ok(wasi::__WASI_FILETYPE_SYMBOLIC_LINK),
        libc::DT_SOCK => {
            // TODO how to discriminate between STREAM and DGRAM?
            // Perhaps, we should create a more general WASI filetype
            // such as __WASI_FILETYPE_SOCKET, and then it would be
            // up to the client to check whether it's actually
            // STREAM or DGRAM?
            Ok(wasi::__WASI_FILETYPE_UNKNOWN)
        }
        libc::DT_UNKNOWN => Ok(wasi::__WASI_FILETYPE_UNKNOWN),
        _ => Err(Error::EINVAL),
    }
}

/// Creates owned WASI path from OS string.
///
/// NB WASI spec requires OS string to be valid UTF-8. Otherwise,
/// `__WASI_ERRNO_ILSEQ` error is returned.
pub(crate) fn path_from_host<S: AsRef<OsStr>>(s: S) -> Result<String> {
    helpers::path_from_slice(s.as_ref().as_bytes()).map(String::from)
}

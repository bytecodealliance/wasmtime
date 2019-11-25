//! WASI host types specific to *nix host.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::hostcalls_impl::FileType;
use crate::{helpers, wasi, Error, Result};
use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;
use yanix::{errno, file::OFlag, sys::SFlag};

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "macos",
                 target_os = "ios",
                 target_os = "dragonfly",
                 target_os = "freebsd"))] {
        pub(crate) const O_RSYNC: OFlag = OFlag::O_SYNC;

        use std::convert::TryFrom;

        pub(crate) fn stdev_from_nix(dev: libc::dev_t) -> Result<wasi::__wasi_device_t> {
            wasi::__wasi_device_t::try_from(dev).map_err(Into::into)
        }

        pub(crate) fn stino_from_nix(ino: libc::ino_t) -> Result<wasi::__wasi_inode_t> {
            wasi::__wasi_device_t::try_from(ino).map_err(Into::into)
        }
    } else {
        pub(crate) const O_RSYNC: OFlag = OFlag::O_RSYNC;

        pub(crate) fn stdev_from_nix(dev: libc::dev_t) -> Result<wasi::__wasi_device_t> {
            Ok(wasi::__wasi_device_t::from(dev))
        }

        pub(crate) fn stino_from_nix(ino: libc::ino_t) -> Result<wasi::__wasi_inode_t> {
            Ok(wasi::__wasi_device_t::from(ino))
        }
    }
}

pub(crate) fn errno_from_nix(errno: errno::Errno) -> Error {
    match errno {
        errno::Errno::EPERM => Error::EPERM,
        errno::Errno::ENOENT => Error::ENOENT,
        errno::Errno::ESRCH => Error::ESRCH,
        errno::Errno::EINTR => Error::EINTR,
        errno::Errno::EIO => Error::EIO,
        errno::Errno::ENXIO => Error::ENXIO,
        errno::Errno::E2BIG => Error::E2BIG,
        errno::Errno::ENOEXEC => Error::ENOEXEC,
        errno::Errno::EBADF => Error::EBADF,
        errno::Errno::ECHILD => Error::ECHILD,
        errno::Errno::EAGAIN => Error::EAGAIN,
        errno::Errno::ENOMEM => Error::ENOMEM,
        errno::Errno::EACCES => Error::EACCES,
        errno::Errno::EFAULT => Error::EFAULT,
        errno::Errno::EBUSY => Error::EBUSY,
        errno::Errno::EEXIST => Error::EEXIST,
        errno::Errno::EXDEV => Error::EXDEV,
        errno::Errno::ENODEV => Error::ENODEV,
        errno::Errno::ENOTDIR => Error::ENOTDIR,
        errno::Errno::EISDIR => Error::EISDIR,
        errno::Errno::EINVAL => Error::EINVAL,
        errno::Errno::ENFILE => Error::ENFILE,
        errno::Errno::EMFILE => Error::EMFILE,
        errno::Errno::ENOTTY => Error::ENOTTY,
        errno::Errno::ETXTBSY => Error::ETXTBSY,
        errno::Errno::EFBIG => Error::EFBIG,
        errno::Errno::ENOSPC => Error::ENOSPC,
        errno::Errno::ESPIPE => Error::ESPIPE,
        errno::Errno::EROFS => Error::EROFS,
        errno::Errno::EMLINK => Error::EMLINK,
        errno::Errno::EPIPE => Error::EPIPE,
        errno::Errno::EDOM => Error::EDOM,
        errno::Errno::ERANGE => Error::ERANGE,
        errno::Errno::EDEADLK => Error::EDEADLK,
        errno::Errno::ENAMETOOLONG => Error::ENAMETOOLONG,
        errno::Errno::ENOLCK => Error::ENOLCK,
        errno::Errno::ENOSYS => Error::ENOSYS,
        errno::Errno::ENOTEMPTY => Error::ENOTEMPTY,
        errno::Errno::ELOOP => Error::ELOOP,
        errno::Errno::ENOMSG => Error::ENOMSG,
        errno::Errno::EIDRM => Error::EIDRM,
        errno::Errno::ENOLINK => Error::ENOLINK,
        errno::Errno::EPROTO => Error::EPROTO,
        errno::Errno::EMULTIHOP => Error::EMULTIHOP,
        errno::Errno::EBADMSG => Error::EBADMSG,
        errno::Errno::EOVERFLOW => Error::EOVERFLOW,
        errno::Errno::EILSEQ => Error::EILSEQ,
        errno::Errno::ENOTSOCK => Error::ENOTSOCK,
        errno::Errno::EDESTADDRREQ => Error::EDESTADDRREQ,
        errno::Errno::EMSGSIZE => Error::EMSGSIZE,
        errno::Errno::EPROTOTYPE => Error::EPROTOTYPE,
        errno::Errno::ENOPROTOOPT => Error::ENOPROTOOPT,
        errno::Errno::EPROTONOSUPPORT => Error::EPROTONOSUPPORT,
        errno::Errno::EAFNOSUPPORT => Error::EAFNOSUPPORT,
        errno::Errno::EADDRINUSE => Error::EADDRINUSE,
        errno::Errno::EADDRNOTAVAIL => Error::EADDRNOTAVAIL,
        errno::Errno::ENETDOWN => Error::ENETDOWN,
        errno::Errno::ENETUNREACH => Error::ENETUNREACH,
        errno::Errno::ENETRESET => Error::ENETRESET,
        errno::Errno::ECONNABORTED => Error::ECONNABORTED,
        errno::Errno::ECONNRESET => Error::ECONNRESET,
        errno::Errno::ENOBUFS => Error::ENOBUFS,
        errno::Errno::EISCONN => Error::EISCONN,
        errno::Errno::ENOTCONN => Error::ENOTCONN,
        errno::Errno::ETIMEDOUT => Error::ETIMEDOUT,
        errno::Errno::ECONNREFUSED => Error::ECONNREFUSED,
        errno::Errno::EHOSTUNREACH => Error::EHOSTUNREACH,
        errno::Errno::EALREADY => Error::EALREADY,
        errno::Errno::EINPROGRESS => Error::EINPROGRESS,
        errno::Errno::ESTALE => Error::ESTALE,
        errno::Errno::EDQUOT => Error::EDQUOT,
        errno::Errno::ECANCELED => Error::ECANCELED,
        errno::Errno::EOWNERDEAD => Error::EOWNERDEAD,
        errno::Errno::ENOTRECOVERABLE => Error::ENOTRECOVERABLE,
    }
}

pub(crate) fn nix_from_fdflags(fdflags: wasi::__wasi_fdflags_t) -> OFlag {
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

pub(crate) fn fdflags_from_nix(oflags: OFlag) -> wasi::__wasi_fdflags_t {
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

pub(crate) fn nix_from_oflags(oflags: wasi::__wasi_oflags_t) -> OFlag {
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

pub(crate) fn filetype_from_nix(sflags: SFlag) -> FileType {
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

pub(crate) fn filestat_from_nix(filestat: libc::stat) -> Result<wasi::__wasi_filestat_t> {
    fn filestat_to_timestamp(secs: u64, nsecs: u64) -> Result<wasi::__wasi_timestamp_t> {
        secs.checked_mul(1_000_000_000)
            .and_then(|sec_nsec| sec_nsec.checked_add(nsecs))
            .ok_or(Error::EOVERFLOW)
    }

    let filetype = SFlag::from_bits_truncate(filestat.st_mode);
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
    host_entry: &libc::dirent,
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

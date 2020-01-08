//! WASI host types specific to *nix host.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::old::snapshot_0::host::FileType;
use crate::old::snapshot_0::{helpers, sys::unix::sys_impl, wasi, Error, Result};
use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;
use yanix::{
    file::{OFlag, SFlag},
    Errno,
};

pub(crate) use sys_impl::host_impl::*;

pub(crate) fn errno_from_nix(errno: Errno) -> Error {
    match errno {
        Errno::EPERM => Error::EPERM,
        Errno::ENOENT => Error::ENOENT,
        Errno::ESRCH => Error::ESRCH,
        Errno::EINTR => Error::EINTR,
        Errno::EIO => Error::EIO,
        Errno::ENXIO => Error::ENXIO,
        Errno::E2BIG => Error::E2BIG,
        Errno::ENOEXEC => Error::ENOEXEC,
        Errno::EBADF => Error::EBADF,
        Errno::ECHILD => Error::ECHILD,
        Errno::EAGAIN => Error::EAGAIN,
        Errno::ENOMEM => Error::ENOMEM,
        Errno::EACCES => Error::EACCES,
        Errno::EFAULT => Error::EFAULT,
        Errno::EBUSY => Error::EBUSY,
        Errno::EEXIST => Error::EEXIST,
        Errno::EXDEV => Error::EXDEV,
        Errno::ENODEV => Error::ENODEV,
        Errno::ENOTDIR => Error::ENOTDIR,
        Errno::EISDIR => Error::EISDIR,
        Errno::EINVAL => Error::EINVAL,
        Errno::ENFILE => Error::ENFILE,
        Errno::EMFILE => Error::EMFILE,
        Errno::ENOTTY => Error::ENOTTY,
        Errno::ETXTBSY => Error::ETXTBSY,
        Errno::EFBIG => Error::EFBIG,
        Errno::ENOSPC => Error::ENOSPC,
        Errno::ESPIPE => Error::ESPIPE,
        Errno::EROFS => Error::EROFS,
        Errno::EMLINK => Error::EMLINK,
        Errno::EPIPE => Error::EPIPE,
        Errno::EDOM => Error::EDOM,
        Errno::ERANGE => Error::ERANGE,
        Errno::EDEADLK => Error::EDEADLK,
        Errno::ENAMETOOLONG => Error::ENAMETOOLONG,
        Errno::ENOLCK => Error::ENOLCK,
        Errno::ENOSYS => Error::ENOSYS,
        Errno::ENOTEMPTY => Error::ENOTEMPTY,
        Errno::ELOOP => Error::ELOOP,
        Errno::ENOMSG => Error::ENOMSG,
        Errno::EIDRM => Error::EIDRM,
        Errno::ENOLINK => Error::ENOLINK,
        Errno::EPROTO => Error::EPROTO,
        Errno::EMULTIHOP => Error::EMULTIHOP,
        Errno::EBADMSG => Error::EBADMSG,
        Errno::EOVERFLOW => Error::EOVERFLOW,
        Errno::EILSEQ => Error::EILSEQ,
        Errno::ENOTSOCK => Error::ENOTSOCK,
        Errno::EDESTADDRREQ => Error::EDESTADDRREQ,
        Errno::EMSGSIZE => Error::EMSGSIZE,
        Errno::EPROTOTYPE => Error::EPROTOTYPE,
        Errno::ENOPROTOOPT => Error::ENOPROTOOPT,
        Errno::EPROTONOSUPPORT => Error::EPROTONOSUPPORT,
        Errno::EAFNOSUPPORT => Error::EAFNOSUPPORT,
        Errno::EADDRINUSE => Error::EADDRINUSE,
        Errno::EADDRNOTAVAIL => Error::EADDRNOTAVAIL,
        Errno::ENETDOWN => Error::ENETDOWN,
        Errno::ENETUNREACH => Error::ENETUNREACH,
        Errno::ENETRESET => Error::ENETRESET,
        Errno::ECONNABORTED => Error::ECONNABORTED,
        Errno::ECONNRESET => Error::ECONNRESET,
        Errno::ENOBUFS => Error::ENOBUFS,
        Errno::EISCONN => Error::EISCONN,
        Errno::ENOTCONN => Error::ENOTCONN,
        Errno::ETIMEDOUT => Error::ETIMEDOUT,
        Errno::ECONNREFUSED => Error::ECONNREFUSED,
        Errno::EHOSTUNREACH => Error::EHOSTUNREACH,
        Errno::EALREADY => Error::EALREADY,
        Errno::EINPROGRESS => Error::EINPROGRESS,
        Errno::ESTALE => Error::ESTALE,
        Errno::EDQUOT => Error::EDQUOT,
        Errno::ECANCELED => Error::ECANCELED,
        Errno::EOWNERDEAD => Error::EOWNERDEAD,
        Errno::ENOTRECOVERABLE => Error::ENOTRECOVERABLE,
    }
}

pub(crate) fn nix_from_fdflags(fdflags: wasi::__wasi_fdflags_t) -> OFlag {
    let mut nix_flags = OFlag::empty();
    if fdflags & wasi::__WASI_FDFLAGS_APPEND != 0 {
        nix_flags.insert(OFlag::APPEND);
    }
    if fdflags & wasi::__WASI_FDFLAGS_DSYNC != 0 {
        nix_flags.insert(OFlag::DSYNC);
    }
    if fdflags & wasi::__WASI_FDFLAGS_NONBLOCK != 0 {
        nix_flags.insert(OFlag::NONBLOCK);
    }
    if fdflags & wasi::__WASI_FDFLAGS_RSYNC != 0 {
        nix_flags.insert(O_RSYNC);
    }
    if fdflags & wasi::__WASI_FDFLAGS_SYNC != 0 {
        nix_flags.insert(OFlag::SYNC);
    }
    nix_flags
}

pub(crate) fn fdflags_from_nix(oflags: OFlag) -> wasi::__wasi_fdflags_t {
    let mut fdflags = 0;
    if oflags.contains(OFlag::APPEND) {
        fdflags |= wasi::__WASI_FDFLAGS_APPEND;
    }
    if oflags.contains(OFlag::DSYNC) {
        fdflags |= wasi::__WASI_FDFLAGS_DSYNC;
    }
    if oflags.contains(OFlag::NONBLOCK) {
        fdflags |= wasi::__WASI_FDFLAGS_NONBLOCK;
    }
    if oflags.contains(O_RSYNC) {
        fdflags |= wasi::__WASI_FDFLAGS_RSYNC;
    }
    if oflags.contains(OFlag::SYNC) {
        fdflags |= wasi::__WASI_FDFLAGS_SYNC;
    }
    fdflags
}

pub(crate) fn nix_from_oflags(oflags: wasi::__wasi_oflags_t) -> OFlag {
    let mut nix_flags = OFlag::empty();
    if oflags & wasi::__WASI_OFLAGS_CREAT != 0 {
        nix_flags.insert(OFlag::CREAT);
    }
    if oflags & wasi::__WASI_OFLAGS_DIRECTORY != 0 {
        nix_flags.insert(OFlag::DIRECTORY);
    }
    if oflags & wasi::__WASI_OFLAGS_EXCL != 0 {
        nix_flags.insert(OFlag::EXCL);
    }
    if oflags & wasi::__WASI_OFLAGS_TRUNC != 0 {
        nix_flags.insert(OFlag::TRUNC);
    }
    nix_flags
}

pub(crate) fn filetype_from_nix(sflags: SFlag) -> FileType {
    if sflags.contains(SFlag::IFCHR) {
        FileType::CharacterDevice
    } else if sflags.contains(SFlag::IFBLK) {
        FileType::BlockDevice
    } else if sflags.contains(SFlag::IFSOCK) {
        FileType::SocketStream
    } else if sflags.contains(SFlag::IFDIR) {
        FileType::Directory
    } else if sflags.contains(SFlag::IFREG) {
        FileType::RegularFile
    } else if sflags.contains(SFlag::IFLNK) {
        FileType::Symlink
    } else {
        FileType::Unknown
    }
}

pub(crate) fn filestat_from_nix(filestat: libc::stat) -> Result<wasi::__wasi_filestat_t> {
    use std::convert::TryInto;

    fn filestat_to_timestamp(secs: u64, nsecs: u64) -> Result<wasi::__wasi_timestamp_t> {
        secs.checked_mul(1_000_000_000)
            .and_then(|sec_nsec| sec_nsec.checked_add(nsecs))
            .ok_or(Error::EOVERFLOW)
    }

    let filetype = SFlag::from_bits_truncate(filestat.st_mode);
    let dev = stdev_from_nix(filestat.st_dev)?;
    let ino = stino_from_nix(filestat.st_ino)?;
    let atim = filestat_to_timestamp(
        filestat.st_atime.try_into()?,
        filestat.st_atime_nsec.try_into()?,
    )?;
    let ctim = filestat_to_timestamp(
        filestat.st_ctime.try_into()?,
        filestat.st_ctime_nsec.try_into()?,
    )?;
    let mtim = filestat_to_timestamp(
        filestat.st_mtime.try_into()?,
        filestat.st_mtime_nsec.try_into()?,
    )?;

    Ok(wasi::__wasi_filestat_t {
        dev,
        ino,
        nlink: stnlink_from_nix(filestat.st_nlink)?,
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

impl From<yanix::dir::FileType> for FileType {
    fn from(ft: yanix::dir::FileType) -> Self {
        use yanix::dir::FileType::*;
        match ft {
            RegularFile => Self::RegularFile,
            Symlink => Self::Symlink,
            Directory => Self::Directory,
            BlockDevice => Self::BlockDevice,
            CharacterDevice => Self::CharacterDevice,
            /* Unknown | Socket | Fifo */ _ => Self::Unknown,
        }
    }
}

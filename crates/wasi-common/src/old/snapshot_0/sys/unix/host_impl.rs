//! WASI host types specific to *nix host.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::old::snapshot_0::host::FileType;
use crate::old::snapshot_0::{
    error::FromRawOsError, helpers, sys::unix::sys_impl, wasi, Error, Result,
};
use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;
use yanix::{file::OFlag, Errno};

pub(crate) use sys_impl::host_impl::*;

impl FromRawOsError for Error {
    fn from_raw_os_error(code: i32) -> Self {
        Self::from(Errno::from_i32(code))
    }
}

impl From<Errno> for Error {
    fn from(errno: Errno) -> Self {
        match errno {
            Errno::EPERM => Self::EPERM,
            Errno::ENOENT => Self::ENOENT,
            Errno::ESRCH => Self::ESRCH,
            Errno::EINTR => Self::EINTR,
            Errno::EIO => Self::EIO,
            Errno::ENXIO => Self::ENXIO,
            Errno::E2BIG => Self::E2BIG,
            Errno::ENOEXEC => Self::ENOEXEC,
            Errno::EBADF => Self::EBADF,
            Errno::ECHILD => Self::ECHILD,
            Errno::EAGAIN => Self::EAGAIN,
            Errno::ENOMEM => Self::ENOMEM,
            Errno::EACCES => Self::EACCES,
            Errno::EFAULT => Self::EFAULT,
            Errno::EBUSY => Self::EBUSY,
            Errno::EEXIST => Self::EEXIST,
            Errno::EXDEV => Self::EXDEV,
            Errno::ENODEV => Self::ENODEV,
            Errno::ENOTDIR => Self::ENOTDIR,
            Errno::EISDIR => Self::EISDIR,
            Errno::EINVAL => Self::EINVAL,
            Errno::ENFILE => Self::ENFILE,
            Errno::EMFILE => Self::EMFILE,
            Errno::ENOTTY => Self::ENOTTY,
            Errno::ETXTBSY => Self::ETXTBSY,
            Errno::EFBIG => Self::EFBIG,
            Errno::ENOSPC => Self::ENOSPC,
            Errno::ESPIPE => Self::ESPIPE,
            Errno::EROFS => Self::EROFS,
            Errno::EMLINK => Self::EMLINK,
            Errno::EPIPE => Self::EPIPE,
            Errno::EDOM => Self::EDOM,
            Errno::ERANGE => Self::ERANGE,
            Errno::EDEADLK => Self::EDEADLK,
            Errno::ENAMETOOLONG => Self::ENAMETOOLONG,
            Errno::ENOLCK => Self::ENOLCK,
            Errno::ENOSYS => Self::ENOSYS,
            Errno::ENOTEMPTY => Self::ENOTEMPTY,
            Errno::ELOOP => Self::ELOOP,
            Errno::ENOMSG => Self::ENOMSG,
            Errno::EIDRM => Self::EIDRM,
            Errno::ENOLINK => Self::ENOLINK,
            Errno::EPROTO => Self::EPROTO,
            Errno::EMULTIHOP => Self::EMULTIHOP,
            Errno::EBADMSG => Self::EBADMSG,
            Errno::EOVERFLOW => Self::EOVERFLOW,
            Errno::EILSEQ => Self::EILSEQ,
            Errno::ENOTSOCK => Self::ENOTSOCK,
            Errno::EDESTADDRREQ => Self::EDESTADDRREQ,
            Errno::EMSGSIZE => Self::EMSGSIZE,
            Errno::EPROTOTYPE => Self::EPROTOTYPE,
            Errno::ENOPROTOOPT => Self::ENOPROTOOPT,
            Errno::EPROTONOSUPPORT => Self::EPROTONOSUPPORT,
            Errno::EAFNOSUPPORT => Self::EAFNOSUPPORT,
            Errno::EADDRINUSE => Self::EADDRINUSE,
            Errno::EADDRNOTAVAIL => Self::EADDRNOTAVAIL,
            Errno::ENETDOWN => Self::ENETDOWN,
            Errno::ENETUNREACH => Self::ENETUNREACH,
            Errno::ENETRESET => Self::ENETRESET,
            Errno::ECONNABORTED => Self::ECONNABORTED,
            Errno::ECONNRESET => Self::ECONNRESET,
            Errno::ENOBUFS => Self::ENOBUFS,
            Errno::EISCONN => Self::EISCONN,
            Errno::ENOTCONN => Self::ENOTCONN,
            Errno::ETIMEDOUT => Self::ETIMEDOUT,
            Errno::ECONNREFUSED => Self::ECONNREFUSED,
            Errno::EHOSTUNREACH => Self::EHOSTUNREACH,
            Errno::EALREADY => Self::EALREADY,
            Errno::EINPROGRESS => Self::EINPROGRESS,
            Errno::ESTALE => Self::ESTALE,
            Errno::EDQUOT => Self::EDQUOT,
            Errno::ECANCELED => Self::ECANCELED,
            Errno::EOWNERDEAD => Self::EOWNERDEAD,
            Errno::ENOTRECOVERABLE => Self::ENOTRECOVERABLE,
        }
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

pub(crate) fn filestat_from_nix(filestat: libc::stat) -> Result<wasi::__wasi_filestat_t> {
    use std::convert::TryInto;

    fn filestat_to_timestamp(secs: u64, nsecs: u64) -> Result<wasi::__wasi_timestamp_t> {
        secs.checked_mul(1_000_000_000)
            .and_then(|sec_nsec| sec_nsec.checked_add(nsecs))
            .ok_or(Error::EOVERFLOW)
    }

    let filetype = yanix::file::FileType::from_stat_st_mode(filestat.st_mode);
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
        filetype: FileType::from(filetype).to_wasi(),
    })
}

/// Creates owned WASI path from OS string.
///
/// NB WASI spec requires OS string to be valid UTF-8. Otherwise,
/// `__WASI_ERRNO_ILSEQ` error is returned.
pub(crate) fn path_from_host<S: AsRef<OsStr>>(s: S) -> Result<String> {
    helpers::path_from_slice(s.as_ref().as_bytes()).map(String::from)
}

impl From<yanix::file::FileType> for FileType {
    fn from(ft: yanix::file::FileType) -> Self {
        use yanix::file::FileType::*;
        match ft {
            RegularFile => Self::RegularFile,
            Symlink => Self::Symlink,
            Directory => Self::Directory,
            BlockDevice => Self::BlockDevice,
            CharacterDevice => Self::CharacterDevice,
            /* Unknown | Socket | Fifo */
            _ => Self::Unknown,
            // TODO how to discriminate between STREAM and DGRAM?
            // Perhaps, we should create a more general WASI filetype
            // such as __WASI_FILETYPE_SOCKET, and then it would be
            // up to the client to check whether it's actually
            // STREAM or DGRAM?
        }
    }
}

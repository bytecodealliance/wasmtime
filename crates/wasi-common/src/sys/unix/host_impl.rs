//! WASI host types specific to *nix host.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::host::FileType;
use crate::{error::FromRawOsError, helpers, sys::unix::sys_impl, wasi, Error, Result};
use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;
use yanix::file::OFlag;

pub(crate) use sys_impl::host_impl::*;

impl From<yanix::Error> for Error {
    fn from(err: yanix::Error) -> Self {
        use yanix::Error::*;
        match err {
            Io(err) => err.into(),
            Nul(err) => err.into(),
            IntConversion(err) => err.into(),
        }
    }
}

impl FromRawOsError for Error {
    fn from_raw_os_error(code: i32) -> Self {
        match code {
            libc::EPERM => Self::EPERM,
            libc::ENOENT => Self::ENOENT,
            libc::ESRCH => Self::ESRCH,
            libc::EINTR => Self::EINTR,
            libc::EIO => Self::EIO,
            libc::ENXIO => Self::ENXIO,
            libc::E2BIG => Self::E2BIG,
            libc::ENOEXEC => Self::ENOEXEC,
            libc::EBADF => Self::EBADF,
            libc::ECHILD => Self::ECHILD,
            libc::EAGAIN => Self::EAGAIN,
            libc::ENOMEM => Self::ENOMEM,
            libc::EACCES => Self::EACCES,
            libc::EFAULT => Self::EFAULT,
            libc::EBUSY => Self::EBUSY,
            libc::EEXIST => Self::EEXIST,
            libc::EXDEV => Self::EXDEV,
            libc::ENODEV => Self::ENODEV,
            libc::ENOTDIR => Self::ENOTDIR,
            libc::EISDIR => Self::EISDIR,
            libc::EINVAL => Self::EINVAL,
            libc::ENFILE => Self::ENFILE,
            libc::EMFILE => Self::EMFILE,
            libc::ENOTTY => Self::ENOTTY,
            libc::ETXTBSY => Self::ETXTBSY,
            libc::EFBIG => Self::EFBIG,
            libc::ENOSPC => Self::ENOSPC,
            libc::ESPIPE => Self::ESPIPE,
            libc::EROFS => Self::EROFS,
            libc::EMLINK => Self::EMLINK,
            libc::EPIPE => Self::EPIPE,
            libc::EDOM => Self::EDOM,
            libc::ERANGE => Self::ERANGE,
            libc::EDEADLK => Self::EDEADLK,
            libc::ENAMETOOLONG => Self::ENAMETOOLONG,
            libc::ENOLCK => Self::ENOLCK,
            libc::ENOSYS => Self::ENOSYS,
            libc::ENOTEMPTY => Self::ENOTEMPTY,
            libc::ELOOP => Self::ELOOP,
            libc::ENOMSG => Self::ENOMSG,
            libc::EIDRM => Self::EIDRM,
            libc::ENOLINK => Self::ENOLINK,
            libc::EPROTO => Self::EPROTO,
            libc::EMULTIHOP => Self::EMULTIHOP,
            libc::EBADMSG => Self::EBADMSG,
            libc::EOVERFLOW => Self::EOVERFLOW,
            libc::EILSEQ => Self::EILSEQ,
            libc::ENOTSOCK => Self::ENOTSOCK,
            libc::EDESTADDRREQ => Self::EDESTADDRREQ,
            libc::EMSGSIZE => Self::EMSGSIZE,
            libc::EPROTOTYPE => Self::EPROTOTYPE,
            libc::ENOPROTOOPT => Self::ENOPROTOOPT,
            libc::EPROTONOSUPPORT => Self::EPROTONOSUPPORT,
            libc::EAFNOSUPPORT => Self::EAFNOSUPPORT,
            libc::EADDRINUSE => Self::EADDRINUSE,
            libc::EADDRNOTAVAIL => Self::EADDRNOTAVAIL,
            libc::ENETDOWN => Self::ENETDOWN,
            libc::ENETUNREACH => Self::ENETUNREACH,
            libc::ENETRESET => Self::ENETRESET,
            libc::ECONNABORTED => Self::ECONNABORTED,
            libc::ECONNRESET => Self::ECONNRESET,
            libc::ENOBUFS => Self::ENOBUFS,
            libc::EISCONN => Self::EISCONN,
            libc::ENOTCONN => Self::ENOTCONN,
            libc::ETIMEDOUT => Self::ETIMEDOUT,
            libc::ECONNREFUSED => Self::ECONNREFUSED,
            libc::EHOSTUNREACH => Self::EHOSTUNREACH,
            libc::EALREADY => Self::EALREADY,
            libc::EINPROGRESS => Self::EINPROGRESS,
            libc::ESTALE => Self::ESTALE,
            libc::EDQUOT => Self::EDQUOT,
            libc::ECANCELED => Self::ECANCELED,
            libc::EOWNERDEAD => Self::EOWNERDEAD,
            libc::ENOTRECOVERABLE => Self::ENOTRECOVERABLE,
            x => {
                log::debug!("Unknown errno value: {}", x);
                Self::EIO
            }
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
        nlink: wasi::__wasi_linkcount_t::from(filestat.st_nlink),
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

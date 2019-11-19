// Due to https://github.com/rust-lang/rust/issues/64247
#![allow(clippy::use_self)]
use crate::old::snapshot_0::wasi;
use std::convert::Infallible;
use std::num::TryFromIntError;
use std::{fmt, str};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[repr(u16)]
pub enum WasiError {
    ESUCCESS = wasi::__WASI_ERRNO_SUCCESS,
    E2BIG = wasi::__WASI_ERRNO_2BIG,
    EACCES = wasi::__WASI_ERRNO_ACCES,
    EADDRINUSE = wasi::__WASI_ERRNO_ADDRINUSE,
    EADDRNOTAVAIL = wasi::__WASI_ERRNO_ADDRNOTAVAIL,
    EAFNOSUPPORT = wasi::__WASI_ERRNO_AFNOSUPPORT,
    EAGAIN = wasi::__WASI_ERRNO_AGAIN,
    EALREADY = wasi::__WASI_ERRNO_ALREADY,
    EBADF = wasi::__WASI_ERRNO_BADF,
    EBADMSG = wasi::__WASI_ERRNO_BADMSG,
    EBUSY = wasi::__WASI_ERRNO_BUSY,
    ECANCELED = wasi::__WASI_ERRNO_CANCELED,
    ECHILD = wasi::__WASI_ERRNO_CHILD,
    ECONNABORTED = wasi::__WASI_ERRNO_CONNABORTED,
    ECONNREFUSED = wasi::__WASI_ERRNO_CONNREFUSED,
    ECONNRESET = wasi::__WASI_ERRNO_CONNRESET,
    EDEADLK = wasi::__WASI_ERRNO_DEADLK,
    EDESTADDRREQ = wasi::__WASI_ERRNO_DESTADDRREQ,
    EDOM = wasi::__WASI_ERRNO_DOM,
    EDQUOT = wasi::__WASI_ERRNO_DQUOT,
    EEXIST = wasi::__WASI_ERRNO_EXIST,
    EFAULT = wasi::__WASI_ERRNO_FAULT,
    EFBIG = wasi::__WASI_ERRNO_FBIG,
    EHOSTUNREACH = wasi::__WASI_ERRNO_HOSTUNREACH,
    EIDRM = wasi::__WASI_ERRNO_IDRM,
    EILSEQ = wasi::__WASI_ERRNO_ILSEQ,
    EINPROGRESS = wasi::__WASI_ERRNO_INPROGRESS,
    EINTR = wasi::__WASI_ERRNO_INTR,
    EINVAL = wasi::__WASI_ERRNO_INVAL,
    EIO = wasi::__WASI_ERRNO_IO,
    EISCONN = wasi::__WASI_ERRNO_ISCONN,
    EISDIR = wasi::__WASI_ERRNO_ISDIR,
    ELOOP = wasi::__WASI_ERRNO_LOOP,
    EMFILE = wasi::__WASI_ERRNO_MFILE,
    EMLINK = wasi::__WASI_ERRNO_MLINK,
    EMSGSIZE = wasi::__WASI_ERRNO_MSGSIZE,
    EMULTIHOP = wasi::__WASI_ERRNO_MULTIHOP,
    ENAMETOOLONG = wasi::__WASI_ERRNO_NAMETOOLONG,
    ENETDOWN = wasi::__WASI_ERRNO_NETDOWN,
    ENETRESET = wasi::__WASI_ERRNO_NETRESET,
    ENETUNREACH = wasi::__WASI_ERRNO_NETUNREACH,
    ENFILE = wasi::__WASI_ERRNO_NFILE,
    ENOBUFS = wasi::__WASI_ERRNO_NOBUFS,
    ENODEV = wasi::__WASI_ERRNO_NODEV,
    ENOENT = wasi::__WASI_ERRNO_NOENT,
    ENOEXEC = wasi::__WASI_ERRNO_NOEXEC,
    ENOLCK = wasi::__WASI_ERRNO_NOLCK,
    ENOLINK = wasi::__WASI_ERRNO_NOLINK,
    ENOMEM = wasi::__WASI_ERRNO_NOMEM,
    ENOMSG = wasi::__WASI_ERRNO_NOMSG,
    ENOPROTOOPT = wasi::__WASI_ERRNO_NOPROTOOPT,
    ENOSPC = wasi::__WASI_ERRNO_NOSPC,
    ENOSYS = wasi::__WASI_ERRNO_NOSYS,
    ENOTCONN = wasi::__WASI_ERRNO_NOTCONN,
    ENOTDIR = wasi::__WASI_ERRNO_NOTDIR,
    ENOTEMPTY = wasi::__WASI_ERRNO_NOTEMPTY,
    ENOTRECOVERABLE = wasi::__WASI_ERRNO_NOTRECOVERABLE,
    ENOTSOCK = wasi::__WASI_ERRNO_NOTSOCK,
    ENOTSUP = wasi::__WASI_ERRNO_NOTSUP,
    ENOTTY = wasi::__WASI_ERRNO_NOTTY,
    ENXIO = wasi::__WASI_ERRNO_NXIO,
    EOVERFLOW = wasi::__WASI_ERRNO_OVERFLOW,
    EOWNERDEAD = wasi::__WASI_ERRNO_OWNERDEAD,
    EPERM = wasi::__WASI_ERRNO_PERM,
    EPIPE = wasi::__WASI_ERRNO_PIPE,
    EPROTO = wasi::__WASI_ERRNO_PROTO,
    EPROTONOSUPPORT = wasi::__WASI_ERRNO_PROTONOSUPPORT,
    EPROTOTYPE = wasi::__WASI_ERRNO_PROTOTYPE,
    ERANGE = wasi::__WASI_ERRNO_RANGE,
    EROFS = wasi::__WASI_ERRNO_ROFS,
    ESPIPE = wasi::__WASI_ERRNO_SPIPE,
    ESRCH = wasi::__WASI_ERRNO_SRCH,
    ESTALE = wasi::__WASI_ERRNO_STALE,
    ETIMEDOUT = wasi::__WASI_ERRNO_TIMEDOUT,
    ETXTBSY = wasi::__WASI_ERRNO_TXTBSY,
    EXDEV = wasi::__WASI_ERRNO_XDEV,
    ENOTCAPABLE = wasi::__WASI_ERRNO_NOTCAPABLE,
}

impl WasiError {
    pub fn as_raw_errno(self) -> wasi::__wasi_errno_t {
        self as wasi::__wasi_errno_t
    }
}

impl fmt::Display for WasiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            _ => write!(f, "{:?}", self),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    Wasi(WasiError),
    Io(std::io::Error),
    #[cfg(unix)]
    Nix(nix::Error),
    #[cfg(windows)]
    Win(winx::winerror::WinError),
}

impl From<WasiError> for Error {
    fn from(err: WasiError) -> Self {
        Self::Wasi(err)
    }
}

#[cfg(unix)]
impl From<nix::Error> for Error {
    fn from(err: nix::Error) -> Self {
        Self::Nix(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<TryFromIntError> for Error {
    fn from(_: TryFromIntError) -> Self {
        Self::Wasi(WasiError::EOVERFLOW)
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl From<str::Utf8Error> for Error {
    fn from(_: str::Utf8Error) -> Self {
        Self::Wasi(WasiError::EILSEQ)
    }
}

#[cfg(windows)]
impl From<winx::winerror::WinError> for Error {
    fn from(err: winx::winerror::WinError) -> Self {
        Self::Win(err)
    }
}

impl Error {
    pub(crate) fn as_wasi_errno(&self) -> wasi::__wasi_errno_t {
        match self {
            Self::Wasi(no) => no.as_raw_errno(),
            Self::Io(e) => errno_from_ioerror(e.to_owned()),
            #[cfg(unix)]
            Self::Nix(err) => err
                .as_errno()
                .map_or_else(
                    || {
                        log::debug!("Unknown nix errno: {}", err);
                        Self::ENOSYS
                    },
                    crate::old::snapshot_0::sys::host_impl::errno_from_nix,
                )
                .as_wasi_errno(),
            #[cfg(windows)]
            Self::Win(err) => crate::old::snapshot_0::sys::host_impl::errno_from_win(*err),
        }
    }
    pub const ESUCCESS: Self = Error::Wasi(WasiError::ESUCCESS);
    pub const E2BIG: Self = Error::Wasi(WasiError::E2BIG);
    pub const EACCES: Self = Error::Wasi(WasiError::EACCES);
    pub const EADDRINUSE: Self = Error::Wasi(WasiError::EADDRINUSE);
    pub const EADDRNOTAVAIL: Self = Error::Wasi(WasiError::EADDRNOTAVAIL);
    pub const EAFNOSUPPORT: Self = Error::Wasi(WasiError::EAFNOSUPPORT);
    pub const EAGAIN: Self = Error::Wasi(WasiError::EAGAIN);
    pub const EALREADY: Self = Error::Wasi(WasiError::EALREADY);
    pub const EBADF: Self = Error::Wasi(WasiError::EBADF);
    pub const EBADMSG: Self = Error::Wasi(WasiError::EBADMSG);
    pub const EBUSY: Self = Error::Wasi(WasiError::EBUSY);
    pub const ECANCELED: Self = Error::Wasi(WasiError::ECANCELED);
    pub const ECHILD: Self = Error::Wasi(WasiError::ECHILD);
    pub const ECONNABORTED: Self = Error::Wasi(WasiError::ECONNABORTED);
    pub const ECONNREFUSED: Self = Error::Wasi(WasiError::ECONNREFUSED);
    pub const ECONNRESET: Self = Error::Wasi(WasiError::ECONNRESET);
    pub const EDEADLK: Self = Error::Wasi(WasiError::EDEADLK);
    pub const EDESTADDRREQ: Self = Error::Wasi(WasiError::EDESTADDRREQ);
    pub const EDOM: Self = Error::Wasi(WasiError::EDOM);
    pub const EDQUOT: Self = Error::Wasi(WasiError::EDQUOT);
    pub const EEXIST: Self = Error::Wasi(WasiError::EEXIST);
    pub const EFAULT: Self = Error::Wasi(WasiError::EFAULT);
    pub const EFBIG: Self = Error::Wasi(WasiError::EFBIG);
    pub const EHOSTUNREACH: Self = Error::Wasi(WasiError::EHOSTUNREACH);
    pub const EIDRM: Self = Error::Wasi(WasiError::EIDRM);
    pub const EILSEQ: Self = Error::Wasi(WasiError::EILSEQ);
    pub const EINPROGRESS: Self = Error::Wasi(WasiError::EINPROGRESS);
    pub const EINTR: Self = Error::Wasi(WasiError::EINTR);
    pub const EINVAL: Self = Error::Wasi(WasiError::EINVAL);
    pub const EIO: Self = Error::Wasi(WasiError::EIO);
    pub const EISCONN: Self = Error::Wasi(WasiError::EISCONN);
    pub const EISDIR: Self = Error::Wasi(WasiError::EISDIR);
    pub const ELOOP: Self = Error::Wasi(WasiError::ELOOP);
    pub const EMFILE: Self = Error::Wasi(WasiError::EMFILE);
    pub const EMLINK: Self = Error::Wasi(WasiError::EMLINK);
    pub const EMSGSIZE: Self = Error::Wasi(WasiError::EMSGSIZE);
    pub const EMULTIHOP: Self = Error::Wasi(WasiError::EMULTIHOP);
    pub const ENAMETOOLONG: Self = Error::Wasi(WasiError::ENAMETOOLONG);
    pub const ENETDOWN: Self = Error::Wasi(WasiError::ENETDOWN);
    pub const ENETRESET: Self = Error::Wasi(WasiError::ENETRESET);
    pub const ENETUNREACH: Self = Error::Wasi(WasiError::ENETUNREACH);
    pub const ENFILE: Self = Error::Wasi(WasiError::ENFILE);
    pub const ENOBUFS: Self = Error::Wasi(WasiError::ENOBUFS);
    pub const ENODEV: Self = Error::Wasi(WasiError::ENODEV);
    pub const ENOENT: Self = Error::Wasi(WasiError::ENOENT);
    pub const ENOEXEC: Self = Error::Wasi(WasiError::ENOEXEC);
    pub const ENOLCK: Self = Error::Wasi(WasiError::ENOLCK);
    pub const ENOLINK: Self = Error::Wasi(WasiError::ENOLINK);
    pub const ENOMEM: Self = Error::Wasi(WasiError::ENOMEM);
    pub const ENOMSG: Self = Error::Wasi(WasiError::ENOMSG);
    pub const ENOPROTOOPT: Self = Error::Wasi(WasiError::ENOPROTOOPT);
    pub const ENOSPC: Self = Error::Wasi(WasiError::ENOSPC);
    pub const ENOSYS: Self = Error::Wasi(WasiError::ENOSYS);
    pub const ENOTCONN: Self = Error::Wasi(WasiError::ENOTCONN);
    pub const ENOTDIR: Self = Error::Wasi(WasiError::ENOTDIR);
    pub const ENOTEMPTY: Self = Error::Wasi(WasiError::ENOTEMPTY);
    pub const ENOTRECOVERABLE: Self = Error::Wasi(WasiError::ENOTRECOVERABLE);
    pub const ENOTSOCK: Self = Error::Wasi(WasiError::ENOTSOCK);
    pub const ENOTSUP: Self = Error::Wasi(WasiError::ENOTSUP);
    pub const ENOTTY: Self = Error::Wasi(WasiError::ENOTTY);
    pub const ENXIO: Self = Error::Wasi(WasiError::ENXIO);
    pub const EOVERFLOW: Self = Error::Wasi(WasiError::EOVERFLOW);
    pub const EOWNERDEAD: Self = Error::Wasi(WasiError::EOWNERDEAD);
    pub const EPERM: Self = Error::Wasi(WasiError::EPERM);
    pub const EPIPE: Self = Error::Wasi(WasiError::EPIPE);
    pub const EPROTO: Self = Error::Wasi(WasiError::EPROTO);
    pub const EPROTONOSUPPORT: Self = Error::Wasi(WasiError::EPROTONOSUPPORT);
    pub const EPROTOTYPE: Self = Error::Wasi(WasiError::EPROTOTYPE);
    pub const ERANGE: Self = Error::Wasi(WasiError::ERANGE);
    pub const EROFS: Self = Error::Wasi(WasiError::EROFS);
    pub const ESPIPE: Self = Error::Wasi(WasiError::ESPIPE);
    pub const ESRCH: Self = Error::Wasi(WasiError::ESRCH);
    pub const ESTALE: Self = Error::Wasi(WasiError::ESTALE);
    pub const ETIMEDOUT: Self = Error::Wasi(WasiError::ETIMEDOUT);
    pub const ETXTBSY: Self = Error::Wasi(WasiError::ETXTBSY);
    pub const EXDEV: Self = Error::Wasi(WasiError::EXDEV);
    pub const ENOTCAPABLE: Self = Error::Wasi(WasiError::ENOTCAPABLE);
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Wasi(e) => e.fmt(f),
            #[cfg(unix)]
            Self::Nix(e) => e.fmt(f),
            #[cfg(windows)]
            Self::Win(e) => e.fmt(f),
        }
    }
}

fn errno_from_ioerror(e: &std::io::Error) -> wasi::__wasi_errno_t {
    match e.raw_os_error() {
        Some(code) => crate::old::snapshot_0::sys::errno_from_host(code),
        None => {
            log::debug!("Inconvertible OS error: {}", e);
            wasi::__WASI_ERRNO_IO
        }
    }
}

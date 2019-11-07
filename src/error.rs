// Due to https://github.com/rust-lang/rust/issues/64247
#![allow(clippy::use_self)]
use crate::wasi;
use failure::Fail;
use std::convert::Infallible;
use std::fmt;
use std::num::TryFromIntError;
use std::str;

#[derive(Clone, Copy, Debug, Fail, Eq, PartialEq)]
#[repr(u16)]
pub enum WasiError {
    ESUCCESS = wasi::__WASI_ESUCCESS,
    E2BIG = wasi::__WASI_E2BIG,
    EACCES = wasi::__WASI_EACCES,
    EADDRINUSE = wasi::__WASI_EADDRINUSE,
    EADDRNOTAVAIL = wasi::__WASI_EADDRNOTAVAIL,
    EAFNOSUPPORT = wasi::__WASI_EAFNOSUPPORT,
    EAGAIN = wasi::__WASI_EAGAIN,
    EALREADY = wasi::__WASI_EALREADY,
    EBADF = wasi::__WASI_EBADF,
    EBADMSG = wasi::__WASI_EBADMSG,
    EBUSY = wasi::__WASI_EBUSY,
    ECANCELED = wasi::__WASI_ECANCELED,
    ECHILD = wasi::__WASI_ECHILD,
    ECONNABORTED = wasi::__WASI_ECONNABORTED,
    ECONNREFUSED = wasi::__WASI_ECONNREFUSED,
    ECONNRESET = wasi::__WASI_ECONNRESET,
    EDEADLK = wasi::__WASI_EDEADLK,
    EDESTADDRREQ = wasi::__WASI_EDESTADDRREQ,
    EDOM = wasi::__WASI_EDOM,
    EDQUOT = wasi::__WASI_EDQUOT,
    EEXIST = wasi::__WASI_EEXIST,
    EFAULT = wasi::__WASI_EFAULT,
    EFBIG = wasi::__WASI_EFBIG,
    EHOSTUNREACH = wasi::__WASI_EHOSTUNREACH,
    EIDRM = wasi::__WASI_EIDRM,
    EILSEQ = wasi::__WASI_EILSEQ,
    EINPROGRESS = wasi::__WASI_EINPROGRESS,
    EINTR = wasi::__WASI_EINTR,
    EINVAL = wasi::__WASI_EINVAL,
    EIO = wasi::__WASI_EIO,
    EISCONN = wasi::__WASI_EISCONN,
    EISDIR = wasi::__WASI_EISDIR,
    ELOOP = wasi::__WASI_ELOOP,
    EMFILE = wasi::__WASI_EMFILE,
    EMLINK = wasi::__WASI_EMLINK,
    EMSGSIZE = wasi::__WASI_EMSGSIZE,
    EMULTIHOP = wasi::__WASI_EMULTIHOP,
    ENAMETOOLONG = wasi::__WASI_ENAMETOOLONG,
    ENETDOWN = wasi::__WASI_ENETDOWN,
    ENETRESET = wasi::__WASI_ENETRESET,
    ENETUNREACH = wasi::__WASI_ENETUNREACH,
    ENFILE = wasi::__WASI_ENFILE,
    ENOBUFS = wasi::__WASI_ENOBUFS,
    ENODEV = wasi::__WASI_ENODEV,
    ENOENT = wasi::__WASI_ENOENT,
    ENOEXEC = wasi::__WASI_ENOEXEC,
    ENOLCK = wasi::__WASI_ENOLCK,
    ENOLINK = wasi::__WASI_ENOLINK,
    ENOMEM = wasi::__WASI_ENOMEM,
    ENOMSG = wasi::__WASI_ENOMSG,
    ENOPROTOOPT = wasi::__WASI_ENOPROTOOPT,
    ENOSPC = wasi::__WASI_ENOSPC,
    ENOSYS = wasi::__WASI_ENOSYS,
    ENOTCONN = wasi::__WASI_ENOTCONN,
    ENOTDIR = wasi::__WASI_ENOTDIR,
    ENOTEMPTY = wasi::__WASI_ENOTEMPTY,
    ENOTRECOVERABLE = wasi::__WASI_ENOTRECOVERABLE,
    ENOTSOCK = wasi::__WASI_ENOTSOCK,
    ENOTSUP = wasi::__WASI_ENOTSUP,
    ENOTTY = wasi::__WASI_ENOTTY,
    ENXIO = wasi::__WASI_ENXIO,
    EOVERFLOW = wasi::__WASI_EOVERFLOW,
    EOWNERDEAD = wasi::__WASI_EOWNERDEAD,
    EPERM = wasi::__WASI_EPERM,
    EPIPE = wasi::__WASI_EPIPE,
    EPROTO = wasi::__WASI_EPROTO,
    EPROTONOSUPPORT = wasi::__WASI_EPROTONOSUPPORT,
    EPROTOTYPE = wasi::__WASI_EPROTOTYPE,
    ERANGE = wasi::__WASI_ERANGE,
    EROFS = wasi::__WASI_EROFS,
    ESPIPE = wasi::__WASI_ESPIPE,
    ESRCH = wasi::__WASI_ESRCH,
    ESTALE = wasi::__WASI_ESTALE,
    ETIMEDOUT = wasi::__WASI_ETIMEDOUT,
    ETXTBSY = wasi::__WASI_ETXTBSY,
    EXDEV = wasi::__WASI_EXDEV,
    ENOTCAPABLE = wasi::__WASI_ENOTCAPABLE,
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

#[derive(Debug, Fail)]
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
                    crate::sys::host_impl::errno_from_nix,
                )
                .as_wasi_errno(),
            #[cfg(windows)]
            Self::Win(err) => crate::sys::host_impl::errno_from_win(*err),
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
        Some(code) => crate::sys::errno_from_host(code),
        None => {
            log::debug!("Inconvertible OS error: {}", e);
            wasi::__WASI_EIO
        }
    }
}

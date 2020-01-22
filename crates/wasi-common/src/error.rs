// Due to https://github.com/rust-lang/rust/issues/64247
#![allow(clippy::use_self)]
use crate::wasi;
use std::convert::Infallible;
use std::num::TryFromIntError;
use std::{ffi, str};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[repr(u16)]
#[error("{:?} ({})", self, wasi::strerror(*self as wasi::__wasi_errno_t))]
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

impl From<WasiError> for std::io::Error {
    fn from(err: WasiError) -> std::io::Error {
        wasi_errno_to_io_error(err.as_raw_errno())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("WASI error code: {0}")]
    Wasi(#[from] WasiError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(unix)]
    #[error("Yanix error: {0}")]
    Yanix(#[from] yanix::YanixError),
}

impl From<TryFromIntError> for Error {
    fn from(_: TryFromIntError) -> Self {
        Self::EOVERFLOW
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl From<str::Utf8Error> for Error {
    fn from(_: str::Utf8Error) -> Self {
        Self::EILSEQ
    }
}

impl From<ffi::NulError> for Error {
    fn from(_: ffi::NulError) -> Self {
        Self::EILSEQ
    }
}

impl From<&ffi::NulError> for Error {
    fn from(_: &ffi::NulError) -> Self {
        Self::EILSEQ
    }
}

impl Error {
    pub(crate) fn as_wasi_error(&self) -> WasiError {
        match self {
            Self::Wasi(err) => *err,
            Self::Io(err) => {
                let err = match err.raw_os_error() {
                    Some(code) => Self::from_raw_os_error(code),
                    None => {
                        log::debug!("Inconvertible OS error: {}", err);
                        Self::EIO
                    }
                };
                err.as_wasi_error()
            }
            #[cfg(unix)]
            Self::Yanix(err) => {
                use yanix::YanixError::*;
                let err: Self = match err {
                    Errno(errno) => (*errno).into(),
                    NulError(err) => err.into(),
                    TryFromIntError(err) => (*err).into(),
                };
                err.as_wasi_error()
            }
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

impl From<Error> for std::io::Error {
    fn from(err: Error) -> std::io::Error {
        match err {
            Error::Wasi(e) => e.into(),
            Error::Io(e) => e,
            #[cfg(unix)]
            Error::Yanix(e) => std::io::Error::new(std::io::ErrorKind::Other, e),
        }
    }
}

pub(crate) trait FromRawOsError {
    fn from_raw_os_error(code: i32) -> Self;
}

/// Translate a WASI errno code into an `io::Result<()>`.
///
/// TODO: Would it be better to have our own version of `io::Error` (and
/// `io::Result`), rather than trying to shoehorn WASI errors into the
/// libstd version?
fn wasi_errno_to_io_error(errno: wasi::__wasi_errno_t) -> std::io::Error {
    #[cfg(unix)]
    let raw_os_error = match errno {
        wasi::__WASI_ERRNO_SUCCESS => 0,
        wasi::__WASI_ERRNO_IO => libc::EIO,
        wasi::__WASI_ERRNO_PERM => libc::EPERM,
        wasi::__WASI_ERRNO_INVAL => libc::EINVAL,
        wasi::__WASI_ERRNO_PIPE => libc::EPIPE,
        wasi::__WASI_ERRNO_NOTCONN => libc::ENOTCONN,
        wasi::__WASI_ERRNO_2BIG => libc::E2BIG,
        wasi::__WASI_ERRNO_ACCES => libc::EACCES,
        wasi::__WASI_ERRNO_ADDRINUSE => libc::EADDRINUSE,
        wasi::__WASI_ERRNO_ADDRNOTAVAIL => libc::EADDRNOTAVAIL,
        wasi::__WASI_ERRNO_AFNOSUPPORT => libc::EAFNOSUPPORT,
        wasi::__WASI_ERRNO_AGAIN => libc::EAGAIN,
        wasi::__WASI_ERRNO_ALREADY => libc::EALREADY,
        wasi::__WASI_ERRNO_BADF => libc::EBADF,
        wasi::__WASI_ERRNO_BADMSG => libc::EBADMSG,
        wasi::__WASI_ERRNO_BUSY => libc::EBUSY,
        wasi::__WASI_ERRNO_CANCELED => libc::ECANCELED,
        wasi::__WASI_ERRNO_CHILD => libc::ECHILD,
        wasi::__WASI_ERRNO_CONNABORTED => libc::ECONNABORTED,
        wasi::__WASI_ERRNO_CONNREFUSED => libc::ECONNREFUSED,
        wasi::__WASI_ERRNO_CONNRESET => libc::ECONNRESET,
        wasi::__WASI_ERRNO_DEADLK => libc::EDEADLK,
        wasi::__WASI_ERRNO_DESTADDRREQ => libc::EDESTADDRREQ,
        wasi::__WASI_ERRNO_DOM => libc::EDOM,
        wasi::__WASI_ERRNO_DQUOT => libc::EDQUOT,
        wasi::__WASI_ERRNO_EXIST => libc::EEXIST,
        wasi::__WASI_ERRNO_FAULT => libc::EFAULT,
        wasi::__WASI_ERRNO_FBIG => libc::EFBIG,
        wasi::__WASI_ERRNO_HOSTUNREACH => libc::EHOSTUNREACH,
        wasi::__WASI_ERRNO_IDRM => libc::EIDRM,
        wasi::__WASI_ERRNO_ILSEQ => libc::EILSEQ,
        wasi::__WASI_ERRNO_INPROGRESS => libc::EINPROGRESS,
        wasi::__WASI_ERRNO_INTR => libc::EINTR,
        wasi::__WASI_ERRNO_ISCONN => libc::EISCONN,
        wasi::__WASI_ERRNO_ISDIR => libc::EISDIR,
        wasi::__WASI_ERRNO_LOOP => libc::ELOOP,
        wasi::__WASI_ERRNO_MFILE => libc::EMFILE,
        wasi::__WASI_ERRNO_MLINK => libc::EMLINK,
        wasi::__WASI_ERRNO_MSGSIZE => libc::EMSGSIZE,
        wasi::__WASI_ERRNO_MULTIHOP => libc::EMULTIHOP,
        wasi::__WASI_ERRNO_NAMETOOLONG => libc::ENAMETOOLONG,
        wasi::__WASI_ERRNO_NETDOWN => libc::ENETDOWN,
        wasi::__WASI_ERRNO_NETRESET => libc::ENETRESET,
        wasi::__WASI_ERRNO_NETUNREACH => libc::ENETUNREACH,
        wasi::__WASI_ERRNO_NFILE => libc::ENFILE,
        wasi::__WASI_ERRNO_NOBUFS => libc::ENOBUFS,
        wasi::__WASI_ERRNO_NODEV => libc::ENODEV,
        wasi::__WASI_ERRNO_NOENT => libc::ENOENT,
        wasi::__WASI_ERRNO_NOEXEC => libc::ENOEXEC,
        wasi::__WASI_ERRNO_NOLCK => libc::ENOLCK,
        wasi::__WASI_ERRNO_NOLINK => libc::ENOLINK,
        wasi::__WASI_ERRNO_NOMEM => libc::ENOMEM,
        wasi::__WASI_ERRNO_NOMSG => libc::ENOMSG,
        wasi::__WASI_ERRNO_NOPROTOOPT => libc::ENOPROTOOPT,
        wasi::__WASI_ERRNO_NOSPC => libc::ENOSPC,
        wasi::__WASI_ERRNO_NOSYS => libc::ENOSYS,
        wasi::__WASI_ERRNO_NOTDIR => libc::ENOTDIR,
        wasi::__WASI_ERRNO_NOTEMPTY => libc::ENOTEMPTY,
        wasi::__WASI_ERRNO_NOTRECOVERABLE => libc::ENOTRECOVERABLE,
        wasi::__WASI_ERRNO_NOTSOCK => libc::ENOTSOCK,
        wasi::__WASI_ERRNO_NOTSUP => libc::ENOTSUP,
        wasi::__WASI_ERRNO_NOTTY => libc::ENOTTY,
        wasi::__WASI_ERRNO_NXIO => libc::ENXIO,
        wasi::__WASI_ERRNO_OVERFLOW => libc::EOVERFLOW,
        wasi::__WASI_ERRNO_OWNERDEAD => libc::EOWNERDEAD,
        wasi::__WASI_ERRNO_PROTO => libc::EPROTO,
        wasi::__WASI_ERRNO_PROTONOSUPPORT => libc::EPROTONOSUPPORT,
        wasi::__WASI_ERRNO_PROTOTYPE => libc::EPROTOTYPE,
        wasi::__WASI_ERRNO_RANGE => libc::ERANGE,
        wasi::__WASI_ERRNO_ROFS => libc::EROFS,
        wasi::__WASI_ERRNO_SPIPE => libc::ESPIPE,
        wasi::__WASI_ERRNO_SRCH => libc::ESRCH,
        wasi::__WASI_ERRNO_STALE => libc::ESTALE,
        wasi::__WASI_ERRNO_TIMEDOUT => libc::ETIMEDOUT,
        wasi::__WASI_ERRNO_TXTBSY => libc::ETXTBSY,
        wasi::__WASI_ERRNO_XDEV => libc::EXDEV,
        #[cfg(target_os = "wasi")]
        wasi::__WASI_ERRNO_NOTCAPABLE => libc::ENOTCAPABLE,
        #[cfg(not(target_os = "wasi"))]
        wasi::__WASI_ERRNO_NOTCAPABLE => libc::EIO,
        _ => panic!("unexpected wasi errno value"),
    };

    #[cfg(windows)]
    use winapi::shared::winerror::*;

    #[cfg(windows)]
    let raw_os_error = match errno {
        wasi::__WASI_ERRNO_SUCCESS => 0,
        wasi::__WASI_ERRNO_INVAL => WSAEINVAL,
        wasi::__WASI_ERRNO_PIPE => ERROR_BROKEN_PIPE,
        wasi::__WASI_ERRNO_NOTCONN => WSAENOTCONN,
        wasi::__WASI_ERRNO_PERM | wasi::__WASI_ERRNO_ACCES => ERROR_ACCESS_DENIED,
        wasi::__WASI_ERRNO_ADDRINUSE => WSAEADDRINUSE,
        wasi::__WASI_ERRNO_ADDRNOTAVAIL => WSAEADDRNOTAVAIL,
        wasi::__WASI_ERRNO_AGAIN => WSAEWOULDBLOCK,
        wasi::__WASI_ERRNO_CONNABORTED => WSAECONNABORTED,
        wasi::__WASI_ERRNO_CONNREFUSED => WSAECONNREFUSED,
        wasi::__WASI_ERRNO_CONNRESET => WSAECONNRESET,
        wasi::__WASI_ERRNO_EXIST => ERROR_ALREADY_EXISTS,
        wasi::__WASI_ERRNO_NOENT => ERROR_FILE_NOT_FOUND,
        wasi::__WASI_ERRNO_TIMEDOUT => WSAETIMEDOUT,
        wasi::__WASI_ERRNO_AFNOSUPPORT => WSAEAFNOSUPPORT,
        wasi::__WASI_ERRNO_ALREADY => WSAEALREADY,
        wasi::__WASI_ERRNO_BADF => WSAEBADF,
        wasi::__WASI_ERRNO_DESTADDRREQ => WSAEDESTADDRREQ,
        wasi::__WASI_ERRNO_DQUOT => WSAEDQUOT,
        wasi::__WASI_ERRNO_FAULT => WSAEFAULT,
        wasi::__WASI_ERRNO_HOSTUNREACH => WSAEHOSTUNREACH,
        wasi::__WASI_ERRNO_INPROGRESS => WSAEINPROGRESS,
        wasi::__WASI_ERRNO_INTR => WSAEINTR,
        wasi::__WASI_ERRNO_ISCONN => WSAEISCONN,
        wasi::__WASI_ERRNO_LOOP => WSAELOOP,
        wasi::__WASI_ERRNO_MFILE => WSAEMFILE,
        wasi::__WASI_ERRNO_MSGSIZE => WSAEMSGSIZE,
        wasi::__WASI_ERRNO_NAMETOOLONG => WSAENAMETOOLONG,
        wasi::__WASI_ERRNO_NETDOWN => WSAENETDOWN,
        wasi::__WASI_ERRNO_NETRESET => WSAENETRESET,
        wasi::__WASI_ERRNO_NETUNREACH => WSAENETUNREACH,
        wasi::__WASI_ERRNO_NOBUFS => WSAENOBUFS,
        wasi::__WASI_ERRNO_NOPROTOOPT => WSAENOPROTOOPT,
        wasi::__WASI_ERRNO_NOTEMPTY => WSAENOTEMPTY,
        wasi::__WASI_ERRNO_NOTSOCK => WSAENOTSOCK,
        wasi::__WASI_ERRNO_PROTONOSUPPORT => WSAEPROTONOSUPPORT,
        wasi::__WASI_ERRNO_PROTOTYPE => WSAEPROTOTYPE,
        wasi::__WASI_ERRNO_STALE => WSAESTALE,
        wasi::__WASI_ERRNO_IO
        | wasi::__WASI_ERRNO_ISDIR
        | wasi::__WASI_ERRNO_2BIG
        | wasi::__WASI_ERRNO_BADMSG
        | wasi::__WASI_ERRNO_BUSY
        | wasi::__WASI_ERRNO_CANCELED
        | wasi::__WASI_ERRNO_CHILD
        | wasi::__WASI_ERRNO_DEADLK
        | wasi::__WASI_ERRNO_DOM
        | wasi::__WASI_ERRNO_FBIG
        | wasi::__WASI_ERRNO_IDRM
        | wasi::__WASI_ERRNO_ILSEQ
        | wasi::__WASI_ERRNO_MLINK
        | wasi::__WASI_ERRNO_MULTIHOP
        | wasi::__WASI_ERRNO_NFILE
        | wasi::__WASI_ERRNO_NODEV
        | wasi::__WASI_ERRNO_NOEXEC
        | wasi::__WASI_ERRNO_NOLCK
        | wasi::__WASI_ERRNO_NOLINK
        | wasi::__WASI_ERRNO_NOMEM
        | wasi::__WASI_ERRNO_NOMSG
        | wasi::__WASI_ERRNO_NOSPC
        | wasi::__WASI_ERRNO_NOSYS
        | wasi::__WASI_ERRNO_NOTDIR
        | wasi::__WASI_ERRNO_NOTRECOVERABLE
        | wasi::__WASI_ERRNO_NOTSUP
        | wasi::__WASI_ERRNO_NOTTY
        | wasi::__WASI_ERRNO_NXIO
        | wasi::__WASI_ERRNO_OVERFLOW
        | wasi::__WASI_ERRNO_OWNERDEAD
        | wasi::__WASI_ERRNO_PROTO
        | wasi::__WASI_ERRNO_RANGE
        | wasi::__WASI_ERRNO_ROFS
        | wasi::__WASI_ERRNO_SPIPE
        | wasi::__WASI_ERRNO_SRCH
        | wasi::__WASI_ERRNO_TXTBSY
        | wasi::__WASI_ERRNO_XDEV
        | wasi::__WASI_ERRNO_NOTCAPABLE => {
            return std::io::Error::new(
                std::io::ErrorKind::Other,
                error_str(errno),
            )
        }
        _ => panic!("unrecognized WASI errno value"),
    } as i32;

    std::io::Error::from_raw_os_error(raw_os_error)
}

#[cfg(windows)]
fn error_str(errno: wasi::__wasi_errno_t) -> &'static str {
    match errno {
        wasi::__WASI_ERRNO_2BIG => "Argument list too long",
        wasi::__WASI_ERRNO_ACCES => "Permission denied",
        wasi::__WASI_ERRNO_ADDRINUSE => "Address in use",
        wasi::__WASI_ERRNO_ADDRNOTAVAIL => "Address not available",
        wasi::__WASI_ERRNO_AFNOSUPPORT => "Address family not supported by protocol",
        wasi::__WASI_ERRNO_AGAIN => "Resource temporarily unavailable",
        wasi::__WASI_ERRNO_ALREADY => "Operation already in progress",
        wasi::__WASI_ERRNO_BADF => "Bad file descriptor",
        wasi::__WASI_ERRNO_BADMSG => "Bad message",
        wasi::__WASI_ERRNO_BUSY => "Resource busy",
        wasi::__WASI_ERRNO_CANCELED => "Operation canceled",
        wasi::__WASI_ERRNO_CHILD => "No child process",
        wasi::__WASI_ERRNO_CONNABORTED => "Connection aborted",
        wasi::__WASI_ERRNO_CONNREFUSED => "Connection refused",
        wasi::__WASI_ERRNO_CONNRESET => "Connection reset by peer",
        wasi::__WASI_ERRNO_DEADLK => "Resource deadlock would occur",
        wasi::__WASI_ERRNO_DESTADDRREQ => "Destination address required",
        wasi::__WASI_ERRNO_DOM => "Domain error",
        wasi::__WASI_ERRNO_DQUOT => "Quota exceeded",
        wasi::__WASI_ERRNO_EXIST => "File exists",
        wasi::__WASI_ERRNO_FAULT => "Bad address",
        wasi::__WASI_ERRNO_FBIG => "File too large",
        wasi::__WASI_ERRNO_HOSTUNREACH => "Host is unreachable",
        wasi::__WASI_ERRNO_IDRM => "Identifier removed",
        wasi::__WASI_ERRNO_ILSEQ => "Illegal byte sequence",
        wasi::__WASI_ERRNO_INPROGRESS => "Operation in progress",
        wasi::__WASI_ERRNO_INTR => "Interrupted system call",
        wasi::__WASI_ERRNO_INVAL => "Invalid argument",
        wasi::__WASI_ERRNO_IO => "Remote I/O error",
        wasi::__WASI_ERRNO_ISCONN => "Socket is connected",
        wasi::__WASI_ERRNO_ISDIR => "Is a directory",
        wasi::__WASI_ERRNO_LOOP => "Symbolic link loop",
        wasi::__WASI_ERRNO_MFILE => "No file descriptors available",
        wasi::__WASI_ERRNO_MLINK => "Too many links",
        wasi::__WASI_ERRNO_MSGSIZE => "Message too large",
        wasi::__WASI_ERRNO_MULTIHOP => "Multihop attempted",
        wasi::__WASI_ERRNO_NAMETOOLONG => "Filename too long",
        wasi::__WASI_ERRNO_NETDOWN => "Network is down",
        wasi::__WASI_ERRNO_NETRESET => "Connection reset by network",
        wasi::__WASI_ERRNO_NETUNREACH => "Network unreachable",
        wasi::__WASI_ERRNO_NFILE => "Too many open files in system",
        wasi::__WASI_ERRNO_NOBUFS => "No buffer space available",
        wasi::__WASI_ERRNO_NODEV => "No such device",
        wasi::__WASI_ERRNO_NOENT => "No such file or directory",
        wasi::__WASI_ERRNO_NOEXEC => "Exec format error",
        wasi::__WASI_ERRNO_NOLCK => "No locks available",
        wasi::__WASI_ERRNO_NOLINK => "Link has been severed",
        wasi::__WASI_ERRNO_NOMEM => "Out of memory",
        wasi::__WASI_ERRNO_NOMSG => "No message of desired type",
        wasi::__WASI_ERRNO_NOPROTOOPT => "Protocol not available",
        wasi::__WASI_ERRNO_NOSPC => "No space left on device",
        wasi::__WASI_ERRNO_NOSYS => "Function not implemented",
        wasi::__WASI_ERRNO_NOTCONN => "Socket not connected",
        wasi::__WASI_ERRNO_NOTDIR => "Not a directory",
        wasi::__WASI_ERRNO_NOTEMPTY => "Directory not empty",
        wasi::__WASI_ERRNO_NOTRECOVERABLE => "State not recoverable",
        wasi::__WASI_ERRNO_NOTSOCK => "Not a socket",
        wasi::__WASI_ERRNO_NOTSUP => "Not supported",
        wasi::__WASI_ERRNO_NOTTY => "Not a tty",
        wasi::__WASI_ERRNO_NXIO => "No such device or address",
        wasi::__WASI_ERRNO_OVERFLOW => "Value too large for data type",
        wasi::__WASI_ERRNO_OWNERDEAD => "Previous owner died",
        wasi::__WASI_ERRNO_PERM => "Operation not permitted",
        wasi::__WASI_ERRNO_PIPE => "Broken pipe",
        wasi::__WASI_ERRNO_PROTO => "Protocol error",
        wasi::__WASI_ERRNO_PROTONOSUPPORT => "Protocol not supported",
        wasi::__WASI_ERRNO_PROTOTYPE => "Protocol wrong type for socket",
        wasi::__WASI_ERRNO_RANGE => "Result not representable",
        wasi::__WASI_ERRNO_ROFS => "Read-only file system",
        wasi::__WASI_ERRNO_SPIPE => "Invalid seek",
        wasi::__WASI_ERRNO_SRCH => "No such process",
        wasi::__WASI_ERRNO_STALE => "Stale file handle",
        wasi::__WASI_ERRNO_TIMEDOUT => "Operation timed out",
        wasi::__WASI_ERRNO_TXTBSY => "Text file busy",
        wasi::__WASI_ERRNO_XDEV => "Cross-device link",
        wasi::__WASI_ERRNO_NOTCAPABLE => "Capabilities insufficient",
        _ => panic!("unrecognized WASI errno value"),
    }
}

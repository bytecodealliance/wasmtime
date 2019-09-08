use crate::host;
use crate::sys::errno_from_ioerror;
use std::num::TryFromIntError;

#[derive(Clone, Copy, Debug)]
pub enum WasiError {
    ESUCCESS = 0,
    E2BIG = 1,
    EACCES = 2,
    EADDRINUSE = 3,
    EADDRNOTAVAIL = 4,
    EAFNOSUPPORT = 5,
    EAGAIN = 6,
    EALREADY = 7,
    EBADF = 8,
    EBADMSG = 9,
    EBUSY = 10,
    ECANCELED = 11,
    ECHILD = 12,
    ECONNABORTED = 13,
    ECONNREFUSED = 14,
    ECONNRESET = 15,
    EDEADLK = 16,
    EDESTADDRREQ = 17,
    EDOM = 18,
    EDQUOT = 19,
    EEXIST = 20,
    EFAULT = 21,
    EFBIG = 22,
    EHOSTUNREACH = 23,
    EIDRM = 24,
    EILSEQ = 25,
    EINPROGRESS = 26,
    EINTR = 27,
    EINVAL = 28,
    EIO = 29,
    EISCONN = 30,
    EISDIR = 31,
    ELOOP = 32,
    EMFILE = 33,
    EMLINK = 34,
    EMSGSIZE = 35,
    EMULTIHOP = 36,
    ENAMETOOLONG = 37,
    ENETDOWN = 38,
    ENETRESET = 39,
    ENETUNREACH = 40,
    ENFILE = 41,
    ENOBUFS = 42,
    ENODEV = 43,
    ENOENT = 44,
    ENOEXEC = 45,
    ENOLCK = 46,
    ENOLINK = 47,
    ENOMEM = 48,
    ENOMSG = 49,
    ENOPROTOOPT = 50,
    ENOSPC = 51,
    ENOSYS = 52,
    ENOTCONN = 53,
    ENOTDIR = 54,
    ENOTEMPTY = 55,
    ENOTRECOVERABLE = 56,
    ENOTSOCK = 57,
    ENOTSUP = 58,
    ENOTTY = 59,
    ENXIO = 60,
    EOVERFLOW = 61,
    EOWNERDEAD = 62,
    EPERM = 63,
    EPIPE = 64,
    EPROTO = 65,
    EPROTONOSUPPORT = 66,
    EPROTOTYPE = 67,
    ERANGE = 68,
    EROFS = 69,
    ESPIPE = 70,
    ESRCH = 71,
    ESTALE = 72,
    ETIMEDOUT = 73,
    ETXTBSY = 74,
    EXDEV = 75,
    ENOTCAPABLE = 76,
}

impl WasiError {
    pub fn as_raw_errno(&self) -> host::__wasi_errno_t {
        *self as host::__wasi_errno_t
    }
}

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

#[cfg(windows)]
impl From<winx::winerror::WinError> for Error {
    fn from(err: winx::winerror::WinError) -> Self {
        Self::Win(err)
    }
}

impl Error {
    pub(crate) fn as_wasi_errno(&self) -> host::__wasi_errno_t {
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

//! Errno-specific for different Unix platforms
use crate::Result;
use std::{fmt, io};
use thiserror::Error;

#[derive(Debug, Copy, Clone, Error, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Errno {
    EPERM = libc::EPERM,
    ENOENT = libc::ENOENT,
    ESRCH = libc::ESRCH,
    EINTR = libc::EINTR,
    EIO = libc::EIO,
    ENXIO = libc::ENXIO,
    E2BIG = libc::E2BIG,
    ENOEXEC = libc::ENOEXEC,
    EBADF = libc::EBADF,
    ECHILD = libc::ECHILD,
    EAGAIN = libc::EAGAIN,
    ENOMEM = libc::ENOMEM,
    EACCES = libc::EACCES,
    EFAULT = libc::EFAULT,
    EBUSY = libc::EBUSY,
    EEXIST = libc::EEXIST,
    EXDEV = libc::EXDEV,
    ENODEV = libc::ENODEV,
    ENOTDIR = libc::ENOTDIR,
    EISDIR = libc::EISDIR,
    EINVAL = libc::EINVAL,
    ENFILE = libc::ENFILE,
    EMFILE = libc::EMFILE,
    ENOTTY = libc::ENOTTY,
    ETXTBSY = libc::ETXTBSY,
    EFBIG = libc::EFBIG,
    ENOSPC = libc::ENOSPC,
    ESPIPE = libc::ESPIPE,
    EROFS = libc::EROFS,
    EMLINK = libc::EMLINK,
    EPIPE = libc::EPIPE,
    EDOM = libc::EDOM,
    ERANGE = libc::ERANGE,
    EDEADLK = libc::EDEADLK,
    ENAMETOOLONG = libc::ENAMETOOLONG,
    ENOLCK = libc::ENOLCK,
    ENOSYS = libc::ENOSYS,
    ENOTEMPTY = libc::ENOTEMPTY,
    ELOOP = libc::ELOOP,
    ENOMSG = libc::ENOMSG,
    EIDRM = libc::EIDRM,
    ENOLINK = libc::ENOLINK,
    EPROTO = libc::EPROTO,
    EMULTIHOP = libc::EMULTIHOP,
    EBADMSG = libc::EBADMSG,
    EOVERFLOW = libc::EOVERFLOW,
    EILSEQ = libc::EILSEQ,
    ENOTSOCK = libc::ENOTSOCK,
    EDESTADDRREQ = libc::EDESTADDRREQ,
    EMSGSIZE = libc::EMSGSIZE,
    EPROTOTYPE = libc::EPROTOTYPE,
    ENOPROTOOPT = libc::ENOPROTOOPT,
    EPROTONOSUPPORT = libc::EPROTONOSUPPORT,
    EAFNOSUPPORT = libc::EAFNOSUPPORT,
    EADDRINUSE = libc::EADDRINUSE,
    EADDRNOTAVAIL = libc::EADDRNOTAVAIL,
    ENETDOWN = libc::ENETDOWN,
    ENETUNREACH = libc::ENETUNREACH,
    ENETRESET = libc::ENETRESET,
    ECONNABORTED = libc::ECONNABORTED,
    ECONNRESET = libc::ECONNRESET,
    ENOBUFS = libc::ENOBUFS,
    EISCONN = libc::EISCONN,
    ENOTCONN = libc::ENOTCONN,
    ETIMEDOUT = libc::ETIMEDOUT,
    ECONNREFUSED = libc::ECONNREFUSED,
    EHOSTUNREACH = libc::EHOSTUNREACH,
    EALREADY = libc::EALREADY,
    EINPROGRESS = libc::EINPROGRESS,
    ESTALE = libc::ESTALE,
    EDQUOT = libc::EDQUOT,
    ECANCELED = libc::ECANCELED,
    EOWNERDEAD = libc::EOWNERDEAD,
    ENOTRECOVERABLE = libc::ENOTRECOVERABLE,
}

impl Errno {
    pub fn from_i32(err: i32) -> Self {
        match err {
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
            other => {
                log::warn!("Unknown errno: {}", other);
                Self::ENOSYS
            }
        }
    }

    pub fn last() -> Self {
        let errno = io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::ENOSYS);
        Self::from_i32(errno)
    }

    pub fn from_success_code<T: IsZero>(t: T) -> Result<()> {
        if t.is_zero() {
            Ok(())
        } else {
            Err(Self::last().into())
        }
    }

    pub fn from_result<T: IsMinusOne>(t: T) -> Result<T> {
        if t.is_minus_one() {
            Err(Self::last().into())
        } else {
            Ok(t)
        }
    }
}

impl fmt::Display for Errno {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Errno code: {}", self)
    }
}

#[doc(hidden)]
pub trait IsZero {
    fn is_zero(&self) -> bool;
}

macro_rules! impl_is_zero {
    ($($t:ident)*) => ($(impl IsZero for $t {
        fn is_zero(&self) -> bool {
            *self == 0
        }
    })*)
}

impl_is_zero! { i32 i64 isize }

#[doc(hidden)]
pub trait IsMinusOne {
    fn is_minus_one(&self) -> bool;
}

macro_rules! impl_is_minus_one {
    ($($t:ident)*) => ($(impl IsMinusOne for $t {
        fn is_minus_one(&self) -> bool {
            *self == -1
        }
    })*)
}

impl_is_minus_one! { i32 i64 isize }

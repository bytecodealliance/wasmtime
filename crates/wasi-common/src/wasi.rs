//! Types and constants shared between 32-bit and 64-bit wasi. Types involving
//! pointer or `usize`-sized data are excluded here, so this file only contains
//! fixed-size types, so it's host/target independent.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use wig::witx_wasi_types;

witx_wasi_types!("snapshot" "wasi_snapshot_preview1");

pub type WasiResult<T> = Result<T, WasiError>;

#[derive(Clone, Copy, Debug, thiserror::Error, Eq, PartialEq)]
#[repr(u16)]
#[error("{:?} ({})", self, strerror(*self as __wasi_errno_t))]
pub enum WasiError {
    ESUCCESS = __WASI_ERRNO_SUCCESS,
    E2BIG = __WASI_ERRNO_2BIG,
    EACCES = __WASI_ERRNO_ACCES,
    EADDRINUSE = __WASI_ERRNO_ADDRINUSE,
    EADDRNOTAVAIL = __WASI_ERRNO_ADDRNOTAVAIL,
    EAFNOSUPPORT = __WASI_ERRNO_AFNOSUPPORT,
    EAGAIN = __WASI_ERRNO_AGAIN,
    EALREADY = __WASI_ERRNO_ALREADY,
    EBADF = __WASI_ERRNO_BADF,
    EBADMSG = __WASI_ERRNO_BADMSG,
    EBUSY = __WASI_ERRNO_BUSY,
    ECANCELED = __WASI_ERRNO_CANCELED,
    ECHILD = __WASI_ERRNO_CHILD,
    ECONNABORTED = __WASI_ERRNO_CONNABORTED,
    ECONNREFUSED = __WASI_ERRNO_CONNREFUSED,
    ECONNRESET = __WASI_ERRNO_CONNRESET,
    EDEADLK = __WASI_ERRNO_DEADLK,
    EDESTADDRREQ = __WASI_ERRNO_DESTADDRREQ,
    EDOM = __WASI_ERRNO_DOM,
    EDQUOT = __WASI_ERRNO_DQUOT,
    EEXIST = __WASI_ERRNO_EXIST,
    EFAULT = __WASI_ERRNO_FAULT,
    EFBIG = __WASI_ERRNO_FBIG,
    EHOSTUNREACH = __WASI_ERRNO_HOSTUNREACH,
    EIDRM = __WASI_ERRNO_IDRM,
    EILSEQ = __WASI_ERRNO_ILSEQ,
    EINPROGRESS = __WASI_ERRNO_INPROGRESS,
    EINTR = __WASI_ERRNO_INTR,
    EINVAL = __WASI_ERRNO_INVAL,
    EIO = __WASI_ERRNO_IO,
    EISCONN = __WASI_ERRNO_ISCONN,
    EISDIR = __WASI_ERRNO_ISDIR,
    ELOOP = __WASI_ERRNO_LOOP,
    EMFILE = __WASI_ERRNO_MFILE,
    EMLINK = __WASI_ERRNO_MLINK,
    EMSGSIZE = __WASI_ERRNO_MSGSIZE,
    EMULTIHOP = __WASI_ERRNO_MULTIHOP,
    ENAMETOOLONG = __WASI_ERRNO_NAMETOOLONG,
    ENETDOWN = __WASI_ERRNO_NETDOWN,
    ENETRESET = __WASI_ERRNO_NETRESET,
    ENETUNREACH = __WASI_ERRNO_NETUNREACH,
    ENFILE = __WASI_ERRNO_NFILE,
    ENOBUFS = __WASI_ERRNO_NOBUFS,
    ENODEV = __WASI_ERRNO_NODEV,
    ENOENT = __WASI_ERRNO_NOENT,
    ENOEXEC = __WASI_ERRNO_NOEXEC,
    ENOLCK = __WASI_ERRNO_NOLCK,
    ENOLINK = __WASI_ERRNO_NOLINK,
    ENOMEM = __WASI_ERRNO_NOMEM,
    ENOMSG = __WASI_ERRNO_NOMSG,
    ENOPROTOOPT = __WASI_ERRNO_NOPROTOOPT,
    ENOSPC = __WASI_ERRNO_NOSPC,
    ENOSYS = __WASI_ERRNO_NOSYS,
    ENOTCONN = __WASI_ERRNO_NOTCONN,
    ENOTDIR = __WASI_ERRNO_NOTDIR,
    ENOTEMPTY = __WASI_ERRNO_NOTEMPTY,
    ENOTRECOVERABLE = __WASI_ERRNO_NOTRECOVERABLE,
    ENOTSOCK = __WASI_ERRNO_NOTSOCK,
    ENOTSUP = __WASI_ERRNO_NOTSUP,
    ENOTTY = __WASI_ERRNO_NOTTY,
    ENXIO = __WASI_ERRNO_NXIO,
    EOVERFLOW = __WASI_ERRNO_OVERFLOW,
    EOWNERDEAD = __WASI_ERRNO_OWNERDEAD,
    EPERM = __WASI_ERRNO_PERM,
    EPIPE = __WASI_ERRNO_PIPE,
    EPROTO = __WASI_ERRNO_PROTO,
    EPROTONOSUPPORT = __WASI_ERRNO_PROTONOSUPPORT,
    EPROTOTYPE = __WASI_ERRNO_PROTOTYPE,
    ERANGE = __WASI_ERRNO_RANGE,
    EROFS = __WASI_ERRNO_ROFS,
    ESPIPE = __WASI_ERRNO_SPIPE,
    ESRCH = __WASI_ERRNO_SRCH,
    ESTALE = __WASI_ERRNO_STALE,
    ETIMEDOUT = __WASI_ERRNO_TIMEDOUT,
    ETXTBSY = __WASI_ERRNO_TXTBSY,
    EXDEV = __WASI_ERRNO_XDEV,
    ENOTCAPABLE = __WASI_ERRNO_NOTCAPABLE,
}

impl WasiError {
    pub fn as_raw_errno(self) -> __wasi_errno_t {
        self as __wasi_errno_t
    }
}

impl From<std::convert::Infallible> for WasiError {
    fn from(_err: std::convert::Infallible) -> Self {
        unreachable!()
    }
}

impl From<std::num::TryFromIntError> for WasiError {
    fn from(_err: std::num::TryFromIntError) -> Self {
        Self::EOVERFLOW
    }
}

impl From<std::str::Utf8Error> for WasiError {
    fn from(_err: std::str::Utf8Error) -> Self {
        Self::EILSEQ
    }
}

pub(crate) const RIGHTS_ALL: __wasi_rights_t = __WASI_RIGHTS_FD_DATASYNC
    | __WASI_RIGHTS_FD_READ
    | __WASI_RIGHTS_FD_SEEK
    | __WASI_RIGHTS_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHTS_FD_SYNC
    | __WASI_RIGHTS_FD_TELL
    | __WASI_RIGHTS_FD_WRITE
    | __WASI_RIGHTS_FD_ADVISE
    | __WASI_RIGHTS_FD_ALLOCATE
    | __WASI_RIGHTS_PATH_CREATE_DIRECTORY
    | __WASI_RIGHTS_PATH_CREATE_FILE
    | __WASI_RIGHTS_PATH_LINK_SOURCE
    | __WASI_RIGHTS_PATH_LINK_TARGET
    | __WASI_RIGHTS_PATH_OPEN
    | __WASI_RIGHTS_FD_READDIR
    | __WASI_RIGHTS_PATH_READLINK
    | __WASI_RIGHTS_PATH_RENAME_SOURCE
    | __WASI_RIGHTS_PATH_RENAME_TARGET
    | __WASI_RIGHTS_PATH_FILESTAT_GET
    | __WASI_RIGHTS_PATH_FILESTAT_SET_SIZE
    | __WASI_RIGHTS_PATH_FILESTAT_SET_TIMES
    | __WASI_RIGHTS_FD_FILESTAT_GET
    | __WASI_RIGHTS_FD_FILESTAT_SET_SIZE
    | __WASI_RIGHTS_FD_FILESTAT_SET_TIMES
    | __WASI_RIGHTS_PATH_SYMLINK
    | __WASI_RIGHTS_PATH_UNLINK_FILE
    | __WASI_RIGHTS_PATH_REMOVE_DIRECTORY
    | __WASI_RIGHTS_POLL_FD_READWRITE
    | __WASI_RIGHTS_SOCK_SHUTDOWN;

// Block and character device interaction is outside the scope of
// WASI. Simply allow everything.
pub(crate) const RIGHTS_BLOCK_DEVICE_BASE: __wasi_rights_t = RIGHTS_ALL;
pub(crate) const RIGHTS_BLOCK_DEVICE_INHERITING: __wasi_rights_t = RIGHTS_ALL;
pub(crate) const RIGHTS_CHARACTER_DEVICE_BASE: __wasi_rights_t = RIGHTS_ALL;
pub(crate) const RIGHTS_CHARACTER_DEVICE_INHERITING: __wasi_rights_t = RIGHTS_ALL;

// Only allow directory operations on directories. Directories can only
// yield file descriptors to other directories and files.
pub(crate) const RIGHTS_DIRECTORY_BASE: __wasi_rights_t = __WASI_RIGHTS_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHTS_FD_SYNC
    | __WASI_RIGHTS_FD_ADVISE
    | __WASI_RIGHTS_PATH_CREATE_DIRECTORY
    | __WASI_RIGHTS_PATH_CREATE_FILE
    | __WASI_RIGHTS_PATH_LINK_SOURCE
    | __WASI_RIGHTS_PATH_LINK_TARGET
    | __WASI_RIGHTS_PATH_OPEN
    | __WASI_RIGHTS_FD_READDIR
    | __WASI_RIGHTS_PATH_READLINK
    | __WASI_RIGHTS_PATH_RENAME_SOURCE
    | __WASI_RIGHTS_PATH_RENAME_TARGET
    | __WASI_RIGHTS_PATH_FILESTAT_GET
    | __WASI_RIGHTS_PATH_FILESTAT_SET_SIZE
    | __WASI_RIGHTS_PATH_FILESTAT_SET_TIMES
    | __WASI_RIGHTS_FD_FILESTAT_GET
    | __WASI_RIGHTS_FD_FILESTAT_SET_TIMES
    | __WASI_RIGHTS_PATH_SYMLINK
    | __WASI_RIGHTS_PATH_UNLINK_FILE
    | __WASI_RIGHTS_PATH_REMOVE_DIRECTORY
    | __WASI_RIGHTS_POLL_FD_READWRITE;
pub(crate) const RIGHTS_DIRECTORY_INHERITING: __wasi_rights_t =
    RIGHTS_DIRECTORY_BASE | RIGHTS_REGULAR_FILE_BASE;

// Operations that apply to regular files.
pub(crate) const RIGHTS_REGULAR_FILE_BASE: __wasi_rights_t = __WASI_RIGHTS_FD_DATASYNC
    | __WASI_RIGHTS_FD_READ
    | __WASI_RIGHTS_FD_SEEK
    | __WASI_RIGHTS_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHTS_FD_SYNC
    | __WASI_RIGHTS_FD_TELL
    | __WASI_RIGHTS_FD_WRITE
    | __WASI_RIGHTS_FD_ADVISE
    | __WASI_RIGHTS_FD_ALLOCATE
    | __WASI_RIGHTS_FD_FILESTAT_GET
    | __WASI_RIGHTS_FD_FILESTAT_SET_SIZE
    | __WASI_RIGHTS_FD_FILESTAT_SET_TIMES
    | __WASI_RIGHTS_POLL_FD_READWRITE;
pub(crate) const RIGHTS_REGULAR_FILE_INHERITING: __wasi_rights_t = 0;

// Operations that apply to sockets and socket pairs.
pub(crate) const RIGHTS_SOCKET_BASE: __wasi_rights_t = __WASI_RIGHTS_FD_READ
    | __WASI_RIGHTS_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHTS_FD_WRITE
    | __WASI_RIGHTS_FD_FILESTAT_GET
    | __WASI_RIGHTS_POLL_FD_READWRITE
    | __WASI_RIGHTS_SOCK_SHUTDOWN;
pub(crate) const RIGHTS_SOCKET_INHERITING: __wasi_rights_t = RIGHTS_ALL;

// Operations that apply to TTYs.
pub(crate) const RIGHTS_TTY_BASE: __wasi_rights_t = __WASI_RIGHTS_FD_READ
    | __WASI_RIGHTS_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHTS_FD_WRITE
    | __WASI_RIGHTS_FD_FILESTAT_GET
    | __WASI_RIGHTS_POLL_FD_READWRITE;
#[allow(unused)]
pub(crate) const RIGHTS_TTY_INHERITING: __wasi_rights_t = 0;

pub fn whence_to_str(whence: __wasi_whence_t) -> &'static str {
    match whence {
        __WASI_WHENCE_CUR => "__WASI_WHENCE_CUR",
        __WASI_WHENCE_END => "__WASI_WHENCE_END",
        __WASI_WHENCE_SET => "__WASI_WHENCE_SET",
        other => panic!("Undefined whence value {:?}", other),
    }
}

pub const __WASI_DIRCOOKIE_START: __wasi_dircookie_t = 0;

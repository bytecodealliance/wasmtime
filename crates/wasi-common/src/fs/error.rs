use crate::wasi;
use std::io;

/// Translate a WASI errno code into an `io::Result<()>`.
///
/// TODO: Would it be better to have our own version of `io::Error` (and
/// `io::Result`), rather than trying to shoehorn WASI errors into the
/// libstd version?
pub(crate) fn wasi_errno_to_io_error(errno: wasi::__wasi_errno_t) -> io::Result<()> {
    #[cfg(unix)]
    let raw_os_error = match errno {
        wasi::__WASI_ERRNO_SUCCESS => return Ok(()),
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
        wasi::__WASI_ERRNO_SUCCESS => return Ok(()),
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
            return Err(io::Error::new(io::ErrorKind::Other, error_str(errno)))
        }
        _ => panic!("unrecognized WASI errno value"),
    } as i32;

    Err(io::Error::from_raw_os_error(raw_os_error))
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

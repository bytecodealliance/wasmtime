use crate::host;
use std::io;

/// Translate a WASI errno code into an `io::Result<()>`.
///
/// TODO: Would it be better to have our own version of `io::Error` (and
/// `io::Result`), rather than trying to shoehorn WASI errors into the
/// libstd version?
pub(crate) fn wasi_errno_to_io_error(errno: host::__wasi_errno_t) -> io::Result<()> {
    #[cfg(unix)]
    let raw_os_error = match errno {
        host::__WASI_ESUCCESS => return Ok(()),
        host::__WASI_EIO => libc::EIO,
        host::__WASI_EPERM => libc::EPERM,
        host::__WASI_EINVAL => libc::EINVAL,
        host::__WASI_EPIPE => libc::EPIPE,
        host::__WASI_ENOTCONN => libc::ENOTCONN,
        host::__WASI_E2BIG => libc::E2BIG,
        host::__WASI_EACCES => libc::EACCES,
        host::__WASI_EADDRINUSE => libc::EADDRINUSE,
        host::__WASI_EADDRNOTAVAIL => libc::EADDRNOTAVAIL,
        host::__WASI_EAFNOSUPPORT => libc::EAFNOSUPPORT,
        host::__WASI_EAGAIN => libc::EAGAIN,
        host::__WASI_EALREADY => libc::EALREADY,
        host::__WASI_EBADF => libc::EBADF,
        host::__WASI_EBADMSG => libc::EBADMSG,
        host::__WASI_EBUSY => libc::EBUSY,
        host::__WASI_ECANCELED => libc::ECANCELED,
        host::__WASI_ECHILD => libc::ECHILD,
        host::__WASI_ECONNABORTED => libc::ECONNABORTED,
        host::__WASI_ECONNREFUSED => libc::ECONNREFUSED,
        host::__WASI_ECONNRESET => libc::ECONNRESET,
        host::__WASI_EDEADLK => libc::EDEADLK,
        host::__WASI_EDESTADDRREQ => libc::EDESTADDRREQ,
        host::__WASI_EDOM => libc::EDOM,
        host::__WASI_EDQUOT => libc::EDQUOT,
        host::__WASI_EEXIST => libc::EEXIST,
        host::__WASI_EFAULT => libc::EFAULT,
        host::__WASI_EFBIG => libc::EFBIG,
        host::__WASI_EHOSTUNREACH => libc::EHOSTUNREACH,
        host::__WASI_EIDRM => libc::EIDRM,
        host::__WASI_EILSEQ => libc::EILSEQ,
        host::__WASI_EINPROGRESS => libc::EINPROGRESS,
        host::__WASI_EINTR => libc::EINTR,
        host::__WASI_EISCONN => libc::EISCONN,
        host::__WASI_EISDIR => libc::EISDIR,
        host::__WASI_ELOOP => libc::ELOOP,
        host::__WASI_EMFILE => libc::EMFILE,
        host::__WASI_EMLINK => libc::EMLINK,
        host::__WASI_EMSGSIZE => libc::EMSGSIZE,
        host::__WASI_EMULTIHOP => libc::EMULTIHOP,
        host::__WASI_ENAMETOOLONG => libc::ENAMETOOLONG,
        host::__WASI_ENETDOWN => libc::ENETDOWN,
        host::__WASI_ENETRESET => libc::ENETRESET,
        host::__WASI_ENETUNREACH => libc::ENETUNREACH,
        host::__WASI_ENFILE => libc::ENFILE,
        host::__WASI_ENOBUFS => libc::ENOBUFS,
        host::__WASI_ENODEV => libc::ENODEV,
        host::__WASI_ENOENT => libc::ENOENT,
        host::__WASI_ENOEXEC => libc::ENOEXEC,
        host::__WASI_ENOLCK => libc::ENOLCK,
        host::__WASI_ENOLINK => libc::ENOLINK,
        host::__WASI_ENOMEM => libc::ENOMEM,
        host::__WASI_ENOMSG => libc::ENOMSG,
        host::__WASI_ENOPROTOOPT => libc::ENOPROTOOPT,
        host::__WASI_ENOSPC => libc::ENOSPC,
        host::__WASI_ENOSYS => libc::ENOSYS,
        host::__WASI_ENOTDIR => libc::ENOTDIR,
        host::__WASI_ENOTEMPTY => libc::ENOTEMPTY,
        host::__WASI_ENOTRECOVERABLE => libc::ENOTRECOVERABLE,
        host::__WASI_ENOTSOCK => libc::ENOTSOCK,
        host::__WASI_ENOTSUP => libc::ENOTSUP,
        host::__WASI_ENOTTY => libc::ENOTTY,
        host::__WASI_ENXIO => libc::ENXIO,
        host::__WASI_EOVERFLOW => libc::EOVERFLOW,
        host::__WASI_EOWNERDEAD => libc::EOWNERDEAD,
        host::__WASI_EPROTO => libc::EPROTO,
        host::__WASI_EPROTONOSUPPORT => libc::EPROTONOSUPPORT,
        host::__WASI_EPROTOTYPE => libc::EPROTOTYPE,
        host::__WASI_ERANGE => libc::ERANGE,
        host::__WASI_EROFS => libc::EROFS,
        host::__WASI_ESPIPE => libc::ESPIPE,
        host::__WASI_ESRCH => libc::ESRCH,
        host::__WASI_ESTALE => libc::ESTALE,
        host::__WASI_ETIMEDOUT => libc::ETIMEDOUT,
        host::__WASI_ETXTBSY => libc::ETXTBSY,
        host::__WASI_EXDEV => libc::EXDEV,
        #[cfg(target_os = "wasi")]
        host::__WASI_ENOTCAPABLE => libc::ENOTCAPABLE,
        #[cfg(not(target_os = "wasi"))]
        host::__WASI_ENOTCAPABLE => libc::EIO,
        _ => panic!("unexpected wasi errno value"),
    };

    #[cfg(windows)]
    use winapi::shared::winerror::*;

    #[cfg(windows)]
    let raw_os_error = match errno {
        host::__WASI_ESUCCESS => return Ok(()),
        host::__WASI_EINVAL => WSAEINVAL,
        host::__WASI_EPIPE => ERROR_BROKEN_PIPE,
        host::__WASI_ENOTCONN => WSAENOTCONN,
        host::__WASI_EPERM | host::__WASI_EACCES => ERROR_ACCESS_DENIED,
        host::__WASI_EADDRINUSE => WSAEADDRINUSE,
        host::__WASI_EADDRNOTAVAIL => WSAEADDRNOTAVAIL,
        host::__WASI_EAGAIN => WSAEWOULDBLOCK,
        host::__WASI_ECONNABORTED => WSAECONNABORTED,
        host::__WASI_ECONNREFUSED => WSAECONNREFUSED,
        host::__WASI_ECONNRESET => WSAECONNRESET,
        host::__WASI_EEXIST => ERROR_ALREADY_EXISTS,
        host::__WASI_ENOENT => ERROR_FILE_NOT_FOUND,
        host::__WASI_ETIMEDOUT => WSAETIMEDOUT,
        host::__WASI_EAFNOSUPPORT => WSAEAFNOSUPPORT,
        host::__WASI_EALREADY => WSAEALREADY,
        host::__WASI_EBADF => WSAEBADF,
        host::__WASI_EDESTADDRREQ => WSAEDESTADDRREQ,
        host::__WASI_EDQUOT => WSAEDQUOT,
        host::__WASI_EFAULT => WSAEFAULT,
        host::__WASI_EHOSTUNREACH => WSAEHOSTUNREACH,
        host::__WASI_EINPROGRESS => WSAEINPROGRESS,
        host::__WASI_EINTR => WSAEINTR,
        host::__WASI_EISCONN => WSAEISCONN,
        host::__WASI_ELOOP => WSAELOOP,
        host::__WASI_EMFILE => WSAEMFILE,
        host::__WASI_EMSGSIZE => WSAEMSGSIZE,
        host::__WASI_ENAMETOOLONG => WSAENAMETOOLONG,
        host::__WASI_ENETDOWN => WSAENETDOWN,
        host::__WASI_ENETRESET => WSAENETRESET,
        host::__WASI_ENETUNREACH => WSAENETUNREACH,
        host::__WASI_ENOBUFS => WSAENOBUFS,
        host::__WASI_ENOPROTOOPT => WSAENOPROTOOPT,
        host::__WASI_ENOTEMPTY => WSAENOTEMPTY,
        host::__WASI_ENOTSOCK => WSAENOTSOCK,
        host::__WASI_EPROTONOSUPPORT => WSAEPROTONOSUPPORT,
        host::__WASI_EPROTOTYPE => WSAEPROTOTYPE,
        host::__WASI_ESTALE => WSAESTALE,
        host::__WASI_EIO
        | host::__WASI_EISDIR
        | host::__WASI_E2BIG
        | host::__WASI_EBADMSG
        | host::__WASI_EBUSY
        | host::__WASI_ECANCELED
        | host::__WASI_ECHILD
        | host::__WASI_EDEADLK
        | host::__WASI_EDOM
        | host::__WASI_EFBIG
        | host::__WASI_EIDRM
        | host::__WASI_EILSEQ
        | host::__WASI_EMLINK
        | host::__WASI_EMULTIHOP
        | host::__WASI_ENFILE
        | host::__WASI_ENODEV
        | host::__WASI_ENOEXEC
        | host::__WASI_ENOLCK
        | host::__WASI_ENOLINK
        | host::__WASI_ENOMEM
        | host::__WASI_ENOMSG
        | host::__WASI_ENOSPC
        | host::__WASI_ENOSYS
        | host::__WASI_ENOTDIR
        | host::__WASI_ENOTRECOVERABLE
        | host::__WASI_ENOTSUP
        | host::__WASI_ENOTTY
        | host::__WASI_ENXIO
        | host::__WASI_EOVERFLOW
        | host::__WASI_EOWNERDEAD
        | host::__WASI_EPROTO
        | host::__WASI_ERANGE
        | host::__WASI_EROFS
        | host::__WASI_ESPIPE
        | host::__WASI_ESRCH
        | host::__WASI_ETXTBSY
        | host::__WASI_EXDEV
        | host::__WASI_ENOTCAPABLE => {
            return Err(io::Error::new(io::ErrorKind::Other, error_str(errno)))
        }
        _ => panic!("unrecognized WASI errno value"),
    } as i32;

    Err(io::Error::from_raw_os_error(raw_os_error))
}

#[cfg(windows)]
fn error_str(errno: host::__wasi_errno_t) -> &'static str {
    match errno {
        host::__WASI_E2BIG => "Argument list too long",
        host::__WASI_EACCES => "Permission denied",
        host::__WASI_EADDRINUSE => "Address in use",
        host::__WASI_EADDRNOTAVAIL => "Address not available",
        host::__WASI_EAFNOSUPPORT => "Address family not supported by protocol",
        host::__WASI_EAGAIN => "Resource temporarily unavailable",
        host::__WASI_EALREADY => "Operation already in progress",
        host::__WASI_EBADF => "Bad file descriptor",
        host::__WASI_EBADMSG => "Bad message",
        host::__WASI_EBUSY => "Resource busy",
        host::__WASI_ECANCELED => "Operation canceled",
        host::__WASI_ECHILD => "No child process",
        host::__WASI_ECONNABORTED => "Connection aborted",
        host::__WASI_ECONNREFUSED => "Connection refused",
        host::__WASI_ECONNRESET => "Connection reset by peer",
        host::__WASI_EDEADLK => "Resource deadlock would occur",
        host::__WASI_EDESTADDRREQ => "Destination address required",
        host::__WASI_EDOM => "Domain error",
        host::__WASI_EDQUOT => "Quota exceeded",
        host::__WASI_EEXIST => "File exists",
        host::__WASI_EFAULT => "Bad address",
        host::__WASI_EFBIG => "File too large",
        host::__WASI_EHOSTUNREACH => "Host is unreachable",
        host::__WASI_EIDRM => "Identifier removed",
        host::__WASI_EILSEQ => "Illegal byte sequence",
        host::__WASI_EINPROGRESS => "Operation in progress",
        host::__WASI_EINTR => "Interrupted system call",
        host::__WASI_EINVAL => "Invalid argument",
        host::__WASI_EIO => "Remote I/O error",
        host::__WASI_EISCONN => "Socket is connected",
        host::__WASI_EISDIR => "Is a directory",
        host::__WASI_ELOOP => "Symbolic link loop",
        host::__WASI_EMFILE => "No file descriptors available",
        host::__WASI_EMLINK => "Too many links",
        host::__WASI_EMSGSIZE => "Message too large",
        host::__WASI_EMULTIHOP => "Multihop attempted",
        host::__WASI_ENAMETOOLONG => "Filename too long",
        host::__WASI_ENETDOWN => "Network is down",
        host::__WASI_ENETRESET => "Connection reset by network",
        host::__WASI_ENETUNREACH => "Network unreachable",
        host::__WASI_ENFILE => "Too many open files in system",
        host::__WASI_ENOBUFS => "No buffer space available",
        host::__WASI_ENODEV => "No such device",
        host::__WASI_ENOENT => "No such file or directory",
        host::__WASI_ENOEXEC => "Exec format error",
        host::__WASI_ENOLCK => "No locks available",
        host::__WASI_ENOLINK => "Link has been severed",
        host::__WASI_ENOMEM => "Out of memory",
        host::__WASI_ENOMSG => "No message of desired type",
        host::__WASI_ENOPROTOOPT => "Protocol not available",
        host::__WASI_ENOSPC => "No space left on device",
        host::__WASI_ENOSYS => "Function not implemented",
        host::__WASI_ENOTCONN => "Socket not connected",
        host::__WASI_ENOTDIR => "Not a directory",
        host::__WASI_ENOTEMPTY => "Directory not empty",
        host::__WASI_ENOTRECOVERABLE => "State not recoverable",
        host::__WASI_ENOTSOCK => "Not a socket",
        host::__WASI_ENOTSUP => "Not supported",
        host::__WASI_ENOTTY => "Not a tty",
        host::__WASI_ENXIO => "No such device or address",
        host::__WASI_EOVERFLOW => "Value too large for data type",
        host::__WASI_EOWNERDEAD => "Previous owner died",
        host::__WASI_EPERM => "Operation not permitted",
        host::__WASI_EPIPE => "Broken pipe",
        host::__WASI_EPROTO => "Protocol error",
        host::__WASI_EPROTONOSUPPORT => "Protocol not supported",
        host::__WASI_EPROTOTYPE => "Protocol wrong type for socket",
        host::__WASI_ERANGE => "Result not representable",
        host::__WASI_EROFS => "Read-only file system",
        host::__WASI_ESPIPE => "Invalid seek",
        host::__WASI_ESRCH => "No such process",
        host::__WASI_ESTALE => "Stale file handle",
        host::__WASI_ETIMEDOUT => "Operation timed out",
        host::__WASI_ETXTBSY => "Text file busy",
        host::__WASI_EXDEV => "Cross-device link",
        host::__WASI_ENOTCAPABLE => "Capabilities insufficient",
        _ => panic!("unrecognized WASI errno value"),
    }
}

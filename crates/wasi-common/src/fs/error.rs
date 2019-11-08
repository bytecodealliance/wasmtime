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
        wasi::__WASI_ESUCCESS => return Ok(()),
        wasi::__WASI_EIO => libc::EIO,
        wasi::__WASI_EPERM => libc::EPERM,
        wasi::__WASI_EINVAL => libc::EINVAL,
        wasi::__WASI_EPIPE => libc::EPIPE,
        wasi::__WASI_ENOTCONN => libc::ENOTCONN,
        wasi::__WASI_E2BIG => libc::E2BIG,
        wasi::__WASI_EACCES => libc::EACCES,
        wasi::__WASI_EADDRINUSE => libc::EADDRINUSE,
        wasi::__WASI_EADDRNOTAVAIL => libc::EADDRNOTAVAIL,
        wasi::__WASI_EAFNOSUPPORT => libc::EAFNOSUPPORT,
        wasi::__WASI_EAGAIN => libc::EAGAIN,
        wasi::__WASI_EALREADY => libc::EALREADY,
        wasi::__WASI_EBADF => libc::EBADF,
        wasi::__WASI_EBADMSG => libc::EBADMSG,
        wasi::__WASI_EBUSY => libc::EBUSY,
        wasi::__WASI_ECANCELED => libc::ECANCELED,
        wasi::__WASI_ECHILD => libc::ECHILD,
        wasi::__WASI_ECONNABORTED => libc::ECONNABORTED,
        wasi::__WASI_ECONNREFUSED => libc::ECONNREFUSED,
        wasi::__WASI_ECONNRESET => libc::ECONNRESET,
        wasi::__WASI_EDEADLK => libc::EDEADLK,
        wasi::__WASI_EDESTADDRREQ => libc::EDESTADDRREQ,
        wasi::__WASI_EDOM => libc::EDOM,
        wasi::__WASI_EDQUOT => libc::EDQUOT,
        wasi::__WASI_EEXIST => libc::EEXIST,
        wasi::__WASI_EFAULT => libc::EFAULT,
        wasi::__WASI_EFBIG => libc::EFBIG,
        wasi::__WASI_EHOSTUNREACH => libc::EHOSTUNREACH,
        wasi::__WASI_EIDRM => libc::EIDRM,
        wasi::__WASI_EILSEQ => libc::EILSEQ,
        wasi::__WASI_EINPROGRESS => libc::EINPROGRESS,
        wasi::__WASI_EINTR => libc::EINTR,
        wasi::__WASI_EISCONN => libc::EISCONN,
        wasi::__WASI_EISDIR => libc::EISDIR,
        wasi::__WASI_ELOOP => libc::ELOOP,
        wasi::__WASI_EMFILE => libc::EMFILE,
        wasi::__WASI_EMLINK => libc::EMLINK,
        wasi::__WASI_EMSGSIZE => libc::EMSGSIZE,
        wasi::__WASI_EMULTIHOP => libc::EMULTIHOP,
        wasi::__WASI_ENAMETOOLONG => libc::ENAMETOOLONG,
        wasi::__WASI_ENETDOWN => libc::ENETDOWN,
        wasi::__WASI_ENETRESET => libc::ENETRESET,
        wasi::__WASI_ENETUNREACH => libc::ENETUNREACH,
        wasi::__WASI_ENFILE => libc::ENFILE,
        wasi::__WASI_ENOBUFS => libc::ENOBUFS,
        wasi::__WASI_ENODEV => libc::ENODEV,
        wasi::__WASI_ENOENT => libc::ENOENT,
        wasi::__WASI_ENOEXEC => libc::ENOEXEC,
        wasi::__WASI_ENOLCK => libc::ENOLCK,
        wasi::__WASI_ENOLINK => libc::ENOLINK,
        wasi::__WASI_ENOMEM => libc::ENOMEM,
        wasi::__WASI_ENOMSG => libc::ENOMSG,
        wasi::__WASI_ENOPROTOOPT => libc::ENOPROTOOPT,
        wasi::__WASI_ENOSPC => libc::ENOSPC,
        wasi::__WASI_ENOSYS => libc::ENOSYS,
        wasi::__WASI_ENOTDIR => libc::ENOTDIR,
        wasi::__WASI_ENOTEMPTY => libc::ENOTEMPTY,
        wasi::__WASI_ENOTRECOVERABLE => libc::ENOTRECOVERABLE,
        wasi::__WASI_ENOTSOCK => libc::ENOTSOCK,
        wasi::__WASI_ENOTSUP => libc::ENOTSUP,
        wasi::__WASI_ENOTTY => libc::ENOTTY,
        wasi::__WASI_ENXIO => libc::ENXIO,
        wasi::__WASI_EOVERFLOW => libc::EOVERFLOW,
        wasi::__WASI_EOWNERDEAD => libc::EOWNERDEAD,
        wasi::__WASI_EPROTO => libc::EPROTO,
        wasi::__WASI_EPROTONOSUPPORT => libc::EPROTONOSUPPORT,
        wasi::__WASI_EPROTOTYPE => libc::EPROTOTYPE,
        wasi::__WASI_ERANGE => libc::ERANGE,
        wasi::__WASI_EROFS => libc::EROFS,
        wasi::__WASI_ESPIPE => libc::ESPIPE,
        wasi::__WASI_ESRCH => libc::ESRCH,
        wasi::__WASI_ESTALE => libc::ESTALE,
        wasi::__WASI_ETIMEDOUT => libc::ETIMEDOUT,
        wasi::__WASI_ETXTBSY => libc::ETXTBSY,
        wasi::__WASI_EXDEV => libc::EXDEV,
        #[cfg(target_os = "wasi")]
        wasi::__WASI_ENOTCAPABLE => libc::ENOTCAPABLE,
        #[cfg(not(target_os = "wasi"))]
        wasi::__WASI_ENOTCAPABLE => libc::EIO,
        _ => panic!("unexpected wasi errno value"),
    };

    #[cfg(windows)]
    use winapi::shared::winerror::*;

    #[cfg(windows)]
    let raw_os_error = match errno {
        wasi::__WASI_ESUCCESS => return Ok(()),
        wasi::__WASI_EINVAL => WSAEINVAL,
        wasi::__WASI_EPIPE => ERROR_BROKEN_PIPE,
        wasi::__WASI_ENOTCONN => WSAENOTCONN,
        wasi::__WASI_EPERM | wasi::__WASI_EACCES => ERROR_ACCESS_DENIED,
        wasi::__WASI_EADDRINUSE => WSAEADDRINUSE,
        wasi::__WASI_EADDRNOTAVAIL => WSAEADDRNOTAVAIL,
        wasi::__WASI_EAGAIN => WSAEWOULDBLOCK,
        wasi::__WASI_ECONNABORTED => WSAECONNABORTED,
        wasi::__WASI_ECONNREFUSED => WSAECONNREFUSED,
        wasi::__WASI_ECONNRESET => WSAECONNRESET,
        wasi::__WASI_EEXIST => ERROR_ALREADY_EXISTS,
        wasi::__WASI_ENOENT => ERROR_FILE_NOT_FOUND,
        wasi::__WASI_ETIMEDOUT => WSAETIMEDOUT,
        wasi::__WASI_EAFNOSUPPORT => WSAEAFNOSUPPORT,
        wasi::__WASI_EALREADY => WSAEALREADY,
        wasi::__WASI_EBADF => WSAEBADF,
        wasi::__WASI_EDESTADDRREQ => WSAEDESTADDRREQ,
        wasi::__WASI_EDQUOT => WSAEDQUOT,
        wasi::__WASI_EFAULT => WSAEFAULT,
        wasi::__WASI_EHOSTUNREACH => WSAEHOSTUNREACH,
        wasi::__WASI_EINPROGRESS => WSAEINPROGRESS,
        wasi::__WASI_EINTR => WSAEINTR,
        wasi::__WASI_EISCONN => WSAEISCONN,
        wasi::__WASI_ELOOP => WSAELOOP,
        wasi::__WASI_EMFILE => WSAEMFILE,
        wasi::__WASI_EMSGSIZE => WSAEMSGSIZE,
        wasi::__WASI_ENAMETOOLONG => WSAENAMETOOLONG,
        wasi::__WASI_ENETDOWN => WSAENETDOWN,
        wasi::__WASI_ENETRESET => WSAENETRESET,
        wasi::__WASI_ENETUNREACH => WSAENETUNREACH,
        wasi::__WASI_ENOBUFS => WSAENOBUFS,
        wasi::__WASI_ENOPROTOOPT => WSAENOPROTOOPT,
        wasi::__WASI_ENOTEMPTY => WSAENOTEMPTY,
        wasi::__WASI_ENOTSOCK => WSAENOTSOCK,
        wasi::__WASI_EPROTONOSUPPORT => WSAEPROTONOSUPPORT,
        wasi::__WASI_EPROTOTYPE => WSAEPROTOTYPE,
        wasi::__WASI_ESTALE => WSAESTALE,
        wasi::__WASI_EIO
        | wasi::__WASI_EISDIR
        | wasi::__WASI_E2BIG
        | wasi::__WASI_EBADMSG
        | wasi::__WASI_EBUSY
        | wasi::__WASI_ECANCELED
        | wasi::__WASI_ECHILD
        | wasi::__WASI_EDEADLK
        | wasi::__WASI_EDOM
        | wasi::__WASI_EFBIG
        | wasi::__WASI_EIDRM
        | wasi::__WASI_EILSEQ
        | wasi::__WASI_EMLINK
        | wasi::__WASI_EMULTIHOP
        | wasi::__WASI_ENFILE
        | wasi::__WASI_ENODEV
        | wasi::__WASI_ENOEXEC
        | wasi::__WASI_ENOLCK
        | wasi::__WASI_ENOLINK
        | wasi::__WASI_ENOMEM
        | wasi::__WASI_ENOMSG
        | wasi::__WASI_ENOSPC
        | wasi::__WASI_ENOSYS
        | wasi::__WASI_ENOTDIR
        | wasi::__WASI_ENOTRECOVERABLE
        | wasi::__WASI_ENOTSUP
        | wasi::__WASI_ENOTTY
        | wasi::__WASI_ENXIO
        | wasi::__WASI_EOVERFLOW
        | wasi::__WASI_EOWNERDEAD
        | wasi::__WASI_EPROTO
        | wasi::__WASI_ERANGE
        | wasi::__WASI_EROFS
        | wasi::__WASI_ESPIPE
        | wasi::__WASI_ESRCH
        | wasi::__WASI_ETXTBSY
        | wasi::__WASI_EXDEV
        | wasi::__WASI_ENOTCAPABLE => {
            return Err(io::Error::new(io::ErrorKind::Other, error_str(errno)))
        }
        _ => panic!("unrecognized WASI errno value"),
    } as i32;

    Err(io::Error::from_raw_os_error(raw_os_error))
}

#[cfg(windows)]
fn error_str(errno: wasi::__wasi_errno_t) -> &'static str {
    match errno {
        wasi::__WASI_E2BIG => "Argument list too long",
        wasi::__WASI_EACCES => "Permission denied",
        wasi::__WASI_EADDRINUSE => "Address in use",
        wasi::__WASI_EADDRNOTAVAIL => "Address not available",
        wasi::__WASI_EAFNOSUPPORT => "Address family not supported by protocol",
        wasi::__WASI_EAGAIN => "Resource temporarily unavailable",
        wasi::__WASI_EALREADY => "Operation already in progress",
        wasi::__WASI_EBADF => "Bad file descriptor",
        wasi::__WASI_EBADMSG => "Bad message",
        wasi::__WASI_EBUSY => "Resource busy",
        wasi::__WASI_ECANCELED => "Operation canceled",
        wasi::__WASI_ECHILD => "No child process",
        wasi::__WASI_ECONNABORTED => "Connection aborted",
        wasi::__WASI_ECONNREFUSED => "Connection refused",
        wasi::__WASI_ECONNRESET => "Connection reset by peer",
        wasi::__WASI_EDEADLK => "Resource deadlock would occur",
        wasi::__WASI_EDESTADDRREQ => "Destination address required",
        wasi::__WASI_EDOM => "Domain error",
        wasi::__WASI_EDQUOT => "Quota exceeded",
        wasi::__WASI_EEXIST => "File exists",
        wasi::__WASI_EFAULT => "Bad address",
        wasi::__WASI_EFBIG => "File too large",
        wasi::__WASI_EHOSTUNREACH => "Host is unreachable",
        wasi::__WASI_EIDRM => "Identifier removed",
        wasi::__WASI_EILSEQ => "Illegal byte sequence",
        wasi::__WASI_EINPROGRESS => "Operation in progress",
        wasi::__WASI_EINTR => "Interrupted system call",
        wasi::__WASI_EINVAL => "Invalid argument",
        wasi::__WASI_EIO => "Remote I/O error",
        wasi::__WASI_EISCONN => "Socket is connected",
        wasi::__WASI_EISDIR => "Is a directory",
        wasi::__WASI_ELOOP => "Symbolic link loop",
        wasi::__WASI_EMFILE => "No file descriptors available",
        wasi::__WASI_EMLINK => "Too many links",
        wasi::__WASI_EMSGSIZE => "Message too large",
        wasi::__WASI_EMULTIHOP => "Multihop attempted",
        wasi::__WASI_ENAMETOOLONG => "Filename too long",
        wasi::__WASI_ENETDOWN => "Network is down",
        wasi::__WASI_ENETRESET => "Connection reset by network",
        wasi::__WASI_ENETUNREACH => "Network unreachable",
        wasi::__WASI_ENFILE => "Too many open files in system",
        wasi::__WASI_ENOBUFS => "No buffer space available",
        wasi::__WASI_ENODEV => "No such device",
        wasi::__WASI_ENOENT => "No such file or directory",
        wasi::__WASI_ENOEXEC => "Exec format error",
        wasi::__WASI_ENOLCK => "No locks available",
        wasi::__WASI_ENOLINK => "Link has been severed",
        wasi::__WASI_ENOMEM => "Out of memory",
        wasi::__WASI_ENOMSG => "No message of desired type",
        wasi::__WASI_ENOPROTOOPT => "Protocol not available",
        wasi::__WASI_ENOSPC => "No space left on device",
        wasi::__WASI_ENOSYS => "Function not implemented",
        wasi::__WASI_ENOTCONN => "Socket not connected",
        wasi::__WASI_ENOTDIR => "Not a directory",
        wasi::__WASI_ENOTEMPTY => "Directory not empty",
        wasi::__WASI_ENOTRECOVERABLE => "State not recoverable",
        wasi::__WASI_ENOTSOCK => "Not a socket",
        wasi::__WASI_ENOTSUP => "Not supported",
        wasi::__WASI_ENOTTY => "Not a tty",
        wasi::__WASI_ENXIO => "No such device or address",
        wasi::__WASI_EOVERFLOW => "Value too large for data type",
        wasi::__WASI_EOWNERDEAD => "Previous owner died",
        wasi::__WASI_EPERM => "Operation not permitted",
        wasi::__WASI_EPIPE => "Broken pipe",
        wasi::__WASI_EPROTO => "Protocol error",
        wasi::__WASI_EPROTONOSUPPORT => "Protocol not supported",
        wasi::__WASI_EPROTOTYPE => "Protocol wrong type for socket",
        wasi::__WASI_ERANGE => "Result not representable",
        wasi::__WASI_EROFS => "Read-only file system",
        wasi::__WASI_ESPIPE => "Invalid seek",
        wasi::__WASI_ESRCH => "No such process",
        wasi::__WASI_ESTALE => "Stale file handle",
        wasi::__WASI_ETIMEDOUT => "Operation timed out",
        wasi::__WASI_ETXTBSY => "Text file busy",
        wasi::__WASI_EXDEV => "Cross-device link",
        wasi::__WASI_ENOTCAPABLE => "Capabilities insufficient",
        _ => panic!("unrecognized WASI errno value"),
    }
}

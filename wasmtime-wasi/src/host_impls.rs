use super::host;
use super::wasm32;
use errno::{errno, Errno};

/// Convert POSIX error code to host's WASI error code
fn convert_errno(error: Errno) -> host::__wasi_errno_t {
    #[allow(unreachable_patterns)]
    match error.into() {
        libc::E2BIG => host::__WASI_E2BIG,
        libc::EACCES => host::__WASI_EACCES,
        libc::EADDRINUSE => host::__WASI_EADDRINUSE,
        libc::EADDRNOTAVAIL => host::__WASI_EADDRNOTAVAIL,
        libc::EAFNOSUPPORT => host::__WASI_EAFNOSUPPORT,
        libc::EAGAIN | libc::EWOULDBLOCK => host::__WASI_EAGAIN,
        libc::EALREADY => host::__WASI_EALREADY,
        libc::EBADF => host::__WASI_EBADF,
        libc::EBADMSG => host::__WASI_EBADMSG,
        libc::EBUSY => host::__WASI_EBUSY,
        libc::ECANCELED => host::__WASI_ECANCELED,
        libc::ECHILD => host::__WASI_ECHILD,
        libc::ECONNABORTED => host::__WASI_ECONNABORTED,
        libc::ECONNREFUSED => host::__WASI_ECONNREFUSED,
        libc::ECONNRESET => host::__WASI_ECONNRESET,
        libc::EDEADLK => host::__WASI_EDEADLK,
        libc::EDESTADDRREQ => host::__WASI_EDESTADDRREQ,
        libc::EDOM => host::__WASI_EDOM,
        libc::EDQUOT => host::__WASI_EDQUOT,
        libc::EEXIST => host::__WASI_EEXIST,
        libc::EFAULT => host::__WASI_EFAULT,
        libc::EFBIG => host::__WASI_EFBIG,
        libc::EHOSTUNREACH => host::__WASI_EHOSTUNREACH,
        libc::EIDRM => host::__WASI_EIDRM,
        libc::EILSEQ => host::__WASI_EILSEQ,
        libc::EINPROGRESS => host::__WASI_EINPROGRESS,
        libc::EINTR => host::__WASI_EINTR,
        libc::EINVAL => host::__WASI_EINVAL,
        libc::EIO => host::__WASI_EIO,
        libc::EISCONN => host::__WASI_EISCONN,
        libc::EISDIR => host::__WASI_EISDIR,
        libc::ELOOP => host::__WASI_ELOOP,
        libc::EMFILE => host::__WASI_EMFILE,
        libc::EMLINK => host::__WASI_EMLINK,
        libc::EMSGSIZE => host::__WASI_EMSGSIZE,
        libc::EMULTIHOP => host::__WASI_EMULTIHOP,
        libc::ENAMETOOLONG => host::__WASI_ENAMETOOLONG,
        libc::ENETDOWN => host::__WASI_ENETDOWN,
        libc::ENETRESET => host::__WASI_ENETRESET,
        libc::ENETUNREACH => host::__WASI_ENETUNREACH,
        libc::ENFILE => host::__WASI_ENFILE,
        libc::ENOBUFS => host::__WASI_ENOBUFS,
        libc::ENODEV => host::__WASI_ENODEV,
        libc::ENOENT => host::__WASI_ENOENT,
        libc::ENOEXEC => host::__WASI_ENOEXEC,
        libc::ENOLCK => host::__WASI_ENOLCK,
        libc::ENOLINK => host::__WASI_ENOLINK,
        libc::ENOMEM => host::__WASI_ENOMEM,
        libc::ENOMSG => host::__WASI_ENOMSG,
        libc::ENOPROTOOPT => host::__WASI_ENOPROTOOPT,
        libc::ENOSPC => host::__WASI_ENOSPC,
        libc::ENOSYS => host::__WASI_ENOSYS,
        // TODO: verify if this is correct
        #[cfg(target_os = "freebsd")]
        libc::ENOTCAPABLE => host::__WASI_ENOTCAPABLE,
        libc::ENOTCONN => host::__WASI_ENOTCONN,
        libc::ENOTDIR => host::__WASI_ENOTDIR,
        libc::ENOTEMPTY => host::__WASI_ENOTEMPTY,
        libc::ENOTRECOVERABLE => host::__WASI_ENOTRECOVERABLE,
        libc::ENOTSOCK => host::__WASI_ENOTSOCK,
        libc::ENOTSUP | libc::EOPNOTSUPP => host::__WASI_ENOTSUP,
        libc::ENOTTY => host::__WASI_ENOTTY,
        libc::ENXIO => host::__WASI_ENXIO,
        libc::EOVERFLOW => host::__WASI_EOVERFLOW,
        libc::EOWNERDEAD => host::__WASI_EOWNERDEAD,
        libc::EPERM => host::__WASI_EPERM,
        libc::EPIPE => host::__WASI_EPIPE,
        libc::EPROTO => host::__WASI_EPROTO,
        libc::EPROTONOSUPPORT => host::__WASI_EPROTONOSUPPORT,
        libc::EPROTOTYPE => host::__WASI_EPROTOTYPE,
        libc::ERANGE => host::__WASI_ERANGE,
        libc::EROFS => host::__WASI_EROFS,
        libc::ESPIPE => host::__WASI_ESPIPE,
        libc::ESRCH => host::__WASI_ESRCH,
        libc::ESTALE => host::__WASI_ESTALE,
        libc::ETIMEDOUT => host::__WASI_ETIMEDOUT,
        libc::ETXTBSY => host::__WASI_ETXTBSY,
        libc::EXDEV => host::__WASI_EXDEV,
        _ => host::__WASI_ENOSYS,
    }
}

pub fn wasmtime_ssp_proc_exit(rval: wasm32::__wasi_exitcode_t) {
    ::std::process::exit(rval as i32)
}

pub fn wasmtime_ssp_sched_yield() -> host::__wasi_errno_t {
    unsafe {
        if libc::sched_yield() < 0 {
            return convert_errno(errno());
        }
    }

    host::__WASI_ESUCCESS
}

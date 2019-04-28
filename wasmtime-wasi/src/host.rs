#![allow(non_camel_case_types, dead_code)]

include!(concat!(env!("OUT_DIR"), "/wasmtime_ssp.rs"));

pub type char = ::std::os::raw::c_char;
pub type void = ::std::os::raw::c_void;

use super::wasm32;

// Taken from rust's implementation of `std::os` functionality for unix systems
// rust/src/libstd/sys/unix/os.rs
// https://github.com/rust-lang/rust/blob/9ebf47851a357faa4cd97f4b1dc7835f6376e639/src/libstd/sys/unix/os.rs
extern "C" {
    #[cfg(not(target_os = "dragonfly"))]
    #[cfg_attr(
        any(target_os = "linux", target_os = "fuchsia", target_os = "l4re"),
        link_name = "__errno_location"
    )]
    #[cfg_attr(
        any(
            target_os = "bitrig",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "android",
            target_os = "hermit",
            target_env = "newlib"
        ),
        link_name = "__errno"
    )]
    #[cfg_attr(target_os = "solaris", link_name = "___errno")]
    #[cfg_attr(
        any(target_os = "macos", target_os = "ios", target_os = "freebsd"),
        link_name = "__error"
    )]
    #[cfg_attr(target_os = "haiku", link_name = "_errnop")]
    fn errno_location() -> *mut libc::c_int;
}

#[cfg(not(target_os = "dragonfly"))]
pub fn errno() -> i32 {
    unsafe { (*errno_location()) as i32 }
}

#[cfg(target_os = "dragonfly")]
pub fn errno() -> i32 {
    extern "C" {
        #[thread_local]
        static errno: c_int;
    }

    unsafe { errno as i32 }
}

pub fn convert_errno(errno: i32) -> wasm32::__wasi_errno_t {
    #[allow(unreachable_patterns)]
    match errno {
        libc::E2BIG => wasm32::__WASI_E2BIG,
        libc::EACCES => wasm32::__WASI_EACCES,
        libc::EADDRINUSE => wasm32::__WASI_EADDRINUSE,
        libc::EADDRNOTAVAIL => wasm32::__WASI_EADDRNOTAVAIL,
        libc::EAFNOSUPPORT => wasm32::__WASI_EAFNOSUPPORT,
        libc::EAGAIN | libc::EWOULDBLOCK => wasm32::__WASI_EAGAIN,
        libc::EALREADY => wasm32::__WASI_EALREADY,
        libc::EBADF => wasm32::__WASI_EBADF,
        libc::EBADMSG => wasm32::__WASI_EBADMSG,
        libc::EBUSY => wasm32::__WASI_EBUSY,
        libc::ECANCELED => wasm32::__WASI_ECANCELED,
        libc::ECHILD => wasm32::__WASI_ECHILD,
        libc::ECONNABORTED => wasm32::__WASI_ECONNABORTED,
        libc::ECONNREFUSED => wasm32::__WASI_ECONNREFUSED,
        libc::ECONNRESET => wasm32::__WASI_ECONNRESET,
        libc::EDEADLK => wasm32::__WASI_EDEADLK,
        libc::EDESTADDRREQ => wasm32::__WASI_EDESTADDRREQ,
        libc::EDOM => wasm32::__WASI_EDOM,
        libc::EDQUOT => wasm32::__WASI_EDQUOT,
        libc::EEXIST => wasm32::__WASI_EEXIST,
        libc::EFAULT => wasm32::__WASI_EFAULT,
        libc::EFBIG => wasm32::__WASI_EFBIG,
        libc::EHOSTUNREACH => wasm32::__WASI_EHOSTUNREACH,
        libc::EIDRM => wasm32::__WASI_EIDRM,
        libc::EILSEQ => wasm32::__WASI_EILSEQ,
        libc::EINPROGRESS => wasm32::__WASI_EINPROGRESS,
        libc::EINTR => wasm32::__WASI_EINTR,
        libc::EINVAL => wasm32::__WASI_EINVAL,
        libc::EIO => wasm32::__WASI_EIO,
        libc::EISCONN => wasm32::__WASI_EISCONN,
        libc::EISDIR => wasm32::__WASI_EISDIR,
        libc::ELOOP => wasm32::__WASI_ELOOP,
        libc::EMFILE => wasm32::__WASI_EMFILE,
        libc::EMLINK => wasm32::__WASI_EMLINK,
        libc::EMSGSIZE => wasm32::__WASI_EMSGSIZE,
        libc::EMULTIHOP => wasm32::__WASI_EMULTIHOP,
        libc::ENAMETOOLONG => wasm32::__WASI_ENAMETOOLONG,
        libc::ENETDOWN => wasm32::__WASI_ENETDOWN,
        libc::ENETRESET => wasm32::__WASI_ENETRESET,
        libc::ENETUNREACH => wasm32::__WASI_ENETUNREACH,
        libc::ENFILE => wasm32::__WASI_ENFILE,
        libc::ENOBUFS => wasm32::__WASI_ENOBUFS,
        libc::ENODEV => wasm32::__WASI_ENODEV,
        libc::ENOENT => wasm32::__WASI_ENOENT,
        libc::ENOEXEC => wasm32::__WASI_ENOEXEC,
        libc::ENOLCK => wasm32::__WASI_ENOLCK,
        libc::ENOLINK => wasm32::__WASI_ENOLINK,
        libc::ENOMEM => wasm32::__WASI_ENOMEM,
        libc::ENOMSG => wasm32::__WASI_ENOMSG,
        libc::ENOPROTOOPT => wasm32::__WASI_ENOPROTOOPT,
        libc::ENOSPC => wasm32::__WASI_ENOSPC,
        libc::ENOSYS => wasm32::__WASI_ENOSYS,
        // TODO: verify if this is correct
        #[cfg(target_os = "freebsd")]
        libc::ENOTCAPABLE => wasm32::__WASI_ENOTCAPABLE,
        libc::ENOTCONN => wasm32::__WASI_ENOTCONN,
        libc::ENOTDIR => wasm32::__WASI_ENOTDIR,
        libc::ENOTEMPTY => wasm32::__WASI_ENOTEMPTY,
        libc::ENOTRECOVERABLE => wasm32::__WASI_ENOTRECOVERABLE,
        libc::ENOTSOCK => wasm32::__WASI_ENOTSOCK,
        libc::ENOTSUP | libc::EOPNOTSUPP => wasm32::__WASI_ENOTSUP,
        libc::ENOTTY => wasm32::__WASI_ENOTTY,
        libc::ENXIO => wasm32::__WASI_ENXIO,
        libc::EOVERFLOW => wasm32::__WASI_EOVERFLOW,
        libc::EOWNERDEAD => wasm32::__WASI_EOWNERDEAD,
        libc::EPERM => wasm32::__WASI_EPERM,
        libc::EPIPE => wasm32::__WASI_EPIPE,
        libc::EPROTO => wasm32::__WASI_EPROTO,
        libc::EPROTONOSUPPORT => wasm32::__WASI_EPROTONOSUPPORT,
        libc::EPROTOTYPE => wasm32::__WASI_EPROTOTYPE,
        libc::ERANGE => wasm32::__WASI_ERANGE,
        libc::EROFS => wasm32::__WASI_EROFS,
        libc::ESPIPE => wasm32::__WASI_ESPIPE,
        libc::ESRCH => wasm32::__WASI_ESRCH,
        libc::ESTALE => wasm32::__WASI_ESTALE,
        libc::ETIMEDOUT => wasm32::__WASI_ETIMEDOUT,
        libc::ETXTBSY => wasm32::__WASI_ETXTBSY,
        libc::EXDEV => wasm32::__WASI_EXDEV,
        _ => wasm32::__WASI_ENOSYS,
    }
}

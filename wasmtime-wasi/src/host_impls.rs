use super::host;
use errno::{errno, Errno};
use std::slice;

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

fn fd_prestats_get_entry(
    pt: &host::fd_prestats,
    fd: host::__wasi_fd_t,
) -> Option<&host::fd_prestat> {
    // Test for file descriptor existence
    if fd as usize >= pt.size {
        return None;
    }

    let prestat = unsafe { &*pt.prestats.add(fd as usize) };
    if prestat.dir_name == ::std::ptr::null() {
        return None;
    }

    Some(prestat)
}

macro_rules! rwlock_rdlock {
    ($prestats:expr) => {
        unsafe {
            host::rwlock_rdlock(&mut (*$prestats).lock as *mut host::rwlock);
        }
    };
}

macro_rules! rwlock_unlock {
    ($prestats:expr) => {
        unsafe {
            host::rwlock_unlock(&mut (*$prestats).lock as *mut host::rwlock);
        }
    };
}

pub fn wasmtime_ssp_args_get(
    argv_environ: &mut host::argv_environ_values,
    argv: &mut [*mut host::char],
    argv_buf: &mut [host::char],
) -> host::__wasi_errno_t {
    for i in 0..argv_environ.argc {
        let buf_off;
        unsafe {
            buf_off = usize::checked_sub(
                argv_environ.argv.offset(i as isize) as _,
                argv_environ.argv_buf as _,
            )
            .expect("argv[i] - argv_buf overflows");
        }
        argv[i] = argv_buf[buf_off..].as_mut_ptr();
    }
    argv[argv_environ.argc] = std::ptr::null_mut();
    let argv_environ_buf;
    unsafe {
        argv_environ_buf =
            slice::from_raw_parts_mut(argv_environ.argv_buf, argv_environ.argv_buf_size);
    }
    argv_buf.copy_from_slice(argv_environ_buf);
    host::__WASI_ESUCCESS
}

pub fn wasmtime_ssp_args_sizes_get(
    argv_environ: &mut host::argv_environ_values,
    argc: &mut usize,
    argv_buf_size: &mut usize,
) -> host::__wasi_errno_t {
    *argc = argv_environ.argc;
    *argv_buf_size = argv_environ.argv_buf_size;
    host::__WASI_ESUCCESS
}

pub fn wasmtime_ssp_environ_get(
    argv_environ: &mut host::argv_environ_values,
    environ: &mut [*mut host::char],
    environ_buf: &mut [host::char],
) -> host::__wasi_errno_t {
    for i in 0..(*argv_environ).environ_count {
        let buf_off;
        unsafe {
            buf_off = usize::checked_sub(
                argv_environ.environ.offset(i as isize) as _,
                argv_environ.environ_buf as _,
            )
            .expect("environ[i] - environ_buf overflows");
        }
        environ[i] = environ_buf[buf_off..].as_mut_ptr();
    }
    environ[argv_environ.environ_count] = std::ptr::null_mut();
    let argv_environ_buf;
    unsafe {
        argv_environ_buf =
            slice::from_raw_parts_mut(argv_environ.environ_buf, argv_environ.environ_buf_size);
    }
    environ_buf.copy_from_slice(argv_environ_buf);
    host::__WASI_ESUCCESS
}

pub fn wasmtime_ssp_environ_sizes_get(
    argv_environ: &mut host::argv_environ_values,
    environ_count: &mut usize,
    environ_buf_size: &mut usize,
) -> host::__wasi_errno_t {
    *environ_count = argv_environ.environ_count;
    *environ_buf_size = argv_environ.environ_buf_size;
    host::__WASI_ESUCCESS
}

pub fn wasmtime_ssp_proc_exit(rval: host::__wasi_exitcode_t) {
    ::std::process::exit(rval as i32)
}

pub fn wasmtime_ssp_fd_prestat_get(
    prestats: &mut host::fd_prestats,
    fd: host::__wasi_fd_t,
    buf: &mut host::__wasi_prestat_t,
) -> host::__wasi_errno_t {
    rwlock_rdlock!(prestats);

    let ret_code = if let Some(prestat) = fd_prestats_get_entry(prestats, fd) {
        buf.pr_type = host::__WASI_PREOPENTYPE_DIR;
        unsafe {
            buf.u.dir.pr_name_len = prestat.dir_name_len;
        }
        host::__WASI_ESUCCESS
    } else {
        host::__WASI_EBADF
    };

    rwlock_unlock!(prestats);

    ret_code
}

pub fn wasmtime_ssp_sched_yield() -> host::__wasi_errno_t {
    unsafe {
        if libc::sched_yield() < 0 {
            return convert_errno(errno());
        }
    }

    host::__WASI_ESUCCESS
}

//! Hostcalls that implement
//! [WASI](https://github.com/CraneStation/wasmtime-wasi/blob/wasi/docs/WASI-overview.md).
//!
//! This code borrows heavily from [wasmtime-wasi](https://github.com/CraneStation/wasmtime-wasi),
//! which in turn borrows from cloudabi-utils. See `LICENSE.wasmtime-wasi` for license information.
//!
//! This is currently a very incomplete prototype, only supporting the hostcalls required to run
//! `/examples/hello.c`, and using a bare-bones translation of the capabilities system rather than
//! something nice.

#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use crate::ctx::VmContext;
use crate::fdentry::{determine_type_rights, FdEntry};
use crate::memory::*;
use crate::{host, wasm32};

use cast::From as _0;

use nix::convert_ioctl_res;
use nix::libc::c_int;
use std::ffi::{OsStr, OsString};
use std::os::unix::prelude::{FromRawFd, OsStrExt, OsStringExt, RawFd};
use std::time::SystemTime;
use std::{cmp, slice};

pub unsafe fn proc_exit(_vmctx: &mut VmContext, rval: wasm32::__wasi_exitcode_t) -> () {
    std::process::exit(dec_exitcode(rval) as i32);
}

pub unsafe fn args_get(
    vmctx: *mut VmContext,
    argv_ptr: wasm32::uintptr_t,
    argv_buf: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = (*vmctx).as_wasi_ctx();

    let mut argv_buf_offset = 0;
    let mut argv = vec![];

    for arg in (*ctx).args.iter() {
        let arg_bytes = arg.as_bytes_with_nul();
        let arg_ptr = argv_buf + argv_buf_offset;

        if let Err(e) = unsafe { enc_slice_of(vmctx, arg_bytes, arg_ptr) } {
            return enc_errno(e);
        }

        argv.push(arg_ptr);

        argv_buf_offset = if let Some(new_offset) = argv_buf_offset.checked_add(
            wasm32::uintptr_t::cast(arg_bytes.len())
                .expect("cast overflow would have been caught by `enc_slice_of` above"),
        ) {
            new_offset
        } else {
            return wasm32::__WASI_EOVERFLOW;
        }
    }

    unsafe {
        enc_slice_of(vmctx, argv.as_slice(), argv_ptr)
            .map(|_| wasm32::__WASI_ESUCCESS)
            .unwrap_or_else(|e| e)
    }
}

pub unsafe fn args_sizes_get(
    vmctx: *mut VmContext,
    argc_ptr: wasm32::uintptr_t,
    argv_buf_size_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = (*vmctx).as_wasi_ctx();

    let argc = (*ctx).args.len();
    let argv_size = (*ctx)
        .args
        .iter()
        .map(|arg| arg.as_bytes_with_nul().len())
        .sum();

    unsafe {
        if let Err(e) = enc_usize_byref(vmctx, argc_ptr, argc) {
            return enc_errno(e);
        }
        if let Err(e) = enc_usize_byref(vmctx, argv_buf_size_ptr, argv_size) {
            return enc_errno(e);
        }
    }
    wasm32::__WASI_ESUCCESS
}

pub unsafe fn clock_res_get(
    vmctx: *mut VmContext,
    clock_id: wasm32::__wasi_clockid_t,
    resolution_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    // convert the supported clocks to the libc types, or return EINVAL
    let clock_id = match dec_clockid(clock_id) {
        host::__WASI_CLOCK_REALTIME => libc::CLOCK_REALTIME,
        host::__WASI_CLOCK_MONOTONIC => libc::CLOCK_MONOTONIC,
        host::__WASI_CLOCK_PROCESS_CPUTIME_ID => libc::CLOCK_PROCESS_CPUTIME_ID,
        host::__WASI_CLOCK_THREAD_CPUTIME_ID => libc::CLOCK_THREAD_CPUTIME_ID,
        _ => return wasm32::__WASI_EINVAL,
    };

    // no `nix` wrapper for clock_getres, so we do it ourselves
    let mut timespec = unsafe { std::mem::uninitialized::<libc::timespec>() };
    let res = unsafe { libc::clock_getres(clock_id, &mut timespec as *mut libc::timespec) };
    if res != 0 {
        return wasm32::errno_from_nix(nix::errno::Errno::last());
    }

    // convert to nanoseconds, returning EOVERFLOW in case of overflow; this is freelancing a bit
    // from the spec but seems like it'll be an unusual situation to hit
    (timespec.tv_sec as host::__wasi_timestamp_t)
        .checked_mul(1_000_000_000)
        .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        .map(|resolution| {
            // a supported clock can never return zero; this case will probably never get hit, but
            // make sure we follow the spec
            if resolution == 0 {
                wasm32::__WASI_EINVAL
            } else {
                unsafe {
                    enc_timestamp_byref(vmctx, resolution_ptr, resolution)
                        .map(|_| wasm32::__WASI_ESUCCESS)
                        .unwrap_or_else(|e| e)
                }
            }
        })
        .unwrap_or(wasm32::__WASI_EOVERFLOW)
}

pub unsafe fn clock_time_get(
    vmctx: *mut VmContext,
    clock_id: wasm32::__wasi_clockid_t,
    // ignored for now, but will be useful once we put optional limits on precision to reduce side
    // channels
    _precision: wasm32::__wasi_timestamp_t,
    time_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    // convert the supported clocks to the libc types, or return EINVAL
    let clock_id = match dec_clockid(clock_id) {
        host::__WASI_CLOCK_REALTIME => libc::CLOCK_REALTIME,
        host::__WASI_CLOCK_MONOTONIC => libc::CLOCK_MONOTONIC,
        host::__WASI_CLOCK_PROCESS_CPUTIME_ID => libc::CLOCK_PROCESS_CPUTIME_ID,
        host::__WASI_CLOCK_THREAD_CPUTIME_ID => libc::CLOCK_THREAD_CPUTIME_ID,
        _ => return wasm32::__WASI_EINVAL,
    };

    // no `nix` wrapper for clock_getres, so we do it ourselves
    let mut timespec = unsafe { std::mem::uninitialized::<libc::timespec>() };
    let res = unsafe { libc::clock_gettime(clock_id, &mut timespec as *mut libc::timespec) };
    if res != 0 {
        return wasm32::errno_from_nix(nix::errno::Errno::last());
    }

    // convert to nanoseconds, returning EOVERFLOW in case of overflow; this is freelancing a bit
    // from the spec but seems like it'll be an unusual situation to hit
    (timespec.tv_sec as host::__wasi_timestamp_t)
        .checked_mul(1_000_000_000)
        .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        .map(|time| unsafe {
            enc_timestamp_byref(vmctx, time_ptr, time)
                .map(|_| wasm32::__WASI_ESUCCESS)
                .unwrap_or_else(|e| e)
        })
        .unwrap_or(wasm32::__WASI_EOVERFLOW)
}

pub unsafe fn environ_get(
    vmctx: *mut VmContext,
    environ_ptr: wasm32::uintptr_t,
    environ_buf: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = (*vmctx).as_wasi_ctx();

    let mut environ_buf_offset = 0;
    let mut environ = vec![];

    for pair in (*ctx).env.iter() {
        let env_bytes = pair.as_bytes_with_nul();
        let env_ptr = environ_buf + environ_buf_offset;

        if let Err(e) = unsafe { enc_slice_of(vmctx, env_bytes, env_ptr) } {
            return enc_errno(e);
        }

        environ.push(env_ptr);

        environ_buf_offset = if let Some(new_offset) = environ_buf_offset.checked_add(
            wasm32::uintptr_t::cast(env_bytes.len())
                .expect("cast overflow would have been caught by `enc_slice_of` above"),
        ) {
            new_offset
        } else {
            return wasm32::__WASI_EOVERFLOW;
        }
    }

    unsafe {
        enc_slice_of(vmctx, environ.as_slice(), environ_ptr)
            .map(|_| wasm32::__WASI_ESUCCESS)
            .unwrap_or_else(|e| e)
    }
}

pub unsafe fn environ_sizes_get(
    vmctx: *mut VmContext,
    environ_count_ptr: wasm32::uintptr_t,
    environ_size_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = (*vmctx).as_wasi_ctx();

    let environ_count = (*ctx).env.len();
    if let Some(environ_size) = (*ctx).env.iter().try_fold(0, |acc: u32, pair| {
        acc.checked_add(pair.as_bytes_with_nul().len() as u32)
    }) {
        unsafe {
            if let Err(e) = enc_usize_byref(vmctx, environ_count_ptr, environ_count) {
                return enc_errno(e);
            }
            if let Err(e) = enc_usize_byref(vmctx, environ_size_ptr, environ_size as usize) {
                return enc_errno(e);
            }
        }
        wasm32::__WASI_ESUCCESS
    } else {
        wasm32::__WASI_EOVERFLOW
    }
}

pub unsafe fn fd_close(vmctx: *mut VmContext, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    let ctx = (*vmctx).as_wasi_ctx_mut();
    let fd = dec_fd(fd);
    if let Some(fdent) = (*ctx).fds.get(&fd) {
        // can't close preopened files
        if fdent.preopen_path.is_some() {
            return wasm32::__WASI_ENOTSUP;
        }
    }
    if let Some(mut fdent) = (*ctx).fds.remove(&fd) {
        fdent.fd_object.needs_close = false;
        match nix::unistd::close(fdent.fd_object.rawfd) {
            Ok(_) => wasm32::__WASI_ESUCCESS,
            Err(e) => wasm32::errno_from_nix(e.as_errno().unwrap()),
        }
    } else {
        wasm32::__WASI_EBADF
    }
}

pub unsafe fn fd_fdstat_get(
    vmctx: *mut VmContext,
    fd: wasm32::__wasi_fd_t,
    fdstat_ptr: wasm32::uintptr_t, // *mut wasm32::__wasi_fdstat_t
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let mut host_fdstat = match unsafe { dec_fdstat_byref(vmctx, fdstat_ptr) } {
        Ok(host_fdstat) => host_fdstat,
        Err(e) => return enc_errno(e),
    };

    let ctx = (*vmctx).as_wasi_ctx_mut();
    let errno = if let Some(fe) = (*ctx).fds.get(&host_fd) {
        host_fdstat.fs_filetype = fe.fd_object.ty;
        host_fdstat.fs_rights_base = fe.rights_base;
        host_fdstat.fs_rights_inheriting = fe.rights_inheriting;
        use nix::fcntl::{fcntl, OFlag, F_GETFL};
        match fcntl(fe.fd_object.rawfd, F_GETFL).map(OFlag::from_bits_truncate) {
            Ok(flags) => {
                host_fdstat.fs_flags = host::fdflags_from_nix(flags);
                wasm32::__WASI_ESUCCESS
            }
            Err(e) => wasm32::errno_from_nix(e.as_errno().unwrap()),
        }
    } else {
        wasm32::__WASI_EBADF
    };

    unsafe {
        enc_fdstat_byref(vmctx, fdstat_ptr, host_fdstat)
            .expect("can write back into the pointer we read from");
    }

    errno
}

pub unsafe fn fd_fdstat_set_flags(
    vmctx: &mut VmContext,
    fd: wasm32::__wasi_fd_t,
    fdflags: wasm32::__wasi_fdflags_t,
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let host_fdflags = dec_fdflags(fdflags);
    let nix_flags = host::nix_from_fdflags(host_fdflags);

    let ctx = (*vmctx).as_wasi_ctx_mut();

    if let Some(fe) = (*ctx).fds.get(&host_fd) {
        match nix::fcntl::fcntl(fe.fd_object.rawfd, nix::fcntl::F_SETFL(nix_flags)) {
            Ok(_) => wasm32::__WASI_ESUCCESS,
            Err(e) => wasm32::errno_from_nix(e.as_errno().unwrap()),
        }
    } else {
        wasm32::__WASI_EBADF
    }
}

pub unsafe fn fd_seek(
    vmctx: *mut VmContext,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filedelta_t,
    whence: wasm32::__wasi_whence_t,
    newoffset: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = (*vmctx).as_wasi_ctx_mut();
    let fd = dec_fd(fd);
    let offset = dec_filedelta(offset);
    let whence = dec_whence(whence);

    let host_newoffset = {
        use nix::unistd::{lseek, Whence};
        let nwhence = match whence {
            host::__WASI_WHENCE_CUR => Whence::SeekCur,
            host::__WASI_WHENCE_END => Whence::SeekEnd,
            host::__WASI_WHENCE_SET => Whence::SeekSet,
            _ => return wasm32::__WASI_EINVAL,
        };

        let rights = if offset == 0 && whence == host::__WASI_WHENCE_CUR {
            host::__WASI_RIGHT_FD_TELL
        } else {
            host::__WASI_RIGHT_FD_SEEK | host::__WASI_RIGHT_FD_TELL
        };
        match (*ctx).get_fd_entry(fd, rights.into(), 0) {
            Ok(fe) => match lseek(fe.fd_object.rawfd, offset, nwhence) {
                Ok(newoffset) => newoffset,
                Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
            },
            Err(e) => return enc_errno(e),
        }
    };

    unsafe {
        enc_filesize_byref(vmctx, newoffset, host_newoffset as u64)
            .map(|_| wasm32::__WASI_ESUCCESS)
            .unwrap_or_else(|e| e)
    }
}

pub unsafe fn fd_prestat_get(
    vmctx: *mut VmContext,
    fd: wasm32::__wasi_fd_t,
    prestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = (*vmctx).as_wasi_ctx();
    let fd = dec_fd(fd);
    // TODO: is this the correct right for this?
    match (*ctx).get_fd_entry(fd, host::__WASI_RIGHT_PATH_OPEN.into(), 0) {
        Ok(fe) => {
            if let Some(po_path) = &fe.preopen_path {
                if fe.fd_object.ty != host::__WASI_FILETYPE_DIRECTORY {
                    return wasm32::__WASI_ENOTDIR;
                }
                unsafe {
                    enc_prestat_byref(
                        vmctx,
                        prestat_ptr,
                        host::__wasi_prestat_t {
                            pr_type: host::__WASI_PREOPENTYPE_DIR,
                            u: host::__wasi_prestat_t___wasi_prestat_u {
                                dir:
                                    host::__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t {
                                        pr_name_len: po_path.as_os_str().as_bytes().len(),
                                    },
                            },
                        },
                    )
                    .map(|_| wasm32::__WASI_ESUCCESS)
                    .unwrap_or_else(|e| e)
                }
            } else {
                wasm32::__WASI_ENOTSUP
            }
        }
        Err(e) => enc_errno(e),
    }
}

pub unsafe fn fd_prestat_dir_name(
    vmctx: *mut VmContext,
    fd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);

    match (*(*vmctx).as_wasi_ctx()).get_fd_entry(fd, host::__WASI_RIGHT_PATH_OPEN.into(), 0) {
        Ok(fe) => {
            if let Some(po_path) = &fe.preopen_path {
                if fe.fd_object.ty != host::__WASI_FILETYPE_DIRECTORY {
                    return wasm32::__WASI_ENOTDIR;
                }
                let path_bytes = po_path.as_os_str().as_bytes();
                if path_bytes.len() > dec_usize(path_len) {
                    return wasm32::__WASI_ENAMETOOLONG;
                }
                unsafe {
                    enc_slice_of(vmctx, path_bytes, path_ptr)
                        .map(|_| wasm32::__WASI_ESUCCESS)
                        .unwrap_or_else(|e| e)
                }
            } else {
                wasm32::__WASI_ENOTSUP
            }
        }
        Err(e) => enc_errno(e),
    }
}

pub unsafe fn fd_read(
    vmctx: *mut VmContext,
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nread: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::sys::uio::{readv, IoVec};

    let fd = dec_fd(fd);
    let mut iovs = match unsafe { dec_ciovec_slice(vmctx, iovs_ptr, iovs_len) } {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };

    let ctx = (*vmctx).as_wasi_ctx_mut();
    let fe = match (*ctx).get_fd_entry(fd, host::__WASI_RIGHT_FD_READ.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };

    let mut iovs: Vec<IoVec<&mut [u8]>> = iovs
        .iter_mut()
        .map(|iov| unsafe { host::ciovec_to_nix_mut(iov) })
        .collect();

    let host_nread = match readv(fe.fd_object.rawfd, &mut iovs) {
        Ok(len) => len,
        Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
    };

    if host_nread == 0 {
        // we hit eof, so remove the fdentry from the context
        let mut fe = (*ctx).fds.remove(&fd).expect("file entry is still there");
        fe.fd_object.needs_close = false;
    }

    unsafe {
        enc_usize_byref(vmctx, nread, host_nread)
            .map(|_| wasm32::__WASI_ESUCCESS)
            .unwrap_or_else(|e| e)
    }
}

pub unsafe fn fd_write(
    vmctx: *mut VmContext,
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nwritten: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::sys::uio::{writev, IoVec};

    let fd = dec_fd(fd);
    let iovs = match unsafe { dec_ciovec_slice(vmctx, iovs_ptr, iovs_len) } {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };

    let ctx = (*vmctx).as_wasi_ctx();
    let fe = match (*ctx).get_fd_entry(fd, host::__WASI_RIGHT_FD_WRITE.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };

    let iovs: Vec<IoVec<&[u8]>> = iovs
        .iter()
        .map(|iov| unsafe { host::ciovec_to_nix(iov) })
        .collect();

    let host_nwritten = match writev(fe.fd_object.rawfd, &iovs) {
        Ok(len) => len,
        Err(e) => return wasm32::errno_from_nix(e.as_errno().unwrap()),
    };

    unsafe {
        enc_usize_byref(vmctx, nwritten, host_nwritten)
            .map(|_| wasm32::__WASI_ESUCCESS)
            .unwrap_or_else(|e| e)
    }
}

pub unsafe fn path_open(
    vmctx: *mut VmContext,
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    oflags: wasm32::__wasi_oflags_t,
    fs_rights_base: wasm32::__wasi_rights_t,
    fs_rights_inheriting: wasm32::__wasi_rights_t,
    fs_flags: wasm32::__wasi_fdflags_t,
    fd_out_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::errno::Errno;
    use nix::fcntl::{openat, AtFlags, OFlag};
    use nix::sys::stat::{fstatat, Mode, SFlag};

    let dirfd = dec_fd(dirfd);
    let dirflags = dec_lookupflags(dirflags);
    let oflags = dec_oflags(oflags);
    let fs_rights_base = dec_rights(fs_rights_base);
    let fs_rights_inheriting = dec_rights(fs_rights_inheriting);
    let fs_flags = dec_fdflags(fs_flags);

    // which open mode do we need?
    let read = fs_rights_base & (host::__WASI_RIGHT_FD_READ | host::__WASI_RIGHT_FD_READDIR) != 0;
    let write = fs_rights_base
        & (host::__WASI_RIGHT_FD_DATASYNC
            | host::__WASI_RIGHT_FD_WRITE
            | host::__WASI_RIGHT_FD_ALLOCATE
            | host::__WASI_RIGHT_PATH_FILESTAT_SET_SIZE)
        != 0;

    let mut nix_all_oflags = if read && write {
        OFlag::O_RDWR
    } else if read {
        OFlag::O_RDONLY
    } else {
        OFlag::O_WRONLY
    };

    // on non-Capsicum systems, we always want nofollow
    nix_all_oflags.insert(OFlag::O_NOFOLLOW);

    // which rights are needed on the dirfd?
    let mut needed_base = host::__WASI_RIGHT_PATH_OPEN;
    let mut needed_inheriting = fs_rights_base | fs_rights_inheriting;

    // convert open flags
    let nix_oflags = host::nix_from_oflags(oflags);
    nix_all_oflags.insert(nix_oflags);
    if nix_all_oflags.contains(OFlag::O_CREAT) {
        needed_base |= host::__WASI_RIGHT_PATH_CREATE_FILE;
    }
    if nix_all_oflags.contains(OFlag::O_TRUNC) {
        needed_inheriting |= host::__WASI_RIGHT_PATH_FILESTAT_SET_SIZE;
    }

    // convert file descriptor flags
    nix_all_oflags.insert(host::nix_from_fdflags(fs_flags));
    if nix_all_oflags.contains(OFlag::O_DSYNC) {
        needed_inheriting |= host::__WASI_RIGHT_FD_DATASYNC;
    }
    if nix_all_oflags.intersects(host::O_RSYNC | OFlag::O_SYNC) {
        needed_inheriting |= host::__WASI_RIGHT_FD_SYNC;
    }

    let path = match unsafe { dec_slice_of::<u8>(vmctx, path_ptr, path_len) } {
        Ok((ptr, len)) => OsStr::from_bytes(unsafe { std::slice::from_raw_parts(ptr, len) }),
        Err(e) => return enc_errno(e),
    };

    let (dir, path) = match path_get(
        &*vmctx,
        dirfd,
        dirflags,
        path,
        needed_base,
        needed_inheriting,
        nix_oflags.contains(OFlag::O_CREAT),
    ) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };

    let new_fd = match openat(
        dir,
        path.as_os_str(),
        nix_all_oflags,
        Mode::from_bits_truncate(0o777),
    ) {
        Ok(fd) => fd,
        Err(e) => {
            match e.as_errno() {
                // Linux returns ENXIO instead of EOPNOTSUPP when opening a socket
                Some(Errno::ENXIO) => {
                    if let Ok(stat) = fstatat(dir, path.as_os_str(), AtFlags::AT_SYMLINK_NOFOLLOW) {
                        if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFSOCK) {
                            return wasm32::__WASI_ENOTSUP;
                        } else {
                            return wasm32::__WASI_ENXIO;
                        }
                    } else {
                        return wasm32::__WASI_ENXIO;
                    }
                }
                Some(e) => return wasm32::errno_from_nix(e),
                None => return wasm32::__WASI_ENOSYS,
            }
        }
    };

    // Determine the type of the new file descriptor and which rights contradict with this type
    let guest_fd = match unsafe { determine_type_rights(new_fd) } {
        Err(e) => {
            // if `close` fails, note it but do not override the underlying errno
            nix::unistd::close(new_fd).unwrap_or_else(|e| {
                dbg!(e);
            });
            return enc_errno(e);
        }
        Ok((_ty, max_base, max_inheriting)) => {
            let mut fe = unsafe { FdEntry::from_raw_fd(new_fd) };
            fe.rights_base &= max_base;
            fe.rights_inheriting &= max_inheriting;
            match (*(*vmctx).as_wasi_ctx_mut()).insert_fd_entry(fe) {
                Ok(fd) => fd,
                Err(e) => return enc_errno(e),
            }
        }
    };

    unsafe {
        enc_fd_byref(vmctx, fd_out_ptr, guest_fd)
            .map(|_| wasm32::__WASI_ESUCCESS)
            .unwrap_or_else(|e| e)
    }
}

pub unsafe fn random_get(
    vmctx: *mut VmContext,
    buf_ptr: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    use rand::{thread_rng, RngCore};

    let buf_len = dec_usize(buf_len);
    let buf_ptr = match unsafe { (*vmctx).dec_ptr(buf_ptr, buf_len) } {
        Ok(ptr) => ptr,
        Err(e) => return enc_errno(e),
    };

    let buf = unsafe { std::slice::from_raw_parts_mut(buf_ptr, buf_len) };

    thread_rng().fill_bytes(buf);

    return wasm32::__WASI_ESUCCESS;
}

pub unsafe fn poll_oneoff(
    vmctx: *mut VmContext,
    input: wasm32::uintptr_t,
    output: wasm32::uintptr_t,
    nsubscriptions: wasm32::size_t,
    nevents: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    if nsubscriptions as u64 > wasm32::__wasi_filesize_t::max_value() {
        return wasm32::__WASI_EINVAL;
    }
    unsafe { enc_pointee(vmctx, nevents, 0) }.unwrap();
    let input_slice_ =
        unsafe { dec_slice_of::<wasm32::__wasi_subscription_t>(vmctx, input, nsubscriptions) }
            .unwrap();
    let input_slice = unsafe { slice::from_raw_parts(input_slice_.0, input_slice_.1) };

    let output_slice_ =
        unsafe { dec_slice_of::<wasm32::__wasi_event_t>(vmctx, output, nsubscriptions) }.unwrap();
    let output_slice = unsafe { slice::from_raw_parts_mut(output_slice_.0, output_slice_.1) };

    let input: Vec<_> = input_slice.iter().map(|x| dec_subscription(x)).collect();

    let timeout = input
        .iter()
        .filter_map(|event| match event {
            Ok(event) if event.type_ == wasm32::__WASI_EVENTTYPE_CLOCK => Some(ClockEventData {
                delay: wasi_clock_to_relative_ns_delay(unsafe { event.u.clock }) / 1_000_000,
                userdata: event.userdata,
            }),
            _ => None,
        })
        .min_by_key(|event| event.delay);
    let fd_events: Vec<_> = input
        .iter()
        .filter_map(|event| match event {
            Ok(event)
                if event.type_ == wasm32::__WASI_EVENTTYPE_FD_READ
                    || event.type_ == wasm32::__WASI_EVENTTYPE_FD_WRITE =>
            {
                Some(FdEventData {
                    fd: unsafe { event.u.fd_readwrite.fd } as c_int,
                    type_: event.type_,
                    userdata: event.userdata,
                })
            }
            _ => None,
        })
        .collect();
    if fd_events.is_empty() && timeout.is_none() {
        return wasm32::__WASI_ESUCCESS;
    }
    let mut poll_fds: Vec<_> = fd_events
        .iter()
        .map(|event| {
            let mut flags = nix::poll::EventFlags::empty();
            match event.type_ {
                wasm32::__WASI_EVENTTYPE_FD_READ => flags.insert(nix::poll::EventFlags::POLLIN),
                wasm32::__WASI_EVENTTYPE_FD_WRITE => flags.insert(nix::poll::EventFlags::POLLOUT),
                // An event on a file descriptor can currently only be of type FD_READ or FD_WRITE
                // Nothing else has been defined in the specification, and these are also the only two
                // events we filtered before. If we get something else here, the code has a serious bug.
                _ => unreachable!(),
            };
            nix::poll::PollFd::new(event.fd, flags)
        })
        .collect();
    let timeout = timeout.map(|ClockEventData { delay, userdata }| ClockEventData {
        delay: cmp::min(delay, c_int::max_value() as u128),
        userdata,
    });
    let poll_timeout = timeout.map(|timeout| timeout.delay as c_int).unwrap_or(-1);
    let ready = loop {
        match nix::poll::poll(&mut poll_fds, poll_timeout) {
            Err(_) => {
                if nix::errno::Errno::last() == nix::errno::Errno::EINTR {
                    continue;
                }
                return wasm32::errno_from_nix(nix::errno::Errno::last());
            }
            Ok(ready) => break ready as usize,
        }
    };
    if ready == 0 {
        return poll_oneoff_handle_timeout_event(&mut *vmctx, output_slice, nevents, timeout);
    }
    let events = fd_events.iter().zip(poll_fds.iter()).take(ready);
    poll_oneoff_handle_fd_event(&mut *vmctx, output_slice, nevents, events)
}

pub unsafe fn fd_filestat_get(
    vmctx: *mut VmContext,
    fd: wasm32::__wasi_fd_t,
    filestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::sys::stat::fstat;

    let host_fd = dec_fd(fd);
    let ctx = (*vmctx).as_wasi_ctx_mut();

    let errno = if let Some(fe) = (*ctx).fds.get(&host_fd) {
        match fstat(fe.fd_object.rawfd) {
            Err(e) => wasm32::errno_from_nix(e.as_errno().unwrap()),
            Ok(filestat) => {
                let host_filestat = host::filestat_from_nix(filestat);
                unsafe {
                    enc_filestat_byref(vmctx, filestat_ptr, host_filestat)
                        .expect("can write into the pointer");
                }
                wasm32::__WASI_ESUCCESS
            }
        }
    } else {
        wasm32::__WASI_EBADF
    };
    errno
}

pub unsafe fn path_filestat_get(
    vmctx: *mut VmContext,
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    filestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use nix::fcntl::AtFlags;
    use nix::sys::stat::fstatat;

    let dirfd = dec_fd(dirfd);
    let dirflags = dec_lookupflags(dirflags);
    let path = match unsafe { dec_slice_of::<u8>(vmctx, path_ptr, path_len) } {
        Ok((ptr, len)) => OsStr::from_bytes(unsafe { std::slice::from_raw_parts(ptr, len) }),
        Err(e) => return enc_errno(e),
    };
    let (dir, path) = match path_get(
        &*vmctx,
        dirfd,
        dirflags,
        path,
        host::__WASI_RIGHT_PATH_FILESTAT_GET,
        0,
        false,
    ) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };
    let atflags = match dirflags {
        0 => AtFlags::empty(),
        _ => AtFlags::AT_SYMLINK_NOFOLLOW,
    };
    match fstatat(dir, path.as_os_str(), atflags) {
        Err(e) => wasm32::errno_from_nix(e.as_errno().unwrap()),
        Ok(filestat) => {
            let host_filestat = host::filestat_from_nix(filestat);
            unsafe {
                enc_filestat_byref(vmctx, filestat_ptr, host_filestat)
                    .expect("can write into the pointer");
            }
            wasm32::__WASI_ESUCCESS
        }
    }
}

pub unsafe fn path_create_directory(
    vmctx: *mut VmContext,
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    use nix::errno;
    use nix::libc::mkdirat;

    let dirfd = dec_fd(dirfd);
    let path = match unsafe { dec_slice_of::<u8>(vmctx, path_ptr, path_len) } {
        Ok((ptr, len)) => OsStr::from_bytes(unsafe { std::slice::from_raw_parts(ptr, len) }),
        Err(e) => return enc_errno(e),
    };
    let (dir, path) = match path_get(
        &*vmctx,
        dirfd,
        0,
        path,
        host::__WASI_RIGHT_PATH_OPEN | host::__WASI_RIGHT_PATH_CREATE_DIRECTORY,
        0,
        false,
    ) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };
    let path_cstr = match std::ffi::CString::new(path.as_os_str().as_bytes()) {
        Ok(path_cstr) => path_cstr,
        Err(_) => return wasm32::__WASI_EINVAL,
    };
    // nix doesn't expose mkdirat() yet
    match unsafe { mkdirat(dir, path_cstr.as_ptr(), 0o777) } {
        0 => wasm32::__WASI_ESUCCESS,
        _ => wasm32::errno_from_nix(errno::Errno::last()),
    }
}

pub unsafe fn path_unlink_file(
    vmctx: *mut VmContext,
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    use nix::errno;
    use nix::libc::unlinkat;

    let dirfd = dec_fd(dirfd);
    let path = match unsafe { dec_slice_of::<u8>(vmctx, path_ptr, path_len) } {
        Ok((ptr, len)) => OsStr::from_bytes(unsafe { std::slice::from_raw_parts(ptr, len) }),
        Err(e) => return enc_errno(e),
    };
    let (dir, path) = match path_get(
        &*vmctx,
        dirfd,
        0,
        path,
        host::__WASI_RIGHT_PATH_UNLINK_FILE,
        0,
        false,
    ) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };
    let path_cstr = match std::ffi::CString::new(path.as_os_str().as_bytes()) {
        Ok(path_cstr) => path_cstr,
        Err(_) => return wasm32::__WASI_EINVAL,
    };
    // nix doesn't expose unlinkat() yet
    match unsafe { unlinkat(dir, path_cstr.as_ptr(), 0) } {
        0 => wasm32::__WASI_ESUCCESS,
        _ => wasm32::errno_from_nix(errno::Errno::last()),
    }
}

// define the `fionread()` function, equivalent to `ioctl(fd, FIONREAD, *bytes)`
nix::ioctl_read_bad!(fionread, nix::libc::FIONREAD, c_int);

fn wasi_clock_to_relative_ns_delay(
    wasi_clock: host::__wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
) -> u128 {
    if wasi_clock.flags != wasm32::__WASI_SUBSCRIPTION_CLOCK_ABSTIME {
        return wasi_clock.timeout as u128;
    }
    let now: u128 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Current date is before the epoch")
        .as_nanos();
    let deadline = wasi_clock.timeout as u128;
    deadline.saturating_sub(now)
}

#[derive(Debug, Copy, Clone)]
struct ClockEventData {
    delay: u128,
    userdata: host::__wasi_userdata_t,
}
#[derive(Debug, Copy, Clone)]
struct FdEventData {
    fd: c_int,
    type_: host::__wasi_eventtype_t,
    userdata: host::__wasi_userdata_t,
}

fn poll_oneoff_handle_timeout_event(
    vmctx: *mut VmContext,
    output_slice: &mut [wasm32::__wasi_event_t],
    nevents: wasm32::uintptr_t,
    timeout: Option<ClockEventData>,
) -> wasm32::__wasi_errno_t {
    if let Some(ClockEventData { userdata, .. }) = timeout {
        let output_event = host::__wasi_event_t {
            userdata,
            type_: wasm32::__WASI_EVENTTYPE_CLOCK,
            error: wasm32::__WASI_ESUCCESS,
            u: host::__wasi_event_t___wasi_event_u {
                fd_readwrite: host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                    nbytes: 0,
                    flags: 0,
                },
            },
        };
        output_slice[0] = enc_event(output_event);
        if let Err(e) = unsafe { enc_pointee(vmctx, nevents, 1) } {
            return enc_errno(e);
        }
    } else {
        // shouldn't happen
        if let Err(e) = unsafe { enc_pointee(vmctx, nevents, 0) } {
            return enc_errno(e);
        }
    }
    wasm32::__WASI_ESUCCESS
}

fn poll_oneoff_handle_fd_event<'t>(
    vmctx: *mut VmContext,
    output_slice: &mut [wasm32::__wasi_event_t],
    nevents: wasm32::uintptr_t,
    events: impl Iterator<Item = (&'t FdEventData, &'t nix::poll::PollFd)>,
) -> wasm32::__wasi_errno_t {
    let mut output_slice_cur = output_slice.iter_mut();
    let mut revents_count = 0;
    for (fd_event, poll_fd) in events {
        let revents = match poll_fd.revents() {
            Some(revents) => revents,
            None => continue,
        };
        let mut nbytes = 0;
        if fd_event.type_ == wasm32::__WASI_EVENTTYPE_FD_READ {
            let _ = unsafe { fionread(fd_event.fd, &mut nbytes) };
        }
        let output_event = if revents.contains(nix::poll::EventFlags::POLLNVAL) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: wasm32::__WASI_EBADF,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: 0,
                            flags: wasm32::__WASI_EVENT_FD_READWRITE_HANGUP,
                        },
                },
            }
        } else if revents.contains(nix::poll::EventFlags::POLLERR) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: wasm32::__WASI_EIO,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: 0,
                            flags: wasm32::__WASI_EVENT_FD_READWRITE_HANGUP,
                        },
                },
            }
        } else if revents.contains(nix::poll::EventFlags::POLLHUP) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: wasm32::__WASI_ESUCCESS,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: 0,
                            flags: wasm32::__WASI_EVENT_FD_READWRITE_HANGUP,
                        },
                },
            }
        } else if revents.contains(nix::poll::EventFlags::POLLIN)
            | revents.contains(nix::poll::EventFlags::POLLOUT)
        {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: wasm32::__WASI_ESUCCESS,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: nbytes as host::__wasi_filesize_t,
                            flags: 0,
                        },
                },
            }
        } else {
            continue;
        };
        *output_slice_cur.next().unwrap() = enc_event(output_event);
        revents_count += 1;
    }
    if let Err(e) = unsafe { enc_pointee(vmctx, nevents, revents_count) } {
        return enc_errno(e);
    }
    wasm32::__WASI_ESUCCESS
}

/// Normalizes a path to ensure that the target path is located under the directory provided.
///
/// This is a workaround for not having Capsicum support in the OS.
pub fn path_get<P: AsRef<OsStr>>(
    vmctx: &VmContext,
    dirfd: host::__wasi_fd_t,
    dirflags: host::__wasi_lookupflags_t,
    path: P,
    needed_base: host::__wasi_rights_t,
    needed_inheriting: host::__wasi_rights_t,
    needs_final_component: bool,
) -> Result<(RawFd, OsString), host::__wasi_errno_t> {
    use nix::errno::Errno;
    use nix::fcntl::{openat, readlinkat, OFlag};
    use nix::sys::stat::Mode;

    const MAX_SYMLINK_EXPANSIONS: usize = 128;

    /// close all the intermediate file descriptors, but make sure not to drop either the original
    /// dirfd or the one we return (which may be the same dirfd)
    fn ret_dir_success(dir_stack: &mut Vec<RawFd>) -> RawFd {
        let ret_dir = dir_stack.pop().expect("there is always a dirfd to return");
        if let Some(dirfds) = dir_stack.get(1..) {
            for dirfd in dirfds {
                nix::unistd::close(*dirfd).unwrap_or_else(|e| {
                    dbg!(e);
                });
            }
        }
        ret_dir
    }

    /// close all file descriptors other than the base directory, and return the errno for
    /// convenience with `return`
    fn ret_error(
        dir_stack: &mut Vec<RawFd>,
        errno: host::__wasi_errno_t,
    ) -> Result<(RawFd, OsString), host::__wasi_errno_t> {
        if let Some(dirfds) = dir_stack.get(1..) {
            for dirfd in dirfds {
                nix::unistd::close(*dirfd).unwrap_or_else(|e| {
                    dbg!(e);
                });
            }
        }
        Err(errno)
    }

    let ctx = (*vmctx).as_wasi_ctx();

    let dirfe = unsafe { (*ctx).get_fd_entry(dirfd, needed_base, needed_inheriting)? };

    // Stack of directory file descriptors. Index 0 always corresponds with the directory provided
    // to this function. Entering a directory causes a file descriptor to be pushed, while handling
    // ".." entries causes an entry to be popped. Index 0 cannot be popped, as this would imply
    // escaping the base directory.
    let mut dir_stack = vec![dirfe.fd_object.rawfd];

    // Stack of paths left to process. This is initially the `path` argument to this function, but
    // any symlinks we encounter are processed by pushing them on the stack.
    let mut path_stack = vec![path.as_ref().to_owned().into_vec()];

    // Track the number of symlinks we've expanded, so we can return `ELOOP` after too many.
    let mut symlink_expansions = 0;

    // Buffer to read links into; defined outside of the loop so we don't reallocate it constantly.
    let mut readlink_buf = vec![0u8; libc::PATH_MAX as usize + 1];

    // TODO: rewrite this using a custom posix path type, with a component iterator that respects
    // trailing slashes. This version does way too much allocation, and is way too fiddly.
    loop {
        let component = if let Some(cur_path) = path_stack.pop() {
            // eprintln!(
            //     "cur_path = {:?}",
            //     std::str::from_utf8(cur_path.as_slice()).unwrap()
            // );
            let mut split = cur_path.splitn(2, |&c| c == '/' as u8);
            let head = split.next();
            let tail = split.next();
            match (head, tail) {
                (None, _) => {
                    // split always returns at least a singleton iterator with an empty slice
                    panic!("unreachable");
                }
                // path is empty
                (Some([]), None) => {
                    return ret_error(&mut dir_stack, host::__WASI_ENOENT);
                }
                // path starts with `/`, is absolute
                (Some([]), Some(_)) => {
                    return ret_error(&mut dir_stack, host::__WASI_ENOTCAPABLE);
                }
                // the final component of the path with no trailing slash
                (Some(component), None) => component.to_vec(),
                (Some(component), Some(rest)) => {
                    if rest.iter().all(|&c| c == '/' as u8) {
                        // the final component of the path with trailing slashes; put one trailing
                        // slash back on
                        let mut component = component.to_vec();
                        component.push('/' as u8);
                        component
                    } else {
                        // non-final component; push the rest back on the stack
                        path_stack.push(rest.to_vec());
                        component.to_vec()
                    }
                }
            }
        } else {
            // if the path stack is ever empty, we return rather than going through the loop again
            panic!("unreachable");
        };

        // eprintln!(
        //     "component = {:?}",
        //     std::str::from_utf8(component.as_slice()).unwrap()
        // );

        match component.as_slice() {
            b"." => {
                // skip component
            }
            b".." => {
                // pop a directory
                let dirfd = dir_stack.pop().expect("dir_stack is never empty");

                // we're not allowed to pop past the original directory
                if dir_stack.is_empty() {
                    return ret_error(&mut dir_stack, host::__WASI_ENOTCAPABLE);
                } else {
                    nix::unistd::close(dirfd).unwrap_or_else(|e| {
                        dbg!(e);
                    });
                }
            }
            // should the component be a directory? it should if there is more path left to process, or
            // if it has a trailing slash and `needs_final_component` is not set
            component
                if !path_stack.is_empty()
                    || (component.ends_with(b"/") && !needs_final_component) =>
            {
                match openat(
                    *dir_stack.first().expect("dir_stack is never empty"),
                    component,
                    OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW,
                    Mode::empty(),
                ) {
                    Ok(new_dir) => {
                        dir_stack.push(new_dir);
                        continue;
                    }
                    Err(e)
                        if e.as_errno() == Some(Errno::ELOOP)
                            || e.as_errno() == Some(Errno::EMLINK) =>
                    {
                        // attempt symlink expansion
                        match readlinkat(
                            *dir_stack.last().expect("dir_stack is never empty"),
                            component,
                            readlink_buf.as_mut_slice(),
                        ) {
                            Ok(link_path) => {
                                symlink_expansions += 1;
                                if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                    return ret_error(&mut dir_stack, host::__WASI_ELOOP);
                                }

                                let mut link_path = link_path.as_bytes().to_vec();

                                // append a trailing slash if the component leading to it has one, so
                                // that we preserve any ENOTDIR that might come from trying to open a
                                // non-directory
                                if component.ends_with(b"/") {
                                    link_path.push('/' as u8);
                                }

                                path_stack.push(link_path);
                                continue;
                            }
                            Err(e) => {
                                return ret_error(
                                    &mut dir_stack,
                                    host::errno_from_nix(e.as_errno().unwrap()),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        return ret_error(
                            &mut dir_stack,
                            host::errno_from_nix(e.as_errno().unwrap()),
                        );
                    }
                }
            }
            // the final component
            component => {
                // if there's a trailing slash, or if `LOOKUP_SYMLINK_FOLLOW` is set, attempt
                // symlink expansion
                if component.ends_with(b"/") || (dirflags & host::__WASI_LOOKUP_SYMLINK_FOLLOW) != 0
                {
                    match readlinkat(
                        *dir_stack.last().expect("dir_stack is never empty"),
                        component,
                        readlink_buf.as_mut_slice(),
                    ) {
                        Ok(link_path) => {
                            symlink_expansions += 1;
                            if symlink_expansions > MAX_SYMLINK_EXPANSIONS {
                                return ret_error(&mut dir_stack, host::__WASI_ELOOP);
                            }

                            let mut link_path = link_path.as_bytes().to_vec();

                            // append a trailing slash if the component leading to it has one, so
                            // that we preserve any ENOTDIR that might come from trying to open a
                            // non-directory
                            if component.ends_with(b"/") {
                                link_path.push('/' as u8);
                            }

                            path_stack.push(link_path);
                            continue;
                        }
                        Err(e) => {
                            let errno = e.as_errno().unwrap();
                            if errno != Errno::EINVAL && errno != Errno::ENOENT {
                                // only return an error if this path is not actually a symlink
                                return ret_error(&mut dir_stack, host::errno_from_nix(errno));
                            }
                        }
                    }
                }

                // not a symlink, so we're done;
                return Ok((
                    ret_dir_success(&mut dir_stack),
                    OsStr::from_bytes(component).to_os_string(),
                ));
            }
        }

        if path_stack.is_empty() {
            // no further components to process. means we've hit a case like "." or "a/..", or if the
            // input path has trailing slashes and `needs_final_component` is not set
            return Ok((
                ret_dir_success(&mut dir_stack),
                OsStr::new(".").to_os_string(),
            ));
        } else {
            continue;
        }
    }
}

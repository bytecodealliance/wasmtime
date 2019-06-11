#![allow(non_camel_case_types)]
use crate::ctx::WasiCtx;
use crate::memory::*;
use crate::sys::host_impl;
use crate::sys::hostcalls_impl;
use crate::{host, wasm32};

use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
pub fn fd_close(wasi_ctx: &mut WasiCtx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);
    if let Some(fdent) = wasi_ctx.fds.get(&fd) {
        // can't close preopened files
        if fdent.preopen_path.is_some() {
            return wasm32::__WASI_ENOTSUP;
        }
    }
    if let Some(mut fdent) = wasi_ctx.fds.remove(&fd) {
        fdent.fd_object.needs_close = false;
        match hostcalls_impl::fd_close(fdent) {
            Ok(()) => wasm32::__WASI_ESUCCESS,
            Err(e) => enc_errno(e),
        }
    } else {
        wasm32::__WASI_EBADF
    }
}

#[wasi_common_cbindgen]
pub fn fd_datasync(wasi_ctx: &WasiCtx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let rights = host::__WASI_RIGHT_FD_DATASYNC;
    let fe = match wasi_ctx.get_fd_entry(host_fd, rights, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    match hostcalls_impl::fd_datasync(fe) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn fd_pread(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    offset: wasm32::__wasi_filesize_t,
    nread: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);
    let iovs = match dec_iovec_slice(memory, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_FD_READ;
    let fe = match wasi_ctx.get_fd_entry(fd, rights, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let offset = dec_filesize(offset);
    if offset > i64::max_value() as u64 {
        return wasm32::__WASI_EIO;
    }
    let buf_size = iovs.iter().map(|v| v.buf_len).sum();
    let mut buf = vec![0; buf_size];
    let host_nread = match hostcalls_impl::fd_pread(fe, &mut buf, offset) {
        Ok(host_nread) => host_nread,
        Err(e) => return enc_errno(e),
    };
    let mut buf_offset = 0;
    let mut left = host_nread;
    for iov in &iovs {
        if left == 0 {
            break;
        }
        let vec_len = std::cmp::min(iov.buf_len, left);
        unsafe { std::slice::from_raw_parts_mut(iov.buf as *mut u8, vec_len) }
            .copy_from_slice(&buf[buf_offset..buf_offset + vec_len]);
        buf_offset += vec_len;
        left -= vec_len;
    }
    enc_usize_byref(memory, nread, host_nread)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(enc_errno)
}

#[wasi_common_cbindgen]
pub fn fd_pwrite(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    offset: wasm32::__wasi_filesize_t,
    nwritten: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);
    let iovs = match dec_iovec_slice(memory, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_FD_READ;
    let fe = match wasi_ctx.get_fd_entry(fd, rights, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let offset = dec_filesize(offset);
    if offset > i64::max_value() as u64 {
        return wasm32::__WASI_EIO;
    }
    let buf_size = iovs.iter().map(|v| v.buf_len).sum();
    let mut buf = Vec::with_capacity(buf_size);
    for iov in &iovs {
        buf.extend_from_slice(unsafe {
            std::slice::from_raw_parts(iov.buf as *const u8, iov.buf_len)
        });
    }
    let host_nwritten = match hostcalls_impl::fd_pwrite(fe, &buf, offset) {
        Ok(host_nwritten) => host_nwritten,
        Err(e) => return enc_errno(e),
    };
    enc_usize_byref(memory, nwritten, host_nwritten)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(enc_errno)
}

#[wasi_common_cbindgen]
pub fn fd_read(
    wasi_ctx: &mut WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nread: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);
    let mut iovs = match dec_iovec_slice(memory, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };

    let fe = match wasi_ctx.get_fd_entry(fd, host::__WASI_RIGHT_FD_READ, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };

    let host_nread = match hostcalls_impl::fd_read(fe, &mut iovs) {
        Ok(host_nread) => host_nread,
        Err(e) => return enc_errno(e),
    };

    if host_nread == 0 {
        // we hit eof, so remove the fdentry from the context
        let mut fe = wasi_ctx.fds.remove(&fd).expect("file entry is still there");
        fe.fd_object.needs_close = false;
    }

    enc_usize_byref(memory, nread, host_nread)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(enc_errno)
}

#[wasi_common_cbindgen]
pub fn fd_renumber(
    wasi_ctx: &mut WasiCtx,
    from: wasm32::__wasi_fd_t,
    to: wasm32::__wasi_fd_t,
) -> wasm32::__wasi_errno_t {
    let from = dec_fd(from);
    let to = dec_fd(to);

    match hostcalls_impl::fd_renumber(wasi_ctx, from, to) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn fd_seek(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filedelta_t,
    whence: wasm32::__wasi_whence_t,
    newoffset: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);
    let offset = dec_filedelta(offset);
    let whence = dec_whence(whence);

    let rights = if offset == 0 && whence == host::__WASI_WHENCE_CUR {
        host::__WASI_RIGHT_FD_TELL
    } else {
        host::__WASI_RIGHT_FD_SEEK | host::__WASI_RIGHT_FD_TELL
    };
    let fe = match wasi_ctx.get_fd_entry(fd, rights, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let host_newoffset = match hostcalls_impl::fd_seek(fe, offset, whence) {
        Ok(host_newoffset) => host_newoffset,
        Err(e) => return enc_errno(e),
    };

    enc_filesize_byref(memory, newoffset, host_newoffset)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(enc_errno)
}

#[wasi_common_cbindgen]
pub fn fd_tell(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    newoffset: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);
    let rights = host::__WASI_RIGHT_FD_TELL;

    let fe = match wasi_ctx.get_fd_entry(fd, rights, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let host_offset = match hostcalls_impl::fd_tell(fe) {
        Ok(host_offset) => host_offset,
        Err(e) => return enc_errno(e),
    };

    enc_filesize_byref(memory, newoffset, host_offset)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(enc_errno)
}

#[wasi_common_cbindgen]
pub fn fd_fdstat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    fdstat_ptr: wasm32::uintptr_t, // *mut wasm32::__wasi_fdstat_t
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let mut host_fdstat = match dec_fdstat_byref(memory, fdstat_ptr) {
        Ok(host_fdstat) => host_fdstat,
        Err(e) => return enc_errno(e),
    };

    let errno = if let Some(fe) = wasi_ctx.fds.get(&host_fd) {
        host_fdstat.fs_filetype = fe.fd_object.ty;
        host_fdstat.fs_rights_base = fe.rights_base;
        host_fdstat.fs_rights_inheriting = fe.rights_inheriting;
        host_fdstat.fs_flags = match hostcalls_impl::fd_fdstat_get(fe) {
            Ok(flags) => flags,
            Err(e) => return enc_errno(e),
        };
        wasm32::__WASI_ESUCCESS
    } else {
        wasm32::__WASI_EBADF
    };

    if let Err(e) = enc_fdstat_byref(memory, fdstat_ptr, host_fdstat) {
        return enc_errno(e);
    }

    errno
}

#[wasi_common_cbindgen]
pub fn fd_fdstat_set_flags(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    fdflags: wasm32::__wasi_fdflags_t,
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let host_fdflags = dec_fdflags(fdflags);
    match wasi_ctx.fds.get(&host_fd) {
        Some(fe) => match hostcalls_impl::fd_fdstat_set_flags(fe, host_fdflags) {
            Ok(()) => wasm32::__WASI_ESUCCESS,
            Err(e) => enc_errno(e),
        },
        None => wasm32::__WASI_EBADF,
    }
}

#[wasi_common_cbindgen]
pub fn fd_fdstat_set_rights(
    wasi_ctx: &mut WasiCtx,
    fd: wasm32::__wasi_fd_t,
    fs_rights_base: wasm32::__wasi_rights_t,
    fs_rights_inheriting: wasm32::__wasi_rights_t,
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let fe = match wasi_ctx.fds.get_mut(&host_fd) {
        Some(fe) => fe,
        None => return wasm32::__WASI_EBADF,
    };
    if fe.rights_base & fs_rights_base != fs_rights_base
        || fe.rights_inheriting & fs_rights_inheriting != fs_rights_inheriting
    {
        return wasm32::__WASI_ENOTCAPABLE;
    }

    fe.rights_base = fs_rights_base;
    fe.rights_inheriting = fs_rights_inheriting;
    wasm32::__WASI_ESUCCESS
}

#[wasi_common_cbindgen]
pub fn fd_sync(wasi_ctx: &WasiCtx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let rights = host::__WASI_RIGHT_FD_SYNC;
    let fe = match wasi_ctx.get_fd_entry(host_fd, rights, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    match hostcalls_impl::fd_sync(fe) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn fd_write(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nwritten: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);
    let iovs = match dec_iovec_slice(memory, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };
    let fe = match wasi_ctx.get_fd_entry(fd, host::__WASI_RIGHT_FD_WRITE, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let host_nwritten = match hostcalls_impl::fd_write(fe, &iovs) {
        Ok(host_nwritten) => host_nwritten,
        Err(e) => return enc_errno(e),
    };

    enc_usize_byref(memory, nwritten, host_nwritten)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(enc_errno)
}

#[wasi_common_cbindgen]
pub fn fd_advise(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filesize_t,
    len: wasm32::__wasi_filesize_t,
    advice: wasm32::__wasi_advice_t,
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let rights = host::__WASI_RIGHT_FD_ADVISE;
    let fe = match wasi_ctx.get_fd_entry(host_fd, rights, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let advice = dec_advice(advice);
    let offset = dec_filesize(offset);
    let len = dec_filesize(len);

    match hostcalls_impl::fd_advise(fe, advice, offset, len) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn fd_allocate(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filesize_t,
    len: wasm32::__wasi_filesize_t,
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let rights = host::__WASI_RIGHT_FD_ALLOCATE;
    let fe = match wasi_ctx.get_fd_entry(host_fd, rights, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let offset = dec_filesize(offset);
    let len = dec_filesize(len);

    match hostcalls_impl::fd_allocate(fe, offset, len) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn path_create_directory(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let dirfd = dec_fd(dirfd);
    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len) {
        Ok(slice) => host_impl::path_from_raw(slice),
        Err(e) => return enc_errno(e),
    };

    match hostcalls_impl::path_create_directory(wasi_ctx, dirfd, &path) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn path_link(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    old_dirfd: wasm32::__wasi_fd_t,
    _old_flags: wasm32::__wasi_lookupflags_t,
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    new_dirfd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let old_dirfd = dec_fd(old_dirfd);
    let new_dirfd = dec_fd(new_dirfd);
    let old_path = match dec_slice_of::<u8>(memory, old_path_ptr, old_path_len) {
        Ok(slice) => host_impl::path_from_raw(slice),
        Err(e) => return enc_errno(e),
    };
    let new_path = match dec_slice_of::<u8>(memory, new_path_ptr, new_path_len) {
        Ok(slice) => host_impl::path_from_raw(slice),
        Err(e) => return enc_errno(e),
    };

    match hostcalls_impl::path_link(
        wasi_ctx,
        old_dirfd,
        new_dirfd,
        &old_path,
        &new_path,
        host::__WASI_RIGHT_PATH_LINK_SOURCE,
        host::__WASI_RIGHT_PATH_LINK_TARGET,
    ) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn path_open(
    wasi_ctx: &mut WasiCtx,
    memory: &mut [u8],
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
            | host::__WASI_RIGHT_FD_FILESTAT_SET_SIZE)
        != 0;

    // which rights are needed on the dirfd?
    let needed_base = host::__WASI_RIGHT_PATH_OPEN;
    let needed_inheriting = fs_rights_base | fs_rights_inheriting;

    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len) {
        Ok(slice) => host_impl::path_from_raw(slice),
        Err(e) => return enc_errno(e),
    };

    match hostcalls_impl::path_open(
        wasi_ctx,
        dirfd,
        dirflags,
        &path,
        oflags,
        read,
        write,
        needed_base,
        needed_inheriting,
        fs_flags,
    ) {
        Ok(fe) => {
            let guest_fd = match wasi_ctx.insert_fd_entry(fe) {
                Ok(fd) => fd,
                Err(e) => return enc_errno(e),
            };

            enc_fd_byref(memory, fd_out_ptr, guest_fd)
                .map(|_| wasm32::__WASI_ESUCCESS)
                .unwrap_or_else(enc_errno)
        }
        Err(e) => {
            if let Err(e) = enc_fd_byref(memory, fd_out_ptr, wasm32::__wasi_fd_t::max_value()) {
                return enc_errno(e);
            }

            enc_errno(e)
        }
    }
}

#[wasi_common_cbindgen]
pub fn fd_readdir(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    buf: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
    cookie: wasm32::__wasi_dircookie_t,
    buf_used: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    match enc_usize_byref(memory, buf_used, 0) {
        Ok(_) => {}
        Err(e) => return enc_errno(e),
    };
    let fd = dec_fd(fd);
    let rights = host::__WASI_RIGHT_FD_READDIR;
    let fe = match wasi_ctx.get_fd_entry(fd, rights, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let host_buf = match dec_slice_of_mut::<u8>(memory, buf, buf_len) {
        Ok(host_buf) => host_buf,
        Err(e) => return enc_errno(e),
    };
    let cookie = dec_dircookie(cookie);

    let host_bufused = match hostcalls_impl::fd_readdir(fe, host_buf, cookie) {
        Ok(host_bufused) => host_bufused,
        Err(e) => return enc_errno(e),
    };

    enc_usize_byref(memory, buf_used, host_bufused)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(enc_errno)
}

#[wasi_common_cbindgen]
pub fn path_readlink(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    buf_ptr: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
    buf_used: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    match enc_usize_byref(memory, buf_used, 0) {
        Ok(_) => {}
        Err(e) => return enc_errno(e),
    };
    let dirfd = dec_fd(dirfd);
    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len) {
        Ok(slice) => host_impl::path_from_raw(slice).to_owned(),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_READLINK;
    let mut buf = match dec_slice_of_mut::<u8>(memory, buf_ptr, buf_len) {
        Ok(slice) => slice,
        Err(e) => return enc_errno(e),
    };
    let host_bufused =
        match hostcalls_impl::path_readlink(wasi_ctx, dirfd, path.as_os_str(), rights, &mut buf) {
            Ok(host_bufused) => host_bufused,
            Err(e) => return enc_errno(e),
        };
    match enc_usize_byref(memory, buf_used, host_bufused) {
        Ok(_) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn path_rename(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    old_dirfd: wasm32::__wasi_fd_t,
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    new_dirfd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let old_dirfd = dec_fd(old_dirfd);
    let new_dirfd = dec_fd(new_dirfd);
    let old_path = match dec_slice_of::<u8>(memory, old_path_ptr, old_path_len) {
        Ok(slice) => host_impl::path_from_raw(slice),
        Err(e) => return enc_errno(e),
    };
    let new_path = match dec_slice_of::<u8>(memory, new_path_ptr, new_path_len) {
        Ok(slice) => host_impl::path_from_raw(slice),
        Err(e) => return enc_errno(e),
    };
    let old_rights = host::__WASI_RIGHT_PATH_RENAME_SOURCE;
    let new_rights = host::__WASI_RIGHT_PATH_RENAME_TARGET;

    match hostcalls_impl::path_rename(
        wasi_ctx, old_dirfd, &old_path, old_rights, new_dirfd, &new_path, new_rights,
    ) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn fd_filestat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    filestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let fe = match wasi_ctx.fds.get(&host_fd) {
        Some(fe) => fe,
        None => return wasm32::__WASI_EBADF,
    };

    let host_filestat = match hostcalls_impl::fd_filestat_get(fe) {
        Ok(fstat) => fstat,
        Err(e) => return enc_errno(e),
    };

    match enc_filestat_byref(memory, filestat_ptr, host_filestat) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn fd_filestat_set_times(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    st_atim: wasm32::__wasi_timestamp_t,
    st_mtim: wasm32::__wasi_timestamp_t,
    fst_flags: wasm32::__wasi_fstflags_t,
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let rights = host::__WASI_RIGHT_FD_FILESTAT_SET_TIMES;
    let fe = match wasi_ctx.get_fd_entry(host_fd, rights, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let st_atim = dec_timestamp(st_atim);
    let st_mtim = dec_timestamp(st_mtim);
    let fst_flags = dec_fstflags(fst_flags);

    match hostcalls_impl::fd_filestat_set_times(fe, st_atim, st_mtim, fst_flags) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn fd_filestat_set_size(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    st_size: wasm32::__wasi_filesize_t,
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let rights = host::__WASI_RIGHT_FD_FILESTAT_SET_SIZE;
    let fe = match wasi_ctx.get_fd_entry(host_fd, rights, 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };
    let st_size = dec_filesize(st_size);
    if st_size > i64::max_value() as u64 {
        return wasm32::__WASI_E2BIG;
    }

    match hostcalls_impl::fd_filestat_set_size(fe, st_size) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn path_filestat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    filestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let dirfd = dec_fd(dirfd);
    let dirflags = dec_lookupflags(dirflags);
    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len) {
        Ok(slice) => host_impl::path_from_raw(slice),
        Err(e) => return enc_errno(e),
    };
    let host_filestat = match hostcalls_impl::path_filestat_get(wasi_ctx, dirfd, dirflags, &path) {
        Ok(host_filestat) => host_filestat,
        Err(e) => return enc_errno(e),
    };

    match enc_filestat_byref(memory, filestat_ptr, host_filestat) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn path_filestat_set_times(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    st_atim: wasm32::__wasi_timestamp_t,
    st_mtim: wasm32::__wasi_timestamp_t,
    fst_flags: wasm32::__wasi_fstflags_t,
) -> wasm32::__wasi_errno_t {
    let dirfd = dec_fd(dirfd);
    let dirflags = dec_lookupflags(dirflags);
    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len) {
        Ok(slice) => host_impl::path_from_raw(slice),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_FILESTAT_SET_TIMES;
    let st_atim = dec_timestamp(st_atim);
    let st_mtim = dec_timestamp(st_mtim);
    let fst_flags = dec_fstflags(fst_flags);

    match hostcalls_impl::path_filestat_set_times(
        wasi_ctx, dirfd, dirflags, &path, rights, st_atim, st_mtim, fst_flags,
    ) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn path_symlink(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    dirfd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let dirfd = dec_fd(dirfd);
    let old_path = match dec_slice_of::<u8>(memory, old_path_ptr, old_path_len) {
        Ok(slice) => host_impl::path_from_raw(slice),
        Err(e) => return enc_errno(e),
    };
    let new_path = match dec_slice_of::<u8>(memory, new_path_ptr, new_path_len) {
        Ok(slice) => host_impl::path_from_raw(slice),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_SYMLINK;

    match hostcalls_impl::path_symlink(wasi_ctx, dirfd, rights, &old_path, &new_path) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn path_unlink_file(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let dirfd = dec_fd(dirfd);
    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len) {
        Ok(slice) => host_impl::path_from_raw(slice),
        Err(e) => return enc_errno(e),
    };

    match hostcalls_impl::path_unlink_file(
        wasi_ctx,
        dirfd,
        &path,
        host::__WASI_RIGHT_PATH_UNLINK_FILE,
    ) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn path_remove_directory(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let dirfd = dec_fd(dirfd);
    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len) {
        Ok(slice) => host_impl::path_from_raw(slice),
        Err(e) => return enc_errno(e),
    };
    let rights = host::__WASI_RIGHT_PATH_REMOVE_DIRECTORY;

    match hostcalls_impl::path_remove_directory(wasi_ctx, dirfd, &path, rights) {
        Ok(()) => wasm32::__WASI_ESUCCESS,
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn fd_prestat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    prestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);
    // TODO: is this the correct right for this?
    match wasi_ctx.get_fd_entry(fd, host::__WASI_RIGHT_PATH_OPEN.into(), 0) {
        Ok(fe) => {
            if let Some(po_path) = &fe.preopen_path {
                if fe.fd_object.ty != host::__WASI_FILETYPE_DIRECTORY {
                    return wasm32::__WASI_ENOTDIR;
                }
                enc_prestat_byref(
                    memory,
                    prestat_ptr,
                    host::__wasi_prestat_t {
                        pr_type: host::__WASI_PREOPENTYPE_DIR,
                        u: host::__wasi_prestat_t___wasi_prestat_u {
                            dir: host::__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t {
                                pr_name_len: host_impl::path_to_raw(po_path).len(),
                            },
                        },
                    },
                )
                .map(|_| wasm32::__WASI_ESUCCESS)
                .unwrap_or_else(|e| e)
            } else {
                wasm32::__WASI_ENOTSUP
            }
        }
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn fd_prestat_dir_name(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);

    match wasi_ctx.get_fd_entry(fd, host::__WASI_RIGHT_PATH_OPEN.into(), 0) {
        Ok(fe) => {
            if let Some(po_path) = &fe.preopen_path {
                if fe.fd_object.ty != host::__WASI_FILETYPE_DIRECTORY {
                    return wasm32::__WASI_ENOTDIR;
                }
                let path_bytes = host_impl::path_to_raw(po_path);
                if path_bytes.len() > dec_usize(path_len) {
                    return wasm32::__WASI_ENAMETOOLONG;
                }
                enc_slice_of(memory, &path_bytes, path_ptr)
                    .map(|_| wasm32::__WASI_ESUCCESS)
                    .unwrap_or_else(|e| e)
            } else {
                wasm32::__WASI_ENOTSUP
            }
        }
        Err(e) => enc_errno(e),
    }
}

#![allow(non_camel_case_types)]
use super::return_enc_errno;
use crate::ctx::WasiCtx;
use crate::fdentry::{Descriptor, FdEntry};
use crate::memory::*;
use crate::sys::{errno_from_host, host_impl, hostcalls_impl};
use crate::{host, wasm32};
use log::trace;
use std::convert::identity;
use std::io::{self, Read, Seek, SeekFrom, Write};

use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
pub fn fd_close(wasi_ctx: &mut WasiCtx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    trace!("fd_close(fd={:?})", fd);

    let fd = dec_fd(fd);
    if let Some(fdent) = wasi_ctx.fds.get(&fd) {
        // can't close preopened files
        if fdent.preopen_path.is_some() {
            return return_enc_errno(host::__WASI_ENOTSUP);
        }
    }
    let ret = if let Some(mut fe) = wasi_ctx.fds.remove(&fd) {
        fe.fd_object.needs_close = true;
        host::__WASI_ESUCCESS
    } else {
        host::__WASI_EBADF
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_datasync(wasi_ctx: &WasiCtx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    trace!("fd_datasync(fd={:?})", fd);

    let fd = dec_fd(fd);
    let fd = match wasi_ctx
        .get_fd_entry(fd, host::__WASI_RIGHT_FD_DATASYNC, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };
    let ret = match fd.sync_data() {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(err) => err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host),
    };

    return_enc_errno(ret)
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
    trace!(
        "fd_pread(fd={:?}, iovs_ptr={:#x?}, iovs_len={:?}, offset={}, nread={:#x?})",
        fd,
        iovs_ptr,
        iovs_len,
        offset,
        nread
    );

    let fd = dec_fd(fd);
    let iovs = match dec_iovec_slice(memory, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return return_enc_errno(e),
    };
    let fd = match wasi_ctx
        .get_fd_entry(fd, host::__WASI_RIGHT_FD_READ, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let offset = dec_filesize(offset);
    if offset > i64::max_value() as u64 {
        return return_enc_errno(host::__WASI_EIO);
    }
    let buf_size = iovs.iter().map(|v| v.buf_len).sum();
    let mut buf = vec![0; buf_size];
    let host_nread = match hostcalls_impl::fd_pread(fd, &mut buf, offset) {
        Ok(host_nread) => host_nread,
        Err(e) => return return_enc_errno(e),
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

    trace!("     | *nread={:?}", host_nread);

    let ret = enc_usize_byref(memory, nread, host_nread)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
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
    trace!(
        "fd_pwrite(fd={:?}, iovs_ptr={:#x?}, iovs_len={:?}, offset={}, nwritten={:#x?})",
        fd,
        iovs_ptr,
        iovs_len,
        offset,
        nwritten
    );

    let fd = dec_fd(fd);
    let iovs = match dec_iovec_slice(memory, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return return_enc_errno(e),
    };
    let fd = match wasi_ctx
        .get_fd_entry(fd, host::__WASI_RIGHT_FD_READ, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let offset = dec_filesize(offset);
    if offset > i64::max_value() as u64 {
        return return_enc_errno(host::__WASI_EIO);
    }
    let buf_size = iovs.iter().map(|v| v.buf_len).sum();
    let mut buf = Vec::with_capacity(buf_size);
    for iov in &iovs {
        buf.extend_from_slice(unsafe {
            std::slice::from_raw_parts(iov.buf as *const u8, iov.buf_len)
        });
    }
    let host_nwritten = match hostcalls_impl::fd_pwrite(fd, &buf, offset) {
        Ok(host_nwritten) => host_nwritten,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | *nwritten={:?}", host_nwritten);

    let ret = enc_usize_byref(memory, nwritten, host_nwritten)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
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
    trace!(
        "fd_read(fd={:?}, iovs_ptr={:#x?}, iovs_len={:?}, nread={:#x?})",
        fd,
        iovs_ptr,
        iovs_len,
        nread
    );

    let fd = dec_fd(fd);
    let mut iovs = match dec_iovec_slice(memory, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return return_enc_errno(e),
    };
    let fe = match wasi_ctx.get_fd_entry_mut(fd, host::__WASI_RIGHT_FD_READ, 0) {
        Ok(fe) => fe,
        Err(e) => return return_enc_errno(e),
    };
    let mut iovs: Vec<io::IoSliceMut> = iovs
        .iter_mut()
        .map(|vec| unsafe { host::iovec_to_host_mut(vec) })
        .collect();

    let maybe_host_nread = match &mut *fe.fd_object.descriptor {
        Descriptor::File(f) => f.read_vectored(&mut iovs),
        Descriptor::Stdin => io::stdin().lock().read_vectored(&mut iovs),
        _ => return return_enc_errno(host::__WASI_EBADF),
    };

    let host_nread = match maybe_host_nread {
        Ok(host_nread) => host_nread,
        Err(err) => {
            let err = err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host);
            return return_enc_errno(err);
        }
    };

    trace!("     | *nread={:?}", host_nread);

    let ret = enc_usize_byref(memory, nread, host_nread)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_renumber(
    wasi_ctx: &mut WasiCtx,
    from: wasm32::__wasi_fd_t,
    to: wasm32::__wasi_fd_t,
) -> wasm32::__wasi_errno_t {
    trace!("fd_renumber(from={:?}, to={:?})", from, to);

    let from = dec_fd(from);
    let to = dec_fd(to);

    if !wasi_ctx.contains_fd_entry(from) || !wasi_ctx.contains_fd_entry(to) {
        return return_enc_errno(host::__WASI_EBADF);
    }

    // Don't allow renumbering over a pre-opened resource.
    // TODO: Eventually, we do want to permit this, once libpreopen in
    // userspace is capable of removing entries from its tables as well.
    if wasi_ctx.fds[&from].preopen_path.is_some() || wasi_ctx.fds[&to].preopen_path.is_some() {
        return return_enc_errno(host::__WASI_ENOTSUP);
    }

    // check if stdio fds
    // TODO should we renumber stdio fds?
    if !wasi_ctx.fds[&from].fd_object.descriptor.is_file()
        || !wasi_ctx.fds[&to].fd_object.descriptor.is_file()
    {
        return return_enc_errno(host::__WASI_EBADF);
    }

    let fe_from_dup = match wasi_ctx.fds[&from]
        .fd_object
        .descriptor
        .as_file()
        .and_then(FdEntry::duplicate)
    {
        Ok(fe) => fe,
        Err(e) => return return_enc_errno(e),
    };

    wasi_ctx.fds.insert(to, fe_from_dup);
    wasi_ctx.fds.remove(&from);

    return_enc_errno(host::__WASI_ESUCCESS)
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
    trace!(
        "fd_seek(fd={:?}, offset={:?}, whence={}, newoffset={:#x?})",
        fd,
        offset,
        wasm32::whence_to_str(whence),
        newoffset
    );

    let fd = dec_fd(fd);
    let offset = dec_filedelta(offset);
    let whence = dec_whence(whence);

    let rights = if offset == 0 && whence == host::__WASI_WHENCE_CUR {
        host::__WASI_RIGHT_FD_TELL
    } else {
        host::__WASI_RIGHT_FD_SEEK | host::__WASI_RIGHT_FD_TELL
    };
    let mut fd = match wasi_ctx
        .get_fd_entry(fd, rights, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let pos = match whence {
        host::__WASI_WHENCE_CUR => SeekFrom::Current(offset),
        host::__WASI_WHENCE_END => SeekFrom::End(offset),
        host::__WASI_WHENCE_SET => SeekFrom::Start(offset as u64),
        _ => return return_enc_errno(host::__WASI_EINVAL),
    };
    let host_newoffset = match fd
        .seek(pos)
        .map_err(|err| err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
    {
        Ok(offset) => offset,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | *newoffset={:?}", host_newoffset);

    let ret = enc_filesize_byref(memory, newoffset, host_newoffset)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_tell(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    newoffset: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    trace!("fd_tell(fd={:?}, newoffset={:#x?})", fd, newoffset);

    let fd = dec_fd(fd);
    let mut fd = match wasi_ctx
        .get_fd_entry(fd, host::__WASI_RIGHT_FD_TELL, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let host_offset = match fd
        .seek(SeekFrom::Current(0))
        .map_err(|err| err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
    {
        Ok(offset) => offset,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | *newoffset={:?}", host_offset);

    let ret = enc_filesize_byref(memory, newoffset, host_offset)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_fdstat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    fdstat_ptr: wasm32::uintptr_t, // *mut wasm32::__wasi_fdstat_t
) -> wasm32::__wasi_errno_t {
    trace!("fd_fdstat_get(fd={:?}, fdstat_ptr={:#x?})", fd, fdstat_ptr);

    let mut fdstat = match dec_fdstat_byref(memory, fdstat_ptr) {
        Ok(fdstat) => fdstat,
        Err(e) => return return_enc_errno(e),
    };
    let fd = dec_fd(fd);
    let fe = match wasi_ctx.get_fd_entry(fd, 0, 0) {
        Ok(fe) => fe,
        Err(e) => return return_enc_errno(e),
    };
    let fd = match fe.fd_object.descriptor.as_file() {
        Ok(fd) => fd,
        Err(e) => return return_enc_errno(e),
    };

    let fs_flags = match hostcalls_impl::fd_fdstat_get(fd) {
        Ok(flags) => flags,
        Err(e) => return return_enc_errno(e),
    };

    fdstat.fs_filetype = fe.fd_object.file_type;
    fdstat.fs_rights_base = fe.rights_base;
    fdstat.fs_rights_inheriting = fe.rights_inheriting;
    fdstat.fs_flags = fs_flags;

    trace!("     | *buf={:?}", fdstat);

    let ret = enc_fdstat_byref(memory, fdstat_ptr, fdstat)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_fdstat_set_flags(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    fdflags: wasm32::__wasi_fdflags_t,
) -> wasm32::__wasi_errno_t {
    trace!("fd_fdstat_set_flags(fd={:?}, fdflags={:#x?})", fd, fdflags);

    let fdflags = dec_fdflags(fdflags);
    let fd = dec_fd(fd);
    let fd = match wasi_ctx
        .get_fd_entry(fd, 0, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let ret = hostcalls_impl::fd_fdstat_set_flags(fd, fdflags)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_fdstat_set_rights(
    wasi_ctx: &mut WasiCtx,
    fd: wasm32::__wasi_fd_t,
    fs_rights_base: wasm32::__wasi_rights_t,
    fs_rights_inheriting: wasm32::__wasi_rights_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "fd_fdstat_set_rights(fd={:?}, fs_rights_base={:#x?}, fs_rights_inheriting={:#x?})",
        fd,
        fs_rights_base,
        fs_rights_inheriting
    );

    let fd = dec_fd(fd);
    let fe = match wasi_ctx.fds.get_mut(&fd) {
        Some(fe) => fe,
        None => return return_enc_errno(host::__WASI_EBADF),
    };
    if fe.rights_base & fs_rights_base != fs_rights_base
        || fe.rights_inheriting & fs_rights_inheriting != fs_rights_inheriting
    {
        return return_enc_errno(host::__WASI_ENOTCAPABLE);
    }
    fe.rights_base = fs_rights_base;
    fe.rights_inheriting = fs_rights_inheriting;

    return_enc_errno(host::__WASI_ESUCCESS)
}

#[wasi_common_cbindgen]
pub fn fd_sync(wasi_ctx: &WasiCtx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    trace!("fd_sync(fd={:?})", fd);

    let fd = dec_fd(fd);
    let fd = match wasi_ctx
        .get_fd_entry(fd, host::__WASI_RIGHT_FD_SYNC, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };
    let ret = match fd.sync_all() {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(err) => err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host),
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_write(
    wasi_ctx: &mut WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nwritten: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "fd_write(fd={:?}, iovs_ptr={:#x?}, iovs_len={:?}, nwritten={:#x?})",
        fd,
        iovs_ptr,
        iovs_len,
        nwritten
    );

    let fd = dec_fd(fd);
    let iovs = match dec_iovec_slice(memory, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return return_enc_errno(e),
    };
    let fe = match wasi_ctx.get_fd_entry_mut(fd, host::__WASI_RIGHT_FD_WRITE, 0) {
        Ok(fe) => fe,
        Err(e) => return return_enc_errno(e),
    };
    let iovs: Vec<io::IoSlice> = iovs
        .iter()
        .map(|vec| unsafe { host::iovec_to_host(vec) })
        .collect();

    let maybe_host_nwritten = match &mut *fe.fd_object.descriptor {
        Descriptor::File(f) => f.write_vectored(&iovs),
        Descriptor::Stdin => return return_enc_errno(host::__WASI_EBADF),
        Descriptor::Stdout => io::stdout().lock().write_vectored(&iovs),
        Descriptor::Stderr => io::stderr().lock().write_vectored(&iovs),
    };

    let host_nwritten = match maybe_host_nwritten {
        Ok(host_nwritten) => host_nwritten,
        Err(err) => {
            let err = err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host);
            return return_enc_errno(err);
        }
    };

    trace!("     | *nwritten={:?}", host_nwritten);

    let ret = enc_usize_byref(memory, nwritten, host_nwritten)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_advise(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filesize_t,
    len: wasm32::__wasi_filesize_t,
    advice: wasm32::__wasi_advice_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "fd_advise(fd={:?}, offset={}, len={}, advice={:?})",
        fd,
        offset,
        len,
        advice
    );

    let fd = dec_fd(fd);
    let advice = dec_advice(advice);
    let offset = dec_filesize(offset);
    let len = dec_filesize(len);
    let fd = match wasi_ctx
        .get_fd_entry(fd, host::__WASI_RIGHT_FD_ADVISE, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let ret = match hostcalls_impl::fd_advise(fd, advice, offset, len) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_allocate(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filesize_t,
    len: wasm32::__wasi_filesize_t,
) -> wasm32::__wasi_errno_t {
    trace!("fd_allocate(fd={:?}, offset={}, len={})", fd, offset, len);

    let fd = dec_fd(fd);
    let offset = dec_filesize(offset);
    let len = dec_filesize(len);
    let fd = match wasi_ctx
        .get_fd_entry(fd, host::__WASI_RIGHT_FD_ALLOCATE, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let metadata = match fd
        .metadata()
        .map_err(|err| err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
    {
        Ok(metadata) => metadata,
        Err(e) => return return_enc_errno(e),
    };
    let current_size = metadata.len();
    let wanted_size = match offset.checked_add(len) {
        Some(wanted_size) => wanted_size,
        None => return return_enc_errno(host::__WASI_E2BIG),
    };
    if wanted_size > i64::max_value() as u64 {
        return return_enc_errno(host::__WASI_E2BIG);
    }

    if wanted_size > current_size {
        if let Err(e) = fd
            .set_len(wanted_size)
            .map_err(|err| err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
        {
            return return_enc_errno(e);
        }
    }

    return_enc_errno(host::__WASI_ESUCCESS)
}

#[wasi_common_cbindgen]
pub fn path_create_directory(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "path_create_directory(dirfd={:?}, path_ptr={:#x?}, path_len={})",
        dirfd,
        path_ptr,
        path_len,
    );

    let dirfd = dec_fd(dirfd);
    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len).and_then(host::path_from_slice)
    {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | (path_ptr,path_len)='{}'", path);

    let rights = host::__WASI_RIGHT_PATH_OPEN | host::__WASI_RIGHT_PATH_CREATE_DIRECTORY;
    let dirfd = match wasi_ctx
        .get_fd_entry(dirfd, rights, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let ret = match hostcalls_impl::path_create_directory(dirfd, path) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn path_link(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    old_dirfd: wasm32::__wasi_fd_t,
    old_flags: wasm32::__wasi_lookupflags_t,
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    new_dirfd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "path_link(old_dirfd={:?}, old_flags={:?}, old_path_ptr={:#x?}, old_path_len={}, new_dirfd={:?}, new_path_ptr={:#x?}, new_path_len={})",
        old_dirfd,
        old_flags,
        old_path_ptr,
        old_path_len,
        new_dirfd,
        new_path_ptr,
        new_path_len,
    );

    let old_dirfd = dec_fd(old_dirfd);
    let new_dirfd = dec_fd(new_dirfd);
    let old_path = match dec_slice_of::<u8>(memory, old_path_ptr, old_path_len)
        .and_then(host::path_from_slice)
    {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };
    let new_path = match dec_slice_of::<u8>(memory, new_path_ptr, new_path_len)
        .and_then(host::path_from_slice)
    {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | (old_path_ptr,old_path_len)='{}'", old_path);
    trace!("     | (new_path_ptr,new_path_len)='{}'", new_path);

    let old_dirfd = match wasi_ctx
        .get_fd_entry(old_dirfd, host::__WASI_RIGHT_PATH_LINK_SOURCE, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };
    let new_dirfd = match wasi_ctx
        .get_fd_entry(new_dirfd, host::__WASI_RIGHT_PATH_LINK_TARGET, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let ret = match hostcalls_impl::path_link(old_dirfd, new_dirfd, old_path, new_path) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
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
    trace!(
        "path_open(dirfd={:?}, dirflags={:?}, path_ptr={:#x?}, path_len={:?}, oflags={:#x?}, fs_rights_base={:#x?}, fs_rights_inheriting={:#x?}, fs_flags={:#x?}, fd_out_ptr={:#x?})",
        dirfd,
        dirflags,
        path_ptr,
        path_len,
        oflags,
        fs_rights_base,
        fs_rights_inheriting,
        fs_flags,
        fd_out_ptr
    );

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

    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len).and_then(host::path_from_slice)
    {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | (path_ptr,path_len)='{}'", path);

    let ret = match hostcalls_impl::path_open(
        wasi_ctx,
        dirfd,
        dirflags,
        path,
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
                Err(e) => return return_enc_errno(e),
            };

            trace!("     | *fd={:?}", guest_fd);

            enc_fd_byref(memory, fd_out_ptr, guest_fd)
                .map(|_| host::__WASI_ESUCCESS)
                .unwrap_or_else(identity)
        }
        Err(e) => {
            if let Err(e) = enc_fd_byref(memory, fd_out_ptr, wasm32::__wasi_fd_t::max_value()) {
                return return_enc_errno(e);
            }

            e
        }
    };

    return_enc_errno(ret)
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
    trace!(
        "fd_readdir(fd={:?}, buf={:#x?}, buf_len={}, cookie={:#x?}, buf_used={:#x?})",
        fd,
        buf,
        buf_len,
        cookie,
        buf_used,
    );

    match enc_usize_byref(memory, buf_used, 0) {
        Ok(_) => {}
        Err(e) => return return_enc_errno(e),
    };
    let fd = dec_fd(fd);
    let fd = match wasi_ctx
        .get_fd_entry(fd, host::__WASI_RIGHT_FD_READDIR, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };
    let host_buf = match dec_slice_of_mut::<u8>(memory, buf, buf_len) {
        Ok(host_buf) => host_buf,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | (buf,buf_len)={:?}", host_buf);

    let cookie = dec_dircookie(cookie);

    let host_bufused = match hostcalls_impl::fd_readdir(fd, host_buf, cookie) {
        Ok(host_bufused) => host_bufused,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | *buf_used={:?}", host_bufused);

    let ret = enc_usize_byref(memory, buf_used, host_bufused)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
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
    trace!(
        "path_readlink(dirfd={:?}, path_ptr={:#x?}, path_len={:?}, buf_ptr={:#x?}, buf_len={}, buf_used={:#x?})",
        dirfd,
        path_ptr,
        path_len,
        buf_ptr,
        buf_len,
        buf_used,
    );

    match enc_usize_byref(memory, buf_used, 0) {
        Ok(_) => {}
        Err(e) => return return_enc_errno(e),
    };
    let dirfd = dec_fd(dirfd);
    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len).and_then(host::path_from_vec) {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | (path_ptr,path_len)='{}'", &path);

    let dirfd = match wasi_ctx
        .get_fd_entry(dirfd, host::__WASI_RIGHT_PATH_READLINK, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let mut buf = match dec_slice_of_mut::<u8>(memory, buf_ptr, buf_len) {
        Ok(slice) => slice,
        Err(e) => return return_enc_errno(e),
    };
    let host_bufused = match hostcalls_impl::path_readlink(dirfd, &path, &mut buf) {
        Ok(host_bufused) => host_bufused,
        Err(e) => return return_enc_errno(e),
    };
    trace!("     | (buf_ptr,*buf_used)={:?}", buf);
    trace!("     | *buf_used={:?}", host_bufused);

    let ret = match enc_usize_byref(memory, buf_used, host_bufused) {
        Ok(_) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
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
    trace!(
        "path_rename(old_dirfd={:?}, old_path_ptr={:#x?}, old_path_len={:?}, new_dirfd={:?}, new_path_ptr={:#x?}, new_path_len={:?})",
        old_dirfd,
        old_path_ptr,
        old_path_len,
        new_dirfd,
        new_path_ptr,
        new_path_len,
    );

    let old_dirfd = dec_fd(old_dirfd);
    let new_dirfd = dec_fd(new_dirfd);
    let old_path = match dec_slice_of::<u8>(memory, old_path_ptr, old_path_len)
        .and_then(host::path_from_slice)
    {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };
    let new_path = match dec_slice_of::<u8>(memory, new_path_ptr, new_path_len)
        .and_then(host::path_from_slice)
    {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | (old_path_ptr,old_path_len)='{}'", old_path);
    trace!("     | (new_path_ptr,new_path_len)='{}'", new_path);

    let old_dirfd = match wasi_ctx
        .get_fd_entry(old_dirfd, host::__WASI_RIGHT_PATH_RENAME_SOURCE, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };
    let new_dirfd = match wasi_ctx
        .get_fd_entry(new_dirfd, host::__WASI_RIGHT_PATH_RENAME_TARGET, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let ret = match hostcalls_impl::path_rename(old_dirfd, old_path, new_dirfd, new_path) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_filestat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    filestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "fd_filestat_get(fd={:?}, filestat_ptr={:#x?})",
        fd,
        filestat_ptr
    );

    let fd = dec_fd(fd);
    let fd = match wasi_ctx
        .get_fd_entry(fd, 0, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let host_filestat = match hostcalls_impl::fd_filestat_get(fd) {
        Ok(fstat) => fstat,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | *filestat_ptr={:?}", host_filestat);

    let ret = match enc_filestat_byref(memory, filestat_ptr, host_filestat) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_filestat_set_times(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    st_atim: wasm32::__wasi_timestamp_t,
    st_mtim: wasm32::__wasi_timestamp_t,
    fst_flags: wasm32::__wasi_fstflags_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "fd_filestat_set_times(fd={:?}, st_atim={}, st_mtim={}, fst_flags={:#x?})",
        fd,
        st_atim,
        st_mtim,
        fst_flags
    );

    let fd = dec_fd(fd);
    let fd = match wasi_ctx
        .get_fd_entry(fd, host::__WASI_RIGHT_FD_FILESTAT_SET_TIMES, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };
    let st_atim = dec_timestamp(st_atim);
    let st_mtim = dec_timestamp(st_mtim);
    let fst_flags = dec_fstflags(fst_flags);

    let ret = match hostcalls_impl::fd_filestat_set_times(fd, st_atim, st_mtim, fst_flags) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_filestat_set_size(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    st_size: wasm32::__wasi_filesize_t,
) -> wasm32::__wasi_errno_t {
    trace!("fd_filestat_set_size(fd={:?}, st_size={})", fd, st_size);

    let fd = dec_fd(fd);
    let fd = match wasi_ctx
        .get_fd_entry(fd, host::__WASI_RIGHT_FD_FILESTAT_SET_SIZE, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };
    let st_size = dec_filesize(st_size);
    if st_size > i64::max_value() as u64 {
        return return_enc_errno(host::__WASI_E2BIG);
    }

    let ret = match hostcalls_impl::fd_filestat_set_size(fd, st_size) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
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
    trace!(
        "path_filestat_get(dirfd={:?}, dirflags={:?}, path_ptr={:#x?}, path_len={}, filestat_ptr={:#x?})",
        dirfd,
        dirflags,
        path_ptr,
        path_len,
        filestat_ptr
    );

    let dirfd = dec_fd(dirfd);
    let dirflags = dec_lookupflags(dirflags);
    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len).and_then(host::path_from_slice)
    {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | (path_ptr,path_len)='{}'", path);

    let dirfd = match wasi_ctx
        .get_fd_entry(dirfd, host::__WASI_RIGHT_PATH_FILESTAT_GET, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let host_filestat = match hostcalls_impl::path_filestat_get(dirfd, dirflags, path) {
        Ok(host_filestat) => host_filestat,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | *filestat_ptr={:?}", host_filestat);

    let ret = match enc_filestat_byref(memory, filestat_ptr, host_filestat) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
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
    trace!(
        "path_filestat_set_times(dirfd={:?}, dirflags={:?}, path_ptr={:#x?}, path_len={}, st_atim={}, st_mtim={}, fst_flags={:#x?})",
        dirfd,
        dirflags,
        path_ptr,
        path_len,
        st_atim, st_mtim,
        fst_flags
    );

    let dirfd = dec_fd(dirfd);
    let dirflags = dec_lookupflags(dirflags);
    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len).and_then(host::path_from_slice)
    {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | (path_ptr,path_len)='{}'", path);

    let st_atim = dec_timestamp(st_atim);
    let st_mtim = dec_timestamp(st_mtim);
    let fst_flags = dec_fstflags(fst_flags);

    let dirfd = match wasi_ctx
        .get_fd_entry(dirfd, host::__WASI_RIGHT_PATH_FILESTAT_SET_TIMES, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let ret = match hostcalls_impl::path_filestat_set_times(
        dirfd, dirflags, path, st_atim, st_mtim, fst_flags,
    ) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
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
    trace!(
        "path_symlink(old_path_ptr={:#x?}, old_path_len={}, dirfd={:?}, new_path_ptr={:#x?}, new_path_len={})",
        old_path_ptr,
        old_path_len,
        dirfd,
        new_path_ptr,
        new_path_len
    );

    let dirfd = dec_fd(dirfd);
    let old_path = match dec_slice_of::<u8>(memory, old_path_ptr, old_path_len)
        .and_then(host::path_from_slice)
    {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };
    let new_path = match dec_slice_of::<u8>(memory, new_path_ptr, new_path_len)
        .and_then(host::path_from_slice)
    {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | (old_path_ptr,old_path_len)='{}'", old_path);
    trace!("     | (new_path_ptr,new_path_len)='{}'", new_path);

    let dirfd = match wasi_ctx
        .get_fd_entry(dirfd, host::__WASI_RIGHT_PATH_SYMLINK, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let ret = match hostcalls_impl::path_symlink(dirfd, old_path, new_path) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn path_unlink_file(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "path_unlink_file(dirfd={:?}, path_ptr={:#x?}, path_len={})",
        dirfd,
        path_ptr,
        path_len
    );

    let dirfd = dec_fd(dirfd);
    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len).and_then(host::path_from_slice)
    {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | (path_ptr,path_len)='{}'", path);

    let dirfd = match wasi_ctx
        .get_fd_entry(dirfd, host::__WASI_RIGHT_PATH_UNLINK_FILE, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let ret = match hostcalls_impl::path_unlink_file(dirfd, path) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn path_remove_directory(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "path_remove_directory(dirfd={:?}, path_ptr={:#x?}, path_len={})",
        dirfd,
        path_ptr,
        path_len
    );

    let dirfd = dec_fd(dirfd);
    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len).and_then(host::path_from_slice)
    {
        Ok(path) => path,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | (path_ptr,path_len)='{}'", path);

    let dirfd = match wasi_ctx
        .get_fd_entry(dirfd, host::__WASI_RIGHT_PATH_REMOVE_DIRECTORY, 0)
        .and_then(|fe| fe.fd_object.descriptor.as_file())
    {
        Ok(f) => f,
        Err(e) => return return_enc_errno(e),
    };

    let ret = match hostcalls_impl::path_remove_directory(dirfd, path) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_prestat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    prestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "fd_prestat_get(fd={:?}, prestat_ptr={:#x?})",
        fd,
        prestat_ptr
    );

    let fd = dec_fd(fd);
    // TODO: is this the correct right for this?
    let ret = match wasi_ctx.get_fd_entry(fd, host::__WASI_RIGHT_PATH_OPEN.into(), 0) {
        Ok(fe) => {
            if let Some(po_path) = &fe.preopen_path {
                if fe.fd_object.file_type != host::__WASI_FILETYPE_DIRECTORY {
                    return return_enc_errno(host::__WASI_ENOTDIR);
                }

                let path = match host_impl::path_from_host(po_path.as_os_str()) {
                    Ok(path) => path,
                    Err(e) => return return_enc_errno(e),
                };

                enc_prestat_byref(
                    memory,
                    prestat_ptr,
                    host::__wasi_prestat_t {
                        pr_type: host::__WASI_PREOPENTYPE_DIR,
                        u: host::__wasi_prestat_t___wasi_prestat_u {
                            dir: host::__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t {
                                pr_name_len: path.len(),
                            },
                        },
                    },
                )
                .map(|_| host::__WASI_ESUCCESS)
                .unwrap_or_else(identity)
            } else {
                host::__WASI_ENOTSUP
            }
        }
        Err(e) => e,
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn fd_prestat_dir_name(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "fd_prestat_dir_name(fd={:?}, path_ptr={:#x?}, path_len={})",
        fd,
        path_ptr,
        path_len
    );

    let fd = dec_fd(fd);

    let ret = match wasi_ctx.get_fd_entry(fd, host::__WASI_RIGHT_PATH_OPEN.into(), 0) {
        Ok(fe) => {
            if let Some(po_path) = &fe.preopen_path {
                if fe.fd_object.file_type != host::__WASI_FILETYPE_DIRECTORY {
                    return return_enc_errno(host::__WASI_ENOTDIR);
                }

                let path = match host_impl::path_from_host(po_path.as_os_str()) {
                    Ok(path) => path,
                    Err(e) => return return_enc_errno(e),
                };

                if path.len() > dec_usize(path_len) {
                    return return_enc_errno(host::__WASI_ENAMETOOLONG);
                }

                trace!("     | (path_ptr,path_len)='{}'", path);

                enc_slice_of(memory, path.as_bytes(), path_ptr)
                    .map(|_| host::__WASI_ESUCCESS)
                    .unwrap_or_else(identity)
            } else {
                host::__WASI_ENOTSUP
            }
        }
        Err(e) => e,
    };

    return_enc_errno(ret)
}

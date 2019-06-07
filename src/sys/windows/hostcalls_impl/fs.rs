#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
use super::fdentry::FdEntry;
use super::host_impl;

use crate::ctx::WasiCtx;
use crate::host;

use std::ffi::OsStr;

pub(crate) fn fd_close(fd_entry: FdEntry) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("fd_close")
}

pub(crate) fn fd_datasync(fd_entry: &FdEntry) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("fd_datasync")
}

pub(crate) fn fd_pread(
    fd_entry: &FdEntry,
    buf: &mut [u8],
    offset: host::__wasi_filesize_t,
) -> Result<usize, host::__wasi_errno_t> {
    unimplemented!("fd_pread")
}

pub(crate) fn fd_pwrite(
    fd_entry: &FdEntry,
    buf: &[u8],
    offset: host::__wasi_filesize_t,
) -> Result<usize, host::__wasi_errno_t> {
    unimplemented!("fd_pwrite")
}

pub(crate) fn fd_read(
    fd_entry: &FdEntry,
    iovs: &mut [host::__wasi_iovec_t],
) -> Result<usize, host::__wasi_errno_t> {
    unimplemented!("fd_pread")
}

pub(crate) fn fd_renumber(
    wasi_ctx: &mut WasiCtx,
    from: host::__wasi_fd_t,
    to: host::__wasi_fd_t,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("fd_renumber")
}

pub(crate) fn fd_seek(
    fd_entry: &FdEntry,
    offset: host::__wasi_filedelta_t,
    whence: host::__wasi_whence_t,
) -> Result<u64, host::__wasi_errno_t> {
    unimplemented!("fd_seek")
}

pub(crate) fn fd_tell(fd_entry: &FdEntry) -> Result<u64, host::__wasi_errno_t> {
    unimplemented!("fd_tell")
}

pub(crate) fn fd_fdstat_get(
    fd_entry: &FdEntry,
) -> Result<host::__wasi_fdflags_t, host::__wasi_errno_t> {
    unimplemented!("fd_fdstat_get")
}

pub(crate) fn fd_fdstat_set_flags(
    fd_entry: &FdEntry,
    fdflags: host::__wasi_fdflags_t,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("fd_fdstat_set_flags")
}

pub(crate) fn fd_sync(fd_entry: &FdEntry) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("fd_sync")
}

pub(crate) fn fd_write(
    fd_entry: &FdEntry,
    iovs: &[host::__wasi_iovec_t],
) -> Result<usize, host::__wasi_errno_t> {
    use winapi::shared::minwindef::{DWORD, LPVOID};
    use winapi::um::fileapi::WriteFile;

    let iovs: Vec<host_impl::IoVec> = iovs
        .iter()
        .map(|iov| unsafe { host_impl::iovec_to_win(iov) })
        .collect();

    let buf = iovs
        .iter()
        .find(|b| !b.as_slice().is_empty())
        .map_or(&[][..], |b| b.as_slice());

    let mut host_nwritten = 0;
    let len = std::cmp::min(buf.len(), <DWORD>::max_value() as usize) as DWORD;
    unsafe {
        WriteFile(
            fd_entry.fd_object.raw_handle,
            buf.as_ptr() as LPVOID,
            len,
            &mut host_nwritten,
            std::ptr::null_mut(),
        )
    };

    Ok(host_nwritten as usize)
}

pub(crate) fn fd_advise(
    fd_entry: &FdEntry,
    advice: host::__wasi_advice_t,
    offset: host::__wasi_filesize_t,
    len: host::__wasi_filesize_t,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("fd_advise")
}

pub(crate) fn fd_allocate(
    fd_entry: &FdEntry,
    offset: host::__wasi_filesize_t,
    len: host::__wasi_filesize_t,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("fd_allocate")
}

pub(crate) fn path_create_directory(
    ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    path: &OsStr,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("path_create_directory")
}

pub(crate) fn path_link(
    ctx: &WasiCtx,
    old_dirfd: host::__wasi_fd_t,
    new_dirfd: host::__wasi_fd_t,
    old_path: &OsStr,
    new_path: &OsStr,
    source_rights: host::__wasi_rights_t,
    target_rights: host::__wasi_rights_t,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("path_link")
}

pub(crate) fn path_open(
    ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    dirflags: host::__wasi_lookupflags_t,
    path: &OsStr,
    oflags: host::__wasi_oflags_t,
    read: bool,
    write: bool,
    mut needed_base: host::__wasi_rights_t,
    mut needed_inheriting: host::__wasi_rights_t,
    fs_flags: host::__wasi_fdflags_t,
) -> Result<FdEntry, host::__wasi_errno_t> {
    unimplemented!("path_open")
}

pub(crate) fn fd_readdir(
    fd_entry: &FdEntry,
    host_buf: &mut [u8],
    cookie: host::__wasi_dircookie_t,
) -> Result<usize, host::__wasi_errno_t> {
    unimplemented!("fd_readdir")
}

pub(crate) fn path_readlink(
    wasi_ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    path: &OsStr,
    rights: host::__wasi_rights_t,
    buf: &mut [u8],
) -> Result<usize, host::__wasi_errno_t> {
    unimplemented!("path_readlink")
}

pub(crate) fn path_rename(
    wasi_ctx: &WasiCtx,
    old_dirfd: host::__wasi_fd_t,
    old_path: &OsStr,
    old_rights: host::__wasi_rights_t,
    new_dirfd: host::__wasi_fd_t,
    new_path: &OsStr,
    new_rights: host::__wasi_rights_t,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("path_rename")
}

pub(crate) fn fd_filestat_get(
    fd_entry: &FdEntry,
) -> Result<host::__wasi_filestat_t, host::__wasi_errno_t> {
    unimplemented!("fd_filestat_get")
}

pub(crate) fn fd_filestat_set_times(
    fd_entry: &FdEntry,
    st_atim: host::__wasi_timestamp_t,
    mut st_mtim: host::__wasi_timestamp_t,
    fst_flags: host::__wasi_fstflags_t,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("fd_filestat_set_times")
}

pub(crate) fn fd_filestat_set_size(
    fd_entry: &FdEntry,
    st_size: host::__wasi_filesize_t,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("fd_filestat_set_size")
}

pub(crate) fn path_filestat_get(
    wasi_ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    dirflags: host::__wasi_lookupflags_t,
    path: &OsStr,
) -> Result<host::__wasi_filestat_t, host::__wasi_errno_t> {
    unimplemented!("path_filestat_get")
}

pub(crate) fn path_filestat_set_times(
    wasi_ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    dirflags: host::__wasi_lookupflags_t,
    path: &OsStr,
    rights: host::__wasi_rights_t,
    st_atim: host::__wasi_timestamp_t,
    mut st_mtim: host::__wasi_timestamp_t,
    fst_flags: host::__wasi_fstflags_t,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("path_filestat_set_times")
}

pub(crate) fn path_symlink(
    wasi_ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    rights: host::__wasi_rights_t,
    old_path: &OsStr,
    new_path: &OsStr,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("path_symlink")
}

pub(crate) fn path_unlink_file(
    wasi_ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    path: &OsStr,
    rights: host::__wasi_rights_t,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("path_unlink_file")
}

pub(crate) fn path_remove_directory(
    wasi_ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    path: &OsStr,
    rights: host::__wasi_rights_t,
) -> Result<(), host::__wasi_errno_t> {
    unimplemented!("path_remove_directory")
}

#![allow(non_camel_case_types)]
#![allow(unused)]
use super::fdentry::{determine_type_rights, FdEntry};
use super::fs_helpers::*;
use super::host_impl;

use crate::ctx::WasiCtx;
use crate::host;

use std::ffi::OsStr;
use std::os::windows::prelude::FromRawHandle;

pub(crate) fn fd_close(fd_entry: FdEntry) -> Result<(), host::__wasi_errno_t> {
    winx::handle::close(fd_entry.fd_object.raw_handle).map_err(|e| host_impl::errno_from_win(e))
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
    use winx::io::{readv, IoVecMut};

    let mut iovs: Vec<IoVecMut> = iovs
        .iter_mut()
        .map(|iov| unsafe { host_impl::iovec_to_win_mut(iov) })
        .collect();

    readv(fd_entry.fd_object.raw_handle, &mut iovs).map_err(|e| host_impl::errno_from_win(e))
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
    use winx::file::AccessRight;
    match winx::file::get_file_access_rights(fd_entry.fd_object.raw_handle)
        .map(AccessRight::from_bits_truncate)
    {
        Ok(rights) => Ok(host_impl::fdflags_from_win(rights)),
        Err(e) => Err(host_impl::errno_from_win(e)),
    }
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
    use winx::io::{writev, IoVec};

    let iovs: Vec<IoVec> = iovs
        .iter()
        .map(|iov| unsafe { host_impl::iovec_to_win(iov) })
        .collect();

    writev(fd_entry.fd_object.raw_handle, &iovs).map_err(|e| host_impl::errno_from_win(e))
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
    use winx::file::{AccessRight, CreationDisposition, FlagsAndAttributes, ShareMode};

    let mut win_rights = AccessRight::READ_CONTROL;
    if read {
        win_rights.insert(AccessRight::FILE_GENERIC_READ);
    }
    if write {
        win_rights.insert(AccessRight::FILE_GENERIC_WRITE);
    }

    // convert open flags
    let (win_create_disp, mut win_flags_attrs) = host_impl::win_from_oflags(oflags);
    if win_create_disp == CreationDisposition::CREATE_NEW {
        needed_base |= host::__WASI_RIGHT_PATH_CREATE_FILE;
    } else if win_create_disp == CreationDisposition::CREATE_ALWAYS {
        needed_base |= host::__WASI_RIGHT_PATH_CREATE_FILE;
    } else if win_create_disp == CreationDisposition::TRUNCATE_EXISTING {
        needed_base |= host::__WASI_RIGHT_PATH_FILESTAT_SET_SIZE;
    }

    // convert file descriptor flags
    let win_fdflags_res = host_impl::win_from_fdflags(fs_flags);
    win_rights.insert(win_fdflags_res.0);
    win_flags_attrs.insert(win_fdflags_res.1);
    if win_rights.contains(AccessRight::SYNCHRONIZE) {
        needed_inheriting |= host::__WASI_RIGHT_FD_DATASYNC;
        needed_inheriting |= host::__WASI_RIGHT_FD_SYNC;
    }

    let (dir, path) = match path_get(
        ctx,
        dirfd,
        dirflags,
        path,
        needed_base,
        needed_inheriting,
        !win_flags_attrs.contains(FlagsAndAttributes::FILE_FLAG_BACKUP_SEMANTICS),
    ) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };

    let new_handle =
        match winx::file::openat(dir, &path, win_rights, win_create_disp, win_flags_attrs) {
            Ok(handle) => handle,
            Err(e) => return Err(host_impl::errno_from_win(e)),
        };

    // Determine the type of the new file descriptor and which rights contradict with this type
    match unsafe { determine_type_rights(new_handle) } {
        Err(e) => {
            // if `close` fails, note it but do not override the underlying errno
            winx::handle::close(new_handle).unwrap_or_else(|e| {
                dbg!(e);
            });
            Err(e)
        }
        Ok((_ty, max_base, max_inheriting)) => {
            let mut fe = unsafe { FdEntry::from_raw_handle(new_handle) };
            fe.rights_base &= max_base;
            fe.rights_inheriting &= max_inheriting;
            Ok(fe)
        }
    }
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

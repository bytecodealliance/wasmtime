#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
use super::host_impl;
use super::host_impl::IoVec;

use crate::ctx::WasiCtx;
use crate::memory::*;
use crate::{host, wasm32};

use std::cmp;
use std::os::windows::prelude::OsStrExt;
use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
pub fn fd_close(wasi_ctx: &mut WasiCtx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_close")
}

#[wasi_common_cbindgen]
pub fn fd_datasync(wasi_ctx: &WasiCtx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_datasync")
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
    unimplemented!("fd_pread")
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
    unimplemented!("fd_pwrite")
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
    unimplemented!("fd_read")
}

#[wasi_common_cbindgen]
pub fn fd_renumber(
    wasi_ctx: &mut WasiCtx,
    from: wasm32::__wasi_fd_t,
    to: wasm32::__wasi_fd_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_renumber")
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
    unimplemented!("fd_seek")
}

#[wasi_common_cbindgen]
pub fn fd_tell(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    newoffset: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_tell")
}

#[wasi_common_cbindgen]
pub fn fd_fdstat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    fdstat_ptr: wasm32::uintptr_t, // *mut wasm32::__wasi_fdstat_t
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_fdstat_get")
}

#[wasi_common_cbindgen]
pub fn fd_fdstat_set_flags(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    fdflags: wasm32::__wasi_fdflags_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_fdstat_set_flags")
}

#[wasi_common_cbindgen]
pub fn fd_fdstat_set_rights(
    wasi_ctx: &mut WasiCtx,
    fd: wasm32::__wasi_fd_t,
    fs_rights_base: wasm32::__wasi_rights_t,
    fs_rights_inheriting: wasm32::__wasi_rights_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_fdstat_set_rights")
}

#[wasi_common_cbindgen]
pub fn fd_sync(wasi_ctx: &WasiCtx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_sync")
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
    use winapi::shared::minwindef::{DWORD, LPVOID};
    use winapi::shared::ws2def::WSABUF;
    use winapi::um::fileapi::WriteFile;

    let fd = dec_fd(fd);
    let mut iovs = match dec_iovec_slice(memory, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };

    let fe = match wasi_ctx.get_fd_entry(fd, host::__WASI_RIGHT_FD_WRITE.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };

    let iovs: Vec<IoVec> = iovs
        .iter()
        .map(|iov| unsafe { host_impl::iovec_to_win(iov) })
        .collect();

    let buf = iovs
        .iter()
        .find(|b| !b.as_slice().is_empty())
        .map_or(&[][..], |b| b.as_slice());

    let mut host_nwritten = 0;
    let len = cmp::min(buf.len(), <DWORD>::max_value() as usize) as DWORD;
    unsafe {
        WriteFile(
            fe.fd_object.raw_handle,
            buf.as_ptr() as LPVOID,
            len,
            &mut host_nwritten,
            std::ptr::null_mut(),
        )
    };

    enc_usize_byref(memory, nwritten, host_nwritten as usize)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

#[wasi_common_cbindgen]
pub fn fd_advise(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filesize_t,
    len: wasm32::__wasi_filesize_t,
    advice: wasm32::__wasi_advice_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_advise")
}

#[wasi_common_cbindgen]
pub fn fd_allocate(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filesize_t,
    len: wasm32::__wasi_filesize_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_allocate")
}

#[wasi_common_cbindgen]
pub fn path_create_directory(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("path_create_directory")
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
    unimplemented!("path_link")
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
    unimplemented!("path_open")
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
    unimplemented!("fd_readdir")
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
    unimplemented!("path_readlink")
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
    unimplemented!("path_rename")
}

#[wasi_common_cbindgen]
pub fn fd_filestat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    filestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_filestat_get")
}

#[wasi_common_cbindgen]
pub fn fd_filestat_set_times(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    st_atim: wasm32::__wasi_timestamp_t,
    st_mtim: wasm32::__wasi_timestamp_t,
    fst_flags: wasm32::__wasi_fstflags_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_filestat_set_times")
}

#[wasi_common_cbindgen]
pub fn fd_filestat_set_size(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    st_size: wasm32::__wasi_filesize_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_filestat_set_size")
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
    unimplemented!("path_filestat_get")
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
    unimplemented!("path_filestat_set_times")
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
    unimplemented!("path_symlink")
}

#[wasi_common_cbindgen]
pub fn path_unlink_file(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("path_unlink_file")
}

#[wasi_common_cbindgen]
pub fn path_remove_directory(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("path_remove_directory")
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
                                // TODO: clean up
                                pr_name_len: po_path.as_os_str().encode_wide().count() * 2,
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
                // TODO: clean up
                let path_bytes = &po_path
                    .as_os_str()
                    .encode_wide()
                    .map(u16::to_le_bytes)
                    .fold(Vec::new(), |mut acc, bytes| {
                        acc.extend_from_slice(&bytes);
                        acc
                    });
                if path_bytes.len() > dec_usize(path_len) {
                    return wasm32::__WASI_ENAMETOOLONG;
                }
                enc_slice_of(memory, path_bytes, path_ptr)
                    .map(|_| wasm32::__WASI_ESUCCESS)
                    .unwrap_or_else(|e| e)
            } else {
                wasm32::__WASI_ENOTSUP
            }
        }
        Err(e) => enc_errno(e),
    }
}

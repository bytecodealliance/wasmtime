#![allow(non_camel_case_types)]
use crate::old::snapshot_0::ctx::WasiCtx;
use crate::old::snapshot_0::{wasi, wasi32};

hostcalls_old! {
    pub unsafe fn fd_close(wasi_ctx: &mut WasiCtx, fd: wasi::__wasi_fd_t,) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_datasync(wasi_ctx: &WasiCtx, fd: wasi::__wasi_fd_t,) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_pread(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        fd: wasi::__wasi_fd_t,
        iovs_ptr: wasi32::uintptr_t,
        iovs_len: wasi32::size_t,
        offset: wasi::__wasi_filesize_t,
        nread: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_pwrite(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        fd: wasi::__wasi_fd_t,
        iovs_ptr: wasi32::uintptr_t,
        iovs_len: wasi32::size_t,
        offset: wasi::__wasi_filesize_t,
        nwritten: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_read(
        wasi_ctx: &mut WasiCtx,
        memory: &mut [u8],
        fd: wasi::__wasi_fd_t,
        iovs_ptr: wasi32::uintptr_t,
        iovs_len: wasi32::size_t,
        nread: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_renumber(
        wasi_ctx: &mut WasiCtx,
        from: wasi::__wasi_fd_t,
        to: wasi::__wasi_fd_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_seek(
        wasi_ctx: &mut WasiCtx,
        memory: &mut [u8],
        fd: wasi::__wasi_fd_t,
        offset: wasi::__wasi_filedelta_t,
        whence: wasi::__wasi_whence_t,
        newoffset: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_tell(
        wasi_ctx: &mut WasiCtx,
        memory: &mut [u8],
        fd: wasi::__wasi_fd_t,
        newoffset: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_fdstat_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        fd: wasi::__wasi_fd_t,
        fdstat_ptr: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_fdstat_set_flags(
        wasi_ctx: &WasiCtx,
        fd: wasi::__wasi_fd_t,
        fdflags: wasi::__wasi_fdflags_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_fdstat_set_rights(
        wasi_ctx: &mut WasiCtx,
        fd: wasi::__wasi_fd_t,
        fs_rights_base: wasi::__wasi_rights_t,
        fs_rights_inheriting: wasi::__wasi_rights_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_sync(wasi_ctx: &WasiCtx, fd: wasi::__wasi_fd_t,) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_write(
        wasi_ctx: &mut WasiCtx,
        memory: &mut [u8],
        fd: wasi::__wasi_fd_t,
        iovs_ptr: wasi32::uintptr_t,
        iovs_len: wasi32::size_t,
        nwritten: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_advise(
        wasi_ctx: &WasiCtx,
        fd: wasi::__wasi_fd_t,
        offset: wasi::__wasi_filesize_t,
        len: wasi::__wasi_filesize_t,
        advice: wasi::__wasi_advice_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_allocate(
        wasi_ctx: &WasiCtx,
        fd: wasi::__wasi_fd_t,
        offset: wasi::__wasi_filesize_t,
        len: wasi::__wasi_filesize_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn path_create_directory(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        dirfd: wasi::__wasi_fd_t,
        path_ptr: wasi32::uintptr_t,
        path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn path_link(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        old_dirfd: wasi::__wasi_fd_t,
        old_flags: wasi::__wasi_lookupflags_t,
        old_path_ptr: wasi32::uintptr_t,
        old_path_len: wasi32::size_t,
        new_dirfd: wasi::__wasi_fd_t,
        new_path_ptr: wasi32::uintptr_t,
        new_path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn path_open(
        wasi_ctx: &mut WasiCtx,
        memory: &mut [u8],
        dirfd: wasi::__wasi_fd_t,
        dirflags: wasi::__wasi_lookupflags_t,
        path_ptr: wasi32::uintptr_t,
        path_len: wasi32::size_t,
        oflags: wasi::__wasi_oflags_t,
        fs_rights_base: wasi::__wasi_rights_t,
        fs_rights_inheriting: wasi::__wasi_rights_t,
        fs_flags: wasi::__wasi_fdflags_t,
        fd_out_ptr: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_readdir(
        wasi_ctx: &mut WasiCtx,
        memory: &mut [u8],
        fd: wasi::__wasi_fd_t,
        buf: wasi32::uintptr_t,
        buf_len: wasi32::size_t,
        cookie: wasi::__wasi_dircookie_t,
        buf_used: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn path_readlink(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        dirfd: wasi::__wasi_fd_t,
        path_ptr: wasi32::uintptr_t,
        path_len: wasi32::size_t,
        buf_ptr: wasi32::uintptr_t,
        buf_len: wasi32::size_t,
        buf_used: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn path_rename(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        old_dirfd: wasi::__wasi_fd_t,
        old_path_ptr: wasi32::uintptr_t,
        old_path_len: wasi32::size_t,
        new_dirfd: wasi::__wasi_fd_t,
        new_path_ptr: wasi32::uintptr_t,
        new_path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_filestat_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        fd: wasi::__wasi_fd_t,
        filestat_ptr: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_filestat_set_times(
        wasi_ctx: &WasiCtx,
        fd: wasi::__wasi_fd_t,
        st_atim: wasi::__wasi_timestamp_t,
        st_mtim: wasi::__wasi_timestamp_t,
        fst_flags: wasi::__wasi_fstflags_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_filestat_set_size(
        wasi_ctx: &WasiCtx,
        fd: wasi::__wasi_fd_t,
        st_size: wasi::__wasi_filesize_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn path_filestat_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        dirfd: wasi::__wasi_fd_t,
        dirflags: wasi::__wasi_lookupflags_t,
        path_ptr: wasi32::uintptr_t,
        path_len: wasi32::size_t,
        filestat_ptr: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn path_filestat_set_times(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        dirfd: wasi::__wasi_fd_t,
        dirflags: wasi::__wasi_lookupflags_t,
        path_ptr: wasi32::uintptr_t,
        path_len: wasi32::size_t,
        st_atim: wasi::__wasi_timestamp_t,
        st_mtim: wasi::__wasi_timestamp_t,
        fst_flags: wasi::__wasi_fstflags_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn path_symlink(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        old_path_ptr: wasi32::uintptr_t,
        old_path_len: wasi32::size_t,
        dirfd: wasi::__wasi_fd_t,
        new_path_ptr: wasi32::uintptr_t,
        new_path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn path_unlink_file(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        dirfd: wasi::__wasi_fd_t,
        path_ptr: wasi32::uintptr_t,
        path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn path_remove_directory(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        dirfd: wasi::__wasi_fd_t,
        path_ptr: wasi32::uintptr_t,
        path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_prestat_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        fd: wasi::__wasi_fd_t,
        prestat_ptr: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn fd_prestat_dir_name(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        fd: wasi::__wasi_fd_t,
        path_ptr: wasi32::uintptr_t,
        path_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;
}

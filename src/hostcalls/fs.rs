#![allow(non_camel_case_types)]
use crate::ctx::WasiCtx;
use crate::wasm32;

hostcalls! {
    pub fn fd_close(wasi_ctx: &mut WasiCtx, fd: wasm32::__wasi_fd_t,) -> wasm32::__wasi_errno_t;

    pub fn fd_datasync(wasi_ctx: &WasiCtx, fd: wasm32::__wasi_fd_t,) -> wasm32::__wasi_errno_t;

    pub fn fd_pread(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        fd: wasm32::__wasi_fd_t,
        iovs_ptr: wasm32::uintptr_t,
        iovs_len: wasm32::size_t,
        offset: wasm32::__wasi_filesize_t,
        nread: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_pwrite(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        fd: wasm32::__wasi_fd_t,
        iovs_ptr: wasm32::uintptr_t,
        iovs_len: wasm32::size_t,
        offset: wasm32::__wasi_filesize_t,
        nwritten: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_read(
        wasi_ctx: &mut WasiCtx,
        memory: &mut [u8],
        fd: wasm32::__wasi_fd_t,
        iovs_ptr: wasm32::uintptr_t,
        iovs_len: wasm32::size_t,
        nread: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_renumber(
        wasi_ctx: &mut WasiCtx,
        from: wasm32::__wasi_fd_t,
        to: wasm32::__wasi_fd_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_seek(
        wasi_ctx: &mut WasiCtx,
        memory: &mut [u8],
        fd: wasm32::__wasi_fd_t,
        offset: wasm32::__wasi_filedelta_t,
        whence: wasm32::__wasi_whence_t,
        newoffset: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_tell(
        wasi_ctx: &mut WasiCtx,
        memory: &mut [u8],
        fd: wasm32::__wasi_fd_t,
        newoffset: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_fdstat_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        fd: wasm32::__wasi_fd_t,
        fdstat_ptr: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_fdstat_set_flags(
        wasi_ctx: &WasiCtx,
        fd: wasm32::__wasi_fd_t,
        fdflags: wasm32::__wasi_fdflags_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_fdstat_set_rights(
        wasi_ctx: &mut WasiCtx,
        fd: wasm32::__wasi_fd_t,
        fs_rights_base: wasm32::__wasi_rights_t,
        fs_rights_inheriting: wasm32::__wasi_rights_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_sync(wasi_ctx: &WasiCtx, fd: wasm32::__wasi_fd_t,) -> wasm32::__wasi_errno_t;

    pub fn fd_write(
        wasi_ctx: &mut WasiCtx,
        memory: &mut [u8],
        fd: wasm32::__wasi_fd_t,
        iovs_ptr: wasm32::uintptr_t,
        iovs_len: wasm32::size_t,
        nwritten: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_advise(
        wasi_ctx: &WasiCtx,
        fd: wasm32::__wasi_fd_t,
        offset: wasm32::__wasi_filesize_t,
        len: wasm32::__wasi_filesize_t,
        advice: wasm32::__wasi_advice_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_allocate(
        wasi_ctx: &WasiCtx,
        fd: wasm32::__wasi_fd_t,
        offset: wasm32::__wasi_filesize_t,
        len: wasm32::__wasi_filesize_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn path_create_directory(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        dirfd: wasm32::__wasi_fd_t,
        path_ptr: wasm32::uintptr_t,
        path_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t;

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
    ) -> wasm32::__wasi_errno_t;

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
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_readdir(
        wasi_ctx: &mut WasiCtx,
        memory: &mut [u8],
        fd: wasm32::__wasi_fd_t,
        buf: wasm32::uintptr_t,
        buf_len: wasm32::size_t,
        cookie: wasm32::__wasi_dircookie_t,
        buf_used: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn path_readlink(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        dirfd: wasm32::__wasi_fd_t,
        path_ptr: wasm32::uintptr_t,
        path_len: wasm32::size_t,
        buf_ptr: wasm32::uintptr_t,
        buf_len: wasm32::size_t,
        buf_used: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn path_rename(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        old_dirfd: wasm32::__wasi_fd_t,
        old_path_ptr: wasm32::uintptr_t,
        old_path_len: wasm32::size_t,
        new_dirfd: wasm32::__wasi_fd_t,
        new_path_ptr: wasm32::uintptr_t,
        new_path_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_filestat_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        fd: wasm32::__wasi_fd_t,
        filestat_ptr: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_filestat_set_times(
        wasi_ctx: &WasiCtx,
        fd: wasm32::__wasi_fd_t,
        st_atim: wasm32::__wasi_timestamp_t,
        st_mtim: wasm32::__wasi_timestamp_t,
        fst_flags: wasm32::__wasi_fstflags_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_filestat_set_size(
        wasi_ctx: &WasiCtx,
        fd: wasm32::__wasi_fd_t,
        st_size: wasm32::__wasi_filesize_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn path_filestat_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        dirfd: wasm32::__wasi_fd_t,
        dirflags: wasm32::__wasi_lookupflags_t,
        path_ptr: wasm32::uintptr_t,
        path_len: wasm32::size_t,
        filestat_ptr: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

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
    ) -> wasm32::__wasi_errno_t;

    pub fn path_symlink(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        old_path_ptr: wasm32::uintptr_t,
        old_path_len: wasm32::size_t,
        dirfd: wasm32::__wasi_fd_t,
        new_path_ptr: wasm32::uintptr_t,
        new_path_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn path_unlink_file(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        dirfd: wasm32::__wasi_fd_t,
        path_ptr: wasm32::uintptr_t,
        path_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn path_remove_directory(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        dirfd: wasm32::__wasi_fd_t,
        path_ptr: wasm32::uintptr_t,
        path_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_prestat_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        fd: wasm32::__wasi_fd_t,
        prestat_ptr: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn fd_prestat_dir_name(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        fd: wasm32::__wasi_fd_t,
        path_ptr: wasm32::uintptr_t,
        path_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t;
}

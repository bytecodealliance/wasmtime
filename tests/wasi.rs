use wiggle_runtime::{GuestError, GuestErrorType, GuestPtr};
use wiggle_test::WasiCtx;

wiggle::from_witx!({
    witx: ["tests/wasi.witx"],
    ctx: WasiCtx,
});

type Result<T> = std::result::Result<T, types::Errno>;

impl GuestErrorType for types::Errno {
    type Context = WasiCtx;

    fn success() -> types::Errno {
        types::Errno::Success
    }

    fn from_error(e: GuestError, ctx: &WasiCtx) -> types::Errno {
        eprintln!("GUEST ERROR: {:?}", e);
        ctx.guest_errors.borrow_mut().push(e);
        types::Errno::Io
    }
}

impl crate::wasi_snapshot_preview1::WasiSnapshotPreview1 for WasiCtx {
    fn args_get(&self, _argv: GuestPtr<GuestPtr<u8>>, _argv_buf: GuestPtr<u8>) -> Result<()> {
        unimplemented!("args_get")
    }

    fn args_sizes_get(&self) -> Result<(types::Size, types::Size)> {
        unimplemented!("args_sizes_get")
    }

    fn environ_get(
        &self,
        _environ: GuestPtr<GuestPtr<u8>>,
        _environ_buf: GuestPtr<u8>,
    ) -> Result<()> {
        unimplemented!("environ_get")
    }

    fn environ_sizes_get(&self) -> Result<(types::Size, types::Size)> {
        unimplemented!("environ_sizes_get")
    }

    fn clock_res_get(&self, _id: types::Clockid) -> Result<types::Timestamp> {
        unimplemented!("clock_res_get")
    }

    fn clock_time_get(
        &self,
        _id: types::Clockid,
        _precision: types::Timestamp,
    ) -> Result<types::Timestamp> {
        unimplemented!("clock_time_get")
    }

    fn fd_advise(
        &self,
        _fd: types::Fd,
        _offset: types::Filesize,
        _len: types::Filesize,
        _advice: types::Advice,
    ) -> Result<()> {
        unimplemented!("fd_advise")
    }

    fn fd_allocate(
        &self,
        _fd: types::Fd,
        _offset: types::Filesize,
        _len: types::Filesize,
    ) -> Result<()> {
        unimplemented!("fd_allocate")
    }

    fn fd_close(&self, _fd: types::Fd) -> Result<()> {
        unimplemented!("fd_close")
    }

    fn fd_datasync(&self, _fd: types::Fd) -> Result<()> {
        unimplemented!("fd_datasync")
    }

    fn fd_fdstat_get(&self, _fd: types::Fd) -> Result<types::Fdstat> {
        unimplemented!("fd_fdstat_get")
    }

    fn fd_fdstat_set_flags(&self, _fd: types::Fd, _flags: types::Fdflags) -> Result<()> {
        unimplemented!("fd_fdstat_set_flags")
    }

    fn fd_fdstat_set_rights(
        &self,
        _fd: types::Fd,
        _fs_rights_base: types::Rights,
        _fs_rights_inherting: types::Rights,
    ) -> Result<()> {
        unimplemented!("fd_fdstat_set_rights")
    }

    fn fd_filestat_get(&self, _fd: types::Fd) -> Result<types::Filestat> {
        unimplemented!("fd_filestat_get")
    }

    fn fd_filestat_set_size(&self, _fd: types::Fd, _size: types::Filesize) -> Result<()> {
        unimplemented!("fd_filestat_set_size")
    }

    fn fd_filestat_set_times(
        &self,
        _fd: types::Fd,
        _atim: types::Timestamp,
        _mtim: types::Timestamp,
        _fst_flags: types::Fstflags,
    ) -> Result<()> {
        unimplemented!("fd_filestat_set_times")
    }

    fn fd_pread(
        &self,
        _fd: types::Fd,
        _iovs: &types::IovecArray<'_>,
        _offset: types::Filesize,
    ) -> Result<types::Size> {
        unimplemented!("fd_pread")
    }

    fn fd_prestat_get(&self, _fd: types::Fd) -> Result<types::Prestat> {
        unimplemented!("fd_prestat_get")
    }

    fn fd_prestat_dir_name(
        &self,
        _fd: types::Fd,
        _path: GuestPtr<u8>,
        _path_len: types::Size,
    ) -> Result<()> {
        unimplemented!("fd_prestat_dir_name")
    }

    fn fd_pwrite(
        &self,
        _fd: types::Fd,
        _ciovs: &types::CiovecArray<'_>,
        _offset: types::Filesize,
    ) -> Result<types::Size> {
        unimplemented!("fd_pwrite")
    }

    fn fd_read(&self, _fd: types::Fd, _iovs: &types::IovecArray<'_>) -> Result<types::Size> {
        unimplemented!("fd_read")
    }

    fn fd_readdir(
        &self,
        _fd: types::Fd,
        _buf: GuestPtr<u8>,
        _buf_len: types::Size,
        _cookie: types::Dircookie,
    ) -> Result<types::Size> {
        unimplemented!("fd_readdir")
    }

    fn fd_renumber(&self, _fd: types::Fd, _to: types::Fd) -> Result<()> {
        unimplemented!("fd_renumber")
    }

    fn fd_seek(
        &self,
        _fd: types::Fd,
        _offset: types::Filedelta,
        _whence: types::Whence,
    ) -> Result<types::Filesize> {
        unimplemented!("fd_seek")
    }

    fn fd_sync(&self, _fd: types::Fd) -> Result<()> {
        unimplemented!("fd_sync")
    }

    fn fd_tell(&self, _fd: types::Fd) -> Result<types::Filesize> {
        unimplemented!("fd_tell")
    }

    fn fd_write(&self, _fd: types::Fd, _ciovs: &types::CiovecArray<'_>) -> Result<types::Size> {
        unimplemented!("fd_write")
    }

    fn path_create_directory(&self, _fd: types::Fd, _path: &GuestPtr<'_, str>) -> Result<()> {
        unimplemented!("path_create_directory")
    }

    fn path_filestat_get(
        &self,
        _fd: types::Fd,
        _flags: types::Lookupflags,
        _path: &GuestPtr<'_, str>,
    ) -> Result<types::Filestat> {
        unimplemented!("path_filestat_get")
    }

    fn path_filestat_set_times(
        &self,
        _fd: types::Fd,
        _flags: types::Lookupflags,
        _path: &GuestPtr<'_, str>,
        _atim: types::Timestamp,
        _mtim: types::Timestamp,
        _fst_flags: types::Fstflags,
    ) -> Result<()> {
        unimplemented!("path_filestat_set_times")
    }

    fn path_link(
        &self,
        _old_fd: types::Fd,
        _old_flags: types::Lookupflags,
        _old_path: &GuestPtr<'_, str>,
        _new_fd: types::Fd,
        _new_path: &GuestPtr<'_, str>,
    ) -> Result<()> {
        unimplemented!("path_link")
    }

    fn path_open(
        &self,
        _fd: types::Fd,
        _dirflags: types::Lookupflags,
        _path: &GuestPtr<'_, str>,
        _oflags: types::Oflags,
        _fs_rights_base: types::Rights,
        _fs_rights_inherting: types::Rights,
        _fdflags: types::Fdflags,
    ) -> Result<types::Fd> {
        unimplemented!("path_open")
    }

    fn path_readlink(
        &self,
        _fd: types::Fd,
        _path: &GuestPtr<'_, str>,
        _buf: GuestPtr<u8>,
        _buf_len: types::Size,
    ) -> Result<types::Size> {
        unimplemented!("path_readlink")
    }

    fn path_remove_directory(&self, _fd: types::Fd, _path: &GuestPtr<'_, str>) -> Result<()> {
        unimplemented!("path_remove_directory")
    }

    fn path_rename(
        &self,
        _fd: types::Fd,
        _old_path: &GuestPtr<'_, str>,
        _new_fd: types::Fd,
        _new_path: &GuestPtr<'_, str>,
    ) -> Result<()> {
        unimplemented!("path_rename")
    }

    fn path_symlink(
        &self,
        _old_path: &GuestPtr<'_, str>,
        _fd: types::Fd,
        _new_path: &GuestPtr<'_, str>,
    ) -> Result<()> {
        unimplemented!("path_symlink")
    }

    fn path_unlink_file(&self, _fd: types::Fd, _path: &GuestPtr<'_, str>) -> Result<()> {
        unimplemented!("path_unlink_file")
    }

    fn poll_oneoff(
        &self,
        _in_: GuestPtr<types::Subscription>,
        _out: GuestPtr<types::Event>,
        _nsubscriptions: types::Size,
    ) -> Result<types::Size> {
        unimplemented!("poll_oneoff")
    }

    fn proc_exit(&self, _rval: types::Exitcode) -> std::result::Result<(), ()> {
        unimplemented!("proc_exit")
    }

    fn proc_raise(&self, _sig: types::Signal) -> Result<()> {
        unimplemented!("proc_raise")
    }

    fn sched_yield(&self) -> Result<()> {
        unimplemented!("sched_yield")
    }

    fn random_get(&self, _buf: GuestPtr<u8>, _buf_len: types::Size) -> Result<()> {
        unimplemented!("random_get")
    }

    fn sock_recv(
        &self,
        _fd: types::Fd,
        _ri_data: &types::IovecArray<'_>,
        _ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags)> {
        unimplemented!("sock_recv")
    }

    fn sock_send(
        &self,
        _fd: types::Fd,
        _si_data: &types::CiovecArray<'_>,
        _si_flags: types::Siflags,
    ) -> Result<types::Size> {
        unimplemented!("sock_send")
    }

    fn sock_shutdown(&self, _fd: types::Fd, _how: types::Sdflags) -> Result<()> {
        unimplemented!("sock_shutdown")
    }
}

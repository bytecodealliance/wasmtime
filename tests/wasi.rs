use wiggle_runtime::{GuestError, GuestErrorType, GuestPtr, GuestPtrMut, GuestString};
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
    fn from_error(e: GuestError, ctx: &mut WasiCtx) -> types::Errno {
        eprintln!("GUEST ERROR: {:?}", e);
        ctx.guest_errors.push(e);
        types::Errno::Io
    }
}

impl crate::wasi_snapshot_preview1::WasiSnapshotPreview1 for WasiCtx {
    fn args_get(
        &mut self,
        _argv: GuestPtrMut<GuestPtrMut<u8>>,
        _argv_buf: GuestPtrMut<u8>,
    ) -> Result<()> {
        unimplemented!("args_get")
    }

    fn args_sizes_get(&mut self) -> Result<(types::Size, types::Size)> {
        unimplemented!("args_sizes_get")
    }

    fn environ_get(
        &mut self,
        _environ: GuestPtrMut<GuestPtrMut<u8>>,
        _environ_buf: GuestPtrMut<u8>,
    ) -> Result<()> {
        unimplemented!("environ_get")
    }

    fn environ_sizes_get(&mut self) -> Result<(types::Size, types::Size)> {
        unimplemented!("environ_sizes_get")
    }

    fn clock_res_get(&mut self, _id: types::Clockid) -> Result<types::Timestamp> {
        unimplemented!("clock_res_get")
    }

    fn clock_time_get(
        &mut self,
        _id: types::Clockid,
        _precision: types::Timestamp,
    ) -> Result<types::Timestamp> {
        unimplemented!("clock_time_get")
    }

    fn fd_advise(
        &mut self,
        _fd: types::Fd,
        _offset: types::Filesize,
        _len: types::Filesize,
        _advice: types::Advice,
    ) -> Result<()> {
        unimplemented!("fd_advise")
    }

    fn fd_allocate(
        &mut self,
        _fd: types::Fd,
        _offset: types::Filesize,
        _len: types::Filesize,
    ) -> Result<()> {
        unimplemented!("fd_allocate")
    }

    fn fd_close(&mut self, _fd: types::Fd) -> Result<()> {
        unimplemented!("fd_close")
    }

    fn fd_datasync(&mut self, _fd: types::Fd) -> Result<()> {
        unimplemented!("fd_datasync")
    }

    fn fd_fdstat_get(&mut self, _fd: types::Fd) -> Result<types::Fdstat> {
        unimplemented!("fd_fdstat_get")
    }

    fn fd_fdstat_set_flags(&mut self, _fd: types::Fd, _flags: types::Fdflags) -> Result<()> {
        unimplemented!("fd_fdstat_set_flags")
    }

    fn fd_fdstat_set_rights(
        &mut self,
        _fd: types::Fd,
        _fs_rights_base: types::Rights,
        _fs_rights_inherting: types::Rights,
    ) -> Result<()> {
        unimplemented!("fd_fdstat_set_rights")
    }

    fn fd_filestat_get(&mut self, _fd: types::Fd) -> Result<types::Filestat> {
        unimplemented!("fd_filestat_get")
    }

    fn fd_filestat_set_size(&mut self, _fd: types::Fd, _size: types::Filesize) -> Result<()> {
        unimplemented!("fd_filestat_set_size")
    }

    fn fd_filestat_set_times(
        &mut self,
        _fd: types::Fd,
        _atim: types::Timestamp,
        _mtim: types::Timestamp,
        _fst_flags: types::Fstflags,
    ) -> Result<()> {
        unimplemented!("fd_filestat_set_times")
    }

    fn fd_pread(
        &mut self,
        _fd: types::Fd,
        _iovs: &types::IovecArray<'_>,
        _offset: types::Filesize,
    ) -> Result<types::Size> {
        unimplemented!("fd_pread")
    }

    fn fd_prestat_get(&mut self, _fd: types::Fd) -> Result<types::Prestat> {
        unimplemented!("fd_prestat_get")
    }

    fn fd_prestat_dir_name(
        &mut self,
        _fd: types::Fd,
        _path: GuestPtrMut<u8>,
        _path_len: types::Size,
    ) -> Result<()> {
        unimplemented!("fd_prestat_dir_name")
    }

    fn fd_pwrite(
        &mut self,
        _fd: types::Fd,
        _ciovs: &types::CiovecArray<'_>,
        _offset: types::Filesize,
    ) -> Result<types::Size> {
        unimplemented!("fd_pwrite")
    }

    fn fd_read(&mut self, _fd: types::Fd, _iovs: &types::IovecArray<'_>) -> Result<types::Size> {
        unimplemented!("fd_read")
    }

    fn fd_readdir(
        &mut self,
        _fd: types::Fd,
        _buf: GuestPtrMut<u8>,
        _buf_len: types::Size,
        _cookie: types::Dircookie,
    ) -> Result<types::Size> {
        unimplemented!("fd_readdir")
    }

    fn fd_renumber(&mut self, _fd: types::Fd, _to: types::Fd) -> Result<()> {
        unimplemented!("fd_renumber")
    }

    fn fd_seek(
        &mut self,
        _fd: types::Fd,
        _offset: types::Filedelta,
        _whence: types::Whence,
    ) -> Result<types::Filesize> {
        unimplemented!("fd_seek")
    }

    fn fd_sync(&mut self, _fd: types::Fd) -> Result<()> {
        unimplemented!("fd_sync")
    }

    fn fd_tell(&mut self, _fd: types::Fd) -> Result<types::Filesize> {
        unimplemented!("fd_tell")
    }

    fn fd_write(&mut self, _fd: types::Fd, _ciovs: &types::CiovecArray<'_>) -> Result<types::Size> {
        unimplemented!("fd_write")
    }

    fn path_create_directory(&mut self, _fd: types::Fd, _path: &GuestString<'_>) -> Result<()> {
        unimplemented!("path_create_directory")
    }

    fn path_filestat_get(
        &mut self,
        _fd: types::Fd,
        _flags: types::Lookupflags,
        _path: &GuestString<'_>,
    ) -> Result<types::Filestat> {
        unimplemented!("path_filestat_get")
    }

    fn path_filestat_set_times(
        &mut self,
        _fd: types::Fd,
        _flags: types::Lookupflags,
        _path: &GuestString<'_>,
        _atim: types::Timestamp,
        _mtim: types::Timestamp,
        _fst_flags: types::Fstflags,
    ) -> Result<()> {
        unimplemented!("path_filestat_set_times")
    }

    fn path_link(
        &mut self,
        _old_fd: types::Fd,
        _old_flags: types::Lookupflags,
        _old_path: &GuestString<'_>,
        _new_fd: types::Fd,
        _new_path: &GuestString<'_>,
    ) -> Result<()> {
        unimplemented!("path_link")
    }

    fn path_open(
        &mut self,
        _fd: types::Fd,
        _dirflags: types::Lookupflags,
        _path: &GuestString<'_>,
        _oflags: types::Oflags,
        _fs_rights_base: types::Rights,
        _fs_rights_inherting: types::Rights,
        _fdflags: types::Fdflags,
    ) -> Result<types::Fd> {
        unimplemented!("path_open")
    }

    fn path_readlink(
        &mut self,
        _fd: types::Fd,
        _path: &GuestString<'_>,
        _buf: GuestPtrMut<u8>,
        _buf_len: types::Size,
    ) -> Result<types::Size> {
        unimplemented!("path_readlink")
    }

    fn path_remove_directory(&mut self, _fd: types::Fd, _path: &GuestString<'_>) -> Result<()> {
        unimplemented!("path_remove_directory")
    }

    fn path_rename(
        &mut self,
        _fd: types::Fd,
        _old_path: &GuestString<'_>,
        _new_fd: types::Fd,
        _new_path: &GuestString<'_>,
    ) -> Result<()> {
        unimplemented!("path_rename")
    }

    fn path_symlink(
        &mut self,
        _old_path: &GuestString<'_>,
        _fd: types::Fd,
        _new_path: &GuestString<'_>,
    ) -> Result<()> {
        unimplemented!("path_symlink")
    }

    fn path_unlink_file(&mut self, _fd: types::Fd, _path: &GuestString<'_>) -> Result<()> {
        unimplemented!("path_unlink_file")
    }

    fn poll_oneoff(
        &mut self,
        _in_: GuestPtr<types::Subscription>,
        _out: GuestPtrMut<types::Event>,
        _nsubscriptions: types::Size,
    ) -> Result<types::Size> {
        unimplemented!("poll_oneoff")
    }

    fn proc_exit(&mut self, _rval: types::Exitcode) -> std::result::Result<(), ()> {
        unimplemented!("proc_exit")
    }

    fn proc_raise(&mut self, _sig: types::Signal) -> Result<()> {
        unimplemented!("proc_raise")
    }

    fn sched_yield(&mut self) -> Result<()> {
        unimplemented!("sched_yield")
    }

    fn random_get(&mut self, _buf: GuestPtrMut<u8>, _buf_len: types::Size) -> Result<()> {
        unimplemented!("random_get")
    }

    fn sock_recv(
        &mut self,
        _fd: types::Fd,
        _ri_data: &types::IovecArray<'_>,
        _ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags)> {
        unimplemented!("sock_recv")
    }

    fn sock_send(
        &mut self,
        _fd: types::Fd,
        _si_data: &types::CiovecArray<'_>,
        _si_flags: types::Siflags,
    ) -> Result<types::Size> {
        unimplemented!("sock_send")
    }

    fn sock_shutdown(&mut self, _fd: types::Fd, _how: types::Sdflags) -> Result<()> {
        unimplemented!("sock_shutdown")
    }
}

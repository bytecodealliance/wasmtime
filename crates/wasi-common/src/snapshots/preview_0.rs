use crate::snapshots::preview_1::types as snapshot1_types;
use crate::snapshots::preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1 as Snapshot1;
use crate::{Error, ErrorExt, WasiCtx};
use std::convert::{TryFrom, TryInto};
use tracing::debug;
use wiggle::GuestPtr;

wiggle::from_witx!({
    witx: ["$WASI_ROOT/phases/old/snapshot_0/witx/wasi_unstable.witx"],
    ctx: WasiCtx,
    errors: { errno => Error },
});

impl wiggle::GuestErrorType for types::Errno {
    fn success() -> Self {
        Self::Success
    }
}

impl types::GuestErrorConversion for WasiCtx {
    fn into_errno(&self, e: wiggle::GuestError) -> types::Errno {
        debug!("Guest error: {:?}", e);
        let snapshot1_errno: snapshot1_types::Errno = e.into();
        snapshot1_errno.into()
    }
}

impl types::UserErrorConversion for WasiCtx {
    fn errno_from_error(&self, e: Error) -> Result<types::Errno, wiggle::Trap> {
        debug!("Error: {:?}", e);
        e.try_into()
            .map_err(|e| wiggle::Trap::String(format!("{:?}", e)))
    }
}

impl TryFrom<Error> for types::Errno {
    type Error = Error;
    fn try_from(e: Error) -> Result<types::Errno, Error> {
        let snapshot1_errno: snapshot1_types::Errno = e.try_into()?;
        Ok(snapshot1_errno.into())
    }
}

impl From<snapshot1_types::Errno> for types::Errno {
    fn from(e: snapshot1_types::Errno) -> types::Errno {
        match e {
            snapshot1_types::Errno::Success => types::Errno::Success,
            _ => todo!(),
        }
    }
}

impl From<types::Fd> for snapshot1_types::Fd {
    fn from(fd: types::Fd) -> snapshot1_types::Fd {
        u32::from(fd).into()
    }
}
impl From<snapshot1_types::Fd> for types::Fd {
    fn from(fd: snapshot1_types::Fd) -> types::Fd {
        u32::from(fd).into()
    }
}

macro_rules! convert_enum {
    ($from:ty, $to:ty, $($var:ident),+) => {
        impl From<$from> for $to {
            fn from(e: $from) -> $to {
                match e {
                    $( <$from>::$var => <$to>::$var, )+
                }
            }
        }
    }
}

convert_enum!(
    types::Clockid,
    snapshot1_types::Clockid,
    Realtime,
    Monotonic,
    ProcessCputimeId,
    ThreadCputimeId
);

convert_enum!(
    types::Advice,
    snapshot1_types::Advice,
    Normal,
    Sequential,
    Random,
    Willneed,
    Dontneed,
    Noreuse
);

convert_enum!(
    snapshot1_types::Filetype,
    types::Filetype,
    Directory,
    BlockDevice,
    CharacterDevice,
    RegularFile,
    SocketDgram,
    SocketStream,
    SymbolicLink,
    Unknown
);

convert_enum!(types::Whence, snapshot1_types::Whence, Cur, End, Set);

impl From<snapshot1_types::Prestat> for types::Prestat {
    fn from(p: snapshot1_types::Prestat) -> types::Prestat {
        match p {
            snapshot1_types::Prestat::Dir(d) => types::Prestat::Dir(d.into()),
        }
    }
}

macro_rules! convert_struct {
    ($from:ty, $to:path, $($field:ident),+) => {
        impl From<$from> for $to {
            fn from(e: $from) -> $to {
                $to {
                    $( $field: e.$field.into(), )+
                }
            }
        }
    }
}

convert_struct!(snapshot1_types::PrestatDir, types::PrestatDir, pr_name_len);

convert_struct!(
    snapshot1_types::Fdstat,
    types::Fdstat,
    fs_filetype,
    fs_rights_base,
    fs_rights_inheriting,
    fs_flags
);

impl From<snapshot1_types::Filestat> for types::Filestat {
    fn from(f: snapshot1_types::Filestat) -> types::Filestat {
        types::Filestat {
            dev: f.dev.into(),
            ino: f.ino.into(),
            filetype: f.filetype.into(),
            nlink: f.nlink.try_into().unwrap_or(u32::MAX),
            size: f.size.into(),
            atim: f.atim.into(),
            mtim: f.mtim.into(),
            ctim: f.ctim.into(),
        }
    }
}

macro_rules! convert_flags {
    ($from:ty, $to:ty, $($flag:ident),+) => {
        impl From<$from> for $to {
            fn from(f: $from) -> $to {
                let mut out = <$to>::empty();
                $(
                    if f.contains(<$from>::$flag) {
                        out |= <$to>::$flag;
                    }
                )+
                out
            }
        }
    }
}

macro_rules! convert_flags_bidirectional {
    ($from:ty, $to:ty, $($rest:tt)*) => {
        convert_flags!($from, $to, $($rest)*);
        convert_flags!($to, $from, $($rest)*);
    }
}

convert_flags_bidirectional!(
    snapshot1_types::Fdflags,
    types::Fdflags,
    APPEND,
    DSYNC,
    NONBLOCK,
    RSYNC,
    SYNC
);

convert_flags!(
    types::Lookupflags,
    snapshot1_types::Lookupflags,
    SYMLINK_FOLLOW
);

convert_flags!(
    types::Fstflags,
    snapshot1_types::Fstflags,
    ATIM,
    ATIM_NOW,
    MTIM,
    MTIM_NOW
);

convert_flags!(
    types::Oflags,
    snapshot1_types::Oflags,
    CREAT,
    DIRECTORY,
    EXCL,
    TRUNC
);

convert_flags_bidirectional!(
    types::Rights,
    snapshot1_types::Rights,
    FD_DATASYNC,
    FD_READ,
    FD_SEEK,
    FD_FDSTAT_SET_FLAGS,
    FD_SYNC,
    FD_TELL,
    FD_WRITE,
    FD_ADVISE,
    FD_ALLOCATE,
    PATH_CREATE_DIRECTORY,
    PATH_CREATE_FILE,
    PATH_LINK_SOURCE,
    PATH_LINK_TARGET,
    PATH_OPEN,
    FD_READDIR,
    PATH_READLINK,
    PATH_RENAME_SOURCE,
    PATH_RENAME_TARGET,
    PATH_FILESTAT_GET,
    PATH_FILESTAT_SET_SIZE,
    PATH_FILESTAT_SET_TIMES,
    FD_FILESTAT_GET,
    FD_FILESTAT_SET_SIZE,
    FD_FILESTAT_SET_TIMES,
    PATH_SYMLINK,
    PATH_REMOVE_DIRECTORY,
    PATH_UNLINK_FILE,
    POLL_FD_READWRITE,
    SOCK_SHUTDOWN
);

impl<'a> wasi_unstable::WasiUnstable for WasiCtx {
    fn args_get<'b>(
        &self,
        argv: &GuestPtr<'b, GuestPtr<'b, u8>>,
        argv_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), Error> {
        Snapshot1::args_get(self, argv, argv_buf)
    }

    fn args_sizes_get(&self) -> Result<(types::Size, types::Size), Error> {
        Snapshot1::args_sizes_get(self)
    }

    fn environ_get<'b>(
        &self,
        environ: &GuestPtr<'b, GuestPtr<'b, u8>>,
        environ_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), Error> {
        Snapshot1::environ_get(self, environ, environ_buf)
    }

    fn environ_sizes_get(&self) -> Result<(types::Size, types::Size), Error> {
        Snapshot1::environ_sizes_get(self)
    }

    fn clock_res_get(&self, id: types::Clockid) -> Result<types::Timestamp, Error> {
        Snapshot1::clock_res_get(self, id.into())
    }

    fn clock_time_get(
        &self,
        id: types::Clockid,
        precision: types::Timestamp,
    ) -> Result<types::Timestamp, Error> {
        Snapshot1::clock_time_get(self, id.into(), precision)
    }

    fn fd_advise(
        &self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<(), Error> {
        Snapshot1::fd_advise(self, fd.into(), offset, len, advice.into())
    }

    fn fd_allocate(
        &self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
    ) -> Result<(), Error> {
        Snapshot1::fd_allocate(self, fd.into(), offset, len)
    }

    fn fd_close(&self, fd: types::Fd) -> Result<(), Error> {
        Snapshot1::fd_close(self, fd.into())
    }

    fn fd_datasync(&self, fd: types::Fd) -> Result<(), Error> {
        Snapshot1::fd_datasync(self, fd.into())
    }

    fn fd_fdstat_get(&self, fd: types::Fd) -> Result<types::Fdstat, Error> {
        Ok(Snapshot1::fd_fdstat_get(self, fd.into())?.into())
    }

    fn fd_fdstat_set_flags(&self, fd: types::Fd, flags: types::Fdflags) -> Result<(), Error> {
        Snapshot1::fd_fdstat_set_flags(self, fd.into(), flags.into())
    }

    fn fd_fdstat_set_rights(
        &self,
        fd: types::Fd,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
    ) -> Result<(), Error> {
        Snapshot1::fd_fdstat_set_rights(
            self,
            fd.into(),
            fs_rights_base.into(),
            fs_rights_inheriting.into(),
        )
    }

    fn fd_filestat_get(&self, fd: types::Fd) -> Result<types::Filestat, Error> {
        Ok(Snapshot1::fd_filestat_get(self, fd.into())?.into())
    }

    fn fd_filestat_set_size(&self, fd: types::Fd, size: types::Filesize) -> Result<(), Error> {
        Snapshot1::fd_filestat_set_size(self, fd.into(), size)
    }

    fn fd_filestat_set_times(
        &self,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        Snapshot1::fd_filestat_set_times(self, fd.into(), atim, mtim, fst_flags.into())
    }

    fn fd_read(&self, fd: types::Fd, iovs: &types::IovecArray<'_>) -> Result<types::Size, Error> {
        Snapshot1::fd_read(self, fd.into(), iovs)
    }

    fn fd_pread(
        &self,
        fd: types::Fd,
        iovs: &types::IovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        Snapshot1::fd_pread(self, fd.into(), iovs, offset)
    }

    fn fd_write(
        &self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'_>,
    ) -> Result<types::Size, Error> {
        Snapshot1::fd_write(self, fd.into(), ciovs)
    }

    fn fd_pwrite(
        &self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        Snapshot1::fd_pwrite(self, fd.into(), ciovs, offset)
    }

    fn fd_prestat_get(&self, fd: types::Fd) -> Result<types::Prestat, Error> {
        Ok(Snapshot1::fd_prestat_get(self, fd.into())?.into())
    }

    fn fd_prestat_dir_name(
        &self,
        fd: types::Fd,
        path: &GuestPtr<u8>,
        path_max_len: types::Size,
    ) -> Result<(), Error> {
        Snapshot1::fd_prestat_dir_name(self, fd.into(), path, path_max_len)
    }

    fn fd_renumber(&self, from: types::Fd, to: types::Fd) -> Result<(), Error> {
        Snapshot1::fd_renumber(self, from.into(), to.into())
    }

    fn fd_seek(
        &self,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize, Error> {
        Snapshot1::fd_seek(self, fd.into(), offset, whence.into())
    }

    fn fd_sync(&self, fd: types::Fd) -> Result<(), Error> {
        Snapshot1::fd_sync(self, fd.into())
    }

    fn fd_tell(&self, fd: types::Fd) -> Result<types::Filesize, Error> {
        Snapshot1::fd_tell(self, fd.into())
    }

    fn fd_readdir(
        &self,
        fd: types::Fd,
        buf: &GuestPtr<u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, Error> {
        Snapshot1::fd_readdir(self, fd.into(), buf, buf_len, cookie)
    }

    fn path_create_directory(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        Snapshot1::path_create_directory(self, dirfd.into(), path)
    }

    fn path_filestat_get(
        &self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
    ) -> Result<types::Filestat, Error> {
        Ok(Snapshot1::path_filestat_get(self, dirfd.into(), flags.into(), path)?.into())
    }

    fn path_filestat_set_times(
        &self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        Snapshot1::path_filestat_set_times(
            self,
            dirfd.into(),
            flags.into(),
            path,
            atim,
            mtim,
            fst_flags.into(),
        )
    }

    fn path_link(
        &self,
        src_fd: types::Fd,
        src_flags: types::Lookupflags,
        src_path: &GuestPtr<'_, str>,
        target_fd: types::Fd,
        target_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        Snapshot1::path_link(
            self,
            src_fd.into(),
            src_flags.into(),
            src_path,
            target_fd.into(),
            target_path,
        )
    }

    fn path_open(
        &self,
        dirfd: types::Fd,
        dirflags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
        oflags: types::Oflags,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
        fdflags: types::Fdflags,
    ) -> Result<types::Fd, Error> {
        Ok(Snapshot1::path_open(
            self,
            dirfd.into(),
            dirflags.into(),
            path,
            oflags.into(),
            fs_rights_base.into(),
            fs_rights_inheriting.into(),
            fdflags.into(),
        )?
        .into())
    }

    fn path_readlink(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
        buf: &GuestPtr<u8>,
        buf_len: types::Size,
    ) -> Result<types::Size, Error> {
        Snapshot1::path_readlink(self, dirfd.into(), path, buf, buf_len)
    }

    fn path_remove_directory(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        Snapshot1::path_remove_directory(self, dirfd.into(), path)
    }

    fn path_rename(
        &self,
        src_fd: types::Fd,
        src_path: &GuestPtr<'_, str>,
        dest_fd: types::Fd,
        dest_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        Snapshot1::path_rename(self, src_fd.into(), src_path, dest_fd.into(), dest_path)
    }

    fn path_symlink(
        &self,
        src_path: &GuestPtr<'_, str>,
        dirfd: types::Fd,
        dest_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        Snapshot1::path_symlink(self, src_path, dirfd.into(), dest_path)
    }

    fn path_unlink_file(&self, dirfd: types::Fd, path: &GuestPtr<'_, str>) -> Result<(), Error> {
        Snapshot1::path_unlink_file(self, dirfd.into(), path)
    }

    fn poll_oneoff(
        &self,
        subs: &GuestPtr<types::Subscription>,
        events: &GuestPtr<types::Event>,
        nsubscriptions: types::Size,
    ) -> Result<types::Size, Error> {
        Snapshot1::poll_oneoff(self, subs, events, nsubscriptions)
    }

    fn proc_exit(&self, status: types::Exitcode) -> wiggle::Trap {
        Snapshot1::proc_exit(self, status)
    }

    fn proc_raise(&self, _sig: types::Signal) -> Result<(), Error> {
        Err(Error::trap("proc_raise unsupported"))
    }

    fn sched_yield(&self) -> Result<(), Error> {
        Snapshot1::sched_yield(self)
    }

    fn random_get(&self, buf: &GuestPtr<u8>, buf_len: types::Size) -> Result<(), Error> {
        Snapshot1::random_get(self, buf, buf_len)
    }

    fn sock_recv(
        &self,
        _fd: types::Fd,
        _ri_data: &types::IovecArray<'_>,
        _ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags), Error> {
        Err(Error::trap("sock_recv unsupported"))
    }

    fn sock_send(
        &self,
        _fd: types::Fd,
        _si_data: &types::CiovecArray<'_>,
        _si_flags: types::Siflags,
    ) -> Result<types::Size, Error> {
        Err(Error::trap("sock_send unsupported"))
    }

    fn sock_shutdown(&self, _fd: types::Fd, _how: types::Sdflags) -> Result<(), Error> {
        Err(Error::trap("sock_shutdown unsupported"))
    }
}

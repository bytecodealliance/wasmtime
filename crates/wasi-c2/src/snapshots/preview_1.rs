#![allow(unused_variables)]
use crate::file::{FileCaps, FileEntry};
use crate::{Error, WasiCtx};
use std::cell::RefMut;
use tracing::debug;
use wiggle::GuestPtr;

wiggle::from_witx!({
    witx: ["$WASI_ROOT/phases/snapshot/witx/wasi_snapshot_preview1.witx"],
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
        e.into()
    }
}

impl types::UserErrorConversion for WasiCtx {
    fn errno_from_error(&self, e: Error) -> types::Errno {
        debug!("Error: {:?}", e);
        e.into()
    }
}

impl From<Error> for types::Errno {
    fn from(e: Error) -> types::Errno {
        use types::Errno;
        match e {
            Error::Guest(e) => e.into(),
            Error::TryFromInt(_) => Errno::Overflow,
            Error::Utf8(_) => Errno::Ilseq,
            Error::UnexpectedIo(_) => Errno::Io,
            Error::GetRandom(_) => Errno::Io,
            Error::TooBig => Errno::TooBig,
            Error::Acces => Errno::Acces,
            Error::Badf => Errno::Badf,
            Error::Busy => Errno::Busy,
            Error::Exist => Errno::Exist,
            Error::Fault => Errno::Fault,
            Error::Fbig => Errno::Fbig,
            Error::Ilseq => Errno::Ilseq,
            Error::Inval => Errno::Inval,
            Error::Io => Errno::Io,
            Error::Isdir => Errno::Isdir,
            Error::Loop => Errno::Loop,
            Error::Mfile => Errno::Mfile,
            Error::Mlink => Errno::Mlink,
            Error::Nametoolong => Errno::Nametoolong,
            Error::Nfile => Errno::Nfile,
            Error::Noent => Errno::Noent,
            Error::Nomem => Errno::Nomem,
            Error::Nospc => Errno::Nospc,
            Error::Notdir => Errno::Notdir,
            Error::Notempty => Errno::Notempty,
            Error::Notsup => Errno::Notsup,
            Error::Overflow => Errno::Overflow,
            Error::Pipe => Errno::Pipe,
            Error::Perm => Errno::Perm,
            Error::Spipe => Errno::Spipe,
            Error::FileNotCapable { .. } => Errno::Notcapable,
        }
    }
}

impl From<wiggle::GuestError> for types::Errno {
    fn from(err: wiggle::GuestError) -> Self {
        use wiggle::GuestError::*;
        match err {
            InvalidFlagValue { .. } => Self::Inval,
            InvalidEnumValue { .. } => Self::Inval,
            PtrOverflow { .. } => Self::Fault,
            PtrOutOfBounds { .. } => Self::Fault,
            PtrNotAligned { .. } => Self::Inval,
            PtrBorrowed { .. } => Self::Fault,
            InvalidUtf8 { .. } => Self::Ilseq,
            TryFromIntError { .. } => Self::Overflow,
            InFunc { .. } => Self::Inval,
            InDataField { .. } => Self::Inval,
            SliceLengthsDiffer { .. } => Self::Fault,
            BorrowCheckerOutOfHandles { .. } => Self::Fault,
        }
    }
}

impl<'a> wasi_snapshot_preview1::WasiSnapshotPreview1 for WasiCtx {
    fn args_get<'b>(
        &self,
        argv: &GuestPtr<'b, GuestPtr<'b, u8>>,
        argv_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn args_sizes_get(&self) -> Result<(types::Size, types::Size), Error> {
        unimplemented!()
    }

    fn environ_get<'b>(
        &self,
        environ: &GuestPtr<'b, GuestPtr<'b, u8>>,
        environ_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn environ_sizes_get(&self) -> Result<(types::Size, types::Size), Error> {
        unimplemented!()
    }

    fn clock_res_get(&self, id: types::Clockid) -> Result<types::Timestamp, Error> {
        unimplemented!()
    }

    fn clock_time_get(
        &self,
        id: types::Clockid,
        _precision: types::Timestamp,
    ) -> Result<types::Timestamp, Error> {
        unimplemented!()
    }

    fn fd_advise(
        &self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<(), Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::ADVISE)?;
        f.advise(offset, len, advice.into())?;
        Ok(())
    }

    fn fd_allocate(
        &self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn fd_close(&self, fd: types::Fd) -> Result<(), Error> {
        unimplemented!()
    }

    fn fd_datasync(&self, fd: types::Fd) -> Result<(), Error> {
        unimplemented!()
    }

    fn fd_fdstat_get(&self, fd: types::Fd) -> Result<types::Fdstat, Error> {
        unimplemented!()
    }

    fn fd_fdstat_set_flags(&self, fd: types::Fd, flags: types::Fdflags) -> Result<(), Error> {
        unimplemented!()
    }

    fn fd_fdstat_set_rights(
        &self,
        fd: types::Fd,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn fd_filestat_get(&self, fd: types::Fd) -> Result<types::Filestat, Error> {
        unimplemented!()
    }

    fn fd_filestat_set_size(&self, fd: types::Fd, size: types::Filesize) -> Result<(), Error> {
        unimplemented!()
    }

    fn fd_filestat_set_times(
        &self,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn fd_read(&self, fd: types::Fd, iovs: &types::IovecArray<'_>) -> Result<types::Size, Error> {
        unimplemented!()
    }

    fn fd_pread(
        &self,
        fd: types::Fd,
        iovs: &types::IovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        unimplemented!()
    }

    fn fd_write(
        &self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'_>,
    ) -> Result<types::Size, Error> {
        unimplemented!()
    }

    fn fd_pwrite(
        &self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        unimplemented!()
    }

    fn fd_prestat_get(&self, fd: types::Fd) -> Result<types::Prestat, Error> {
        unimplemented!()
    }

    fn fd_prestat_dir_name(
        &self,
        fd: types::Fd,
        path: &GuestPtr<u8>,
        path_len: types::Size,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn fd_readdir(
        &self,
        fd: types::Fd,
        buf: &GuestPtr<u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, Error> {
        unimplemented!()
    }

    fn fd_renumber(&self, from: types::Fd, to: types::Fd) -> Result<(), Error> {
        unimplemented!()
    }

    fn fd_seek(
        &self,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize, Error> {
        unimplemented!()
    }

    fn fd_sync(&self, fd: types::Fd) -> Result<(), Error> {
        unimplemented!()
    }

    fn fd_tell(&self, fd: types::Fd) -> Result<types::Filesize, Error> {
        unimplemented!()
    }

    fn path_create_directory(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn path_filestat_get(
        &self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
    ) -> Result<types::Filestat, Error> {
        unimplemented!()
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
        unimplemented!()
    }

    fn path_link(
        &self,
        old_fd: types::Fd,
        old_flags: types::Lookupflags,
        old_path: &GuestPtr<'_, str>,
        new_fd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        unimplemented!()
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
        unimplemented!()
    }

    fn path_readlink(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
        buf: &GuestPtr<u8>,
        buf_len: types::Size,
    ) -> Result<types::Size, Error> {
        unimplemented!()
    }

    fn path_remove_directory(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn path_rename(
        &self,
        old_fd: types::Fd,
        old_path: &GuestPtr<'_, str>,
        new_fd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn path_symlink(
        &self,
        old_path: &GuestPtr<'_, str>,
        dirfd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn path_unlink_file(&self, dirfd: types::Fd, path: &GuestPtr<'_, str>) -> Result<(), Error> {
        unimplemented!()
    }

    fn poll_oneoff(
        &self,
        subs: &GuestPtr<types::Subscription>,
        events: &GuestPtr<types::Event>,
        nsubscriptions: types::Size,
    ) -> Result<types::Size, Error> {
        unimplemented!()
    }

    fn proc_exit(&self, _rval: types::Exitcode) -> Result<(), ()> {
        unimplemented!()
    }

    fn proc_raise(&self, _sig: types::Signal) -> Result<(), Error> {
        unimplemented!()
    }

    fn sched_yield(&self) -> Result<(), Error> {
        unimplemented!()
    }

    fn random_get(&self, buf: &GuestPtr<u8>, buf_len: types::Size) -> Result<(), Error> {
        unimplemented!()
    }

    fn sock_recv(
        &self,
        _fd: types::Fd,
        _ri_data: &types::IovecArray<'_>,
        _ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags), Error> {
        unimplemented!()
    }

    fn sock_send(
        &self,
        _fd: types::Fd,
        _si_data: &types::CiovecArray<'_>,
        _si_flags: types::Siflags,
    ) -> Result<types::Size, Error> {
        unimplemented!()
    }

    fn sock_shutdown(&self, _fd: types::Fd, _how: types::Sdflags) -> Result<(), Error> {
        unimplemented!()
    }
}

impl From<types::Advice> for system_interface::fs::Advice {
    fn from(advice: types::Advice) -> system_interface::fs::Advice {
        match advice {
            types::Advice::Normal => system_interface::fs::Advice::Normal,
            types::Advice::Sequential => system_interface::fs::Advice::Sequential,
            types::Advice::Random => system_interface::fs::Advice::Random,
            types::Advice::Willneed => system_interface::fs::Advice::WillNeed,
            types::Advice::Dontneed => system_interface::fs::Advice::DontNeed,
            types::Advice::Noreuse => system_interface::fs::Advice::NoReuse,
        }
    }
}

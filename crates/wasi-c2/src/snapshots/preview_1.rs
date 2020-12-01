#![allow(unused_variables)]
use crate::file::{FileCaps, FileEntry, Filestat, FilestatSetTime, Filetype, OFlags};
use crate::{Error, WasiCtx};
use std::cell::RefMut;
use std::convert::TryFrom;
use std::ops::Deref;
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
    fn errno_from_error(&self, e: Error) -> Result<types::Errno, String> {
        debug!("Error: {:?}", e);
        Ok(e.into())
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
            Error::NotCapable => Errno::Notcapable,
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
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::ALLOCATE)?;
        f.allocate(offset, len)?;
        Ok(())
    }

    fn fd_close(&self, fd: types::Fd) -> Result<(), Error> {
        let mut table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::CLOSE)?;
        drop(f);
        drop(file_entry);
        let _ = table.delete(u32::from(fd));
        Ok(())
    }

    fn fd_datasync(&self, fd: types::Fd) -> Result<(), Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::DATASYNC)?;
        f.datasync()?;
        Ok(())
    }

    fn fd_fdstat_get(&self, fd: types::Fd) -> Result<types::Fdstat, Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        Ok(types::Fdstat::from(file_entry.deref()))
    }

    fn fd_fdstat_set_flags(&self, fd: types::Fd, flags: types::Fdflags) -> Result<(), Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::FDSTAT_SET_FLAGS)?;
        f.set_oflags(OFlags::try_from(flags)?)?;
        Ok(())
    }

    fn fd_fdstat_set_rights(
        &self,
        fd: types::Fd,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
    ) -> Result<(), Error> {
        let table = self.table();
        let mut file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let base_caps = FileCaps::try_from(&fs_rights_base)?;
        let inheriting_caps = FileCaps::try_from(&fs_rights_inheriting)?;
        if file_entry.base_caps.contains(&base_caps)
            && file_entry.inheriting_caps.contains(&inheriting_caps)
        {
            file_entry.base_caps = base_caps;
            file_entry.inheriting_caps = inheriting_caps;
            Ok(())
        } else {
            Err(Error::NotCapable)
        }
    }

    fn fd_filestat_get(&self, fd: types::Fd) -> Result<types::Filestat, Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::FILESTAT_GET)?;
        let filestat = f.filestat_get()?;
        Ok(filestat.into())
    }

    fn fd_filestat_set_size(&self, fd: types::Fd, size: types::Filesize) -> Result<(), Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::FILESTAT_SET_SIZE)?;
        f.filestat_set_size(size)?;
        Ok(())
    }

    fn fd_filestat_set_times(
        &self,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        use std::time::{Duration, UNIX_EPOCH};
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::FILESTAT_SET_TIMES)?;

        // Validate flags, transform into well-structured arguments
        let set_atim = fst_flags.contains(&types::Fstflags::ATIM);
        let set_atim_now = fst_flags.contains(&types::Fstflags::ATIM_NOW);
        let set_mtim = fst_flags.contains(&types::Fstflags::MTIM);
        let set_mtim_now = fst_flags.contains(&types::Fstflags::MTIM_NOW);
        if (set_atim && set_atim_now) || (set_mtim && set_mtim_now) {
            return Err(Error::Inval);
        }
        let atim = if set_atim {
            Some(FilestatSetTime::Absolute(
                UNIX_EPOCH + Duration::from_nanos(atim),
            ))
        } else if set_atim_now {
            Some(FilestatSetTime::Now)
        } else {
            None
        };
        let mtim = if set_mtim {
            Some(FilestatSetTime::Absolute(
                UNIX_EPOCH + Duration::from_nanos(mtim),
            ))
        } else if set_mtim_now {
            Some(FilestatSetTime::Now)
        } else {
            None
        };

        f.filestat_set_times(atim, mtim)?;
        Ok(())
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

impl From<&FileEntry> for types::Fdstat {
    fn from(entry: &FileEntry) -> types::Fdstat {
        types::Fdstat {
            fs_filetype: types::Filetype::from(&entry.file.filetype()),
            fs_rights_base: types::Rights::from(&entry.base_caps),
            fs_rights_inheriting: types::Rights::from(&entry.base_caps),
            fs_flags: types::Fdflags::from(&entry.file.oflags()),
        }
    }
}

// FileCaps can always be represented as wasi Rights
impl From<&FileCaps> for types::Rights {
    fn from(caps: &FileCaps) -> types::Rights {
        todo!("translate FileCaps flags to Rights flags")
    }
}

// FileCaps are a subset of wasi Rights - not all Rights have a valid representation as FileCaps
impl TryFrom<&types::Rights> for FileCaps {
    type Error = Error;
    fn try_from(rights: &types::Rights) -> Result<FileCaps, Self::Error> {
        todo!("translate Rights flags to FileCaps flags")
    }
}

impl From<&Filetype> for types::Filetype {
    fn from(ft: &Filetype) -> types::Filetype {
        match ft {
            Filetype::BlockDevice => types::Filetype::BlockDevice,
            Filetype::CharacterDevice => types::Filetype::CharacterDevice,
            Filetype::RegularFile => types::Filetype::RegularFile,
            Filetype::SocketDgram => types::Filetype::SocketDgram,
            Filetype::SocketStream => types::Filetype::SocketStream,
        }
    }
}
impl From<&OFlags> for types::Fdflags {
    fn from(caps: &OFlags) -> types::Fdflags {
        todo!("translate OFlags flags to Fdflags flags")
    }
}

impl TryFrom<types::Fdflags> for OFlags {
    type Error = Error;
    fn try_from(fdflags: types::Fdflags) -> Result<OFlags, Self::Error> {
        todo!()
    }
}

impl From<Filestat> for types::Filestat {
    fn from(stat: Filestat) -> types::Filestat {
        todo!()
    }
}

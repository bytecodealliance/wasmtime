#![allow(unused_variables)]
use crate::dir::{DirCaps, DirEntry, ReaddirCursor, TableDirExt};
use crate::file::{FdFlags, FdStat, FileCaps, FileEntry, Filestat, Filetype, OFlags};
use crate::{Error, WasiCtx};
use fs_set_times::SystemTimeSpec;
use std::cell::RefMut;
use std::convert::TryFrom;
use std::io::{IoSlice, IoSliceMut};
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
            Error::DirNotCapable { .. } => Errno::Notcapable,
            Error::NotCapable => Errno::Notcapable,
            Error::TableOverflow => Errno::Overflow,
            Error::Unsupported { .. } => Errno::Notcapable, // XXX is this reasonable?
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
        self.args.write_to_guest(argv_buf, argv)
    }

    fn args_sizes_get(&self) -> Result<(types::Size, types::Size), Error> {
        Ok((self.args.number_elements(), self.args.cumulative_size()))
    }

    fn environ_get<'b>(
        &self,
        environ: &GuestPtr<'b, GuestPtr<'b, u8>>,
        environ_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), Error> {
        self.env.write_to_guest(environ_buf, environ)
    }

    fn environ_sizes_get(&self) -> Result<(types::Size, types::Size), Error> {
        Ok((self.env.number_elements(), self.env.cumulative_size()))
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
        let fd = u32::from(fd);

        // fd_close must close either a File or a Dir handle
        if table.is::<FileEntry>(fd) {
            let _ = table.delete(fd);
        } else if table.is::<DirEntry>(fd) {
            // We cannot close preopened directories
            let dir_entry: RefMut<DirEntry> = table.get(fd).unwrap();
            if dir_entry.preopen_path.is_some() {
                return Err(Error::Notsup);
            }
            drop(dir_entry);
            let _ = table.delete(fd);
        }

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
        let fdstat = file_entry.get_fdstat()?;
        Ok(types::Fdstat::from(&fdstat))
    }

    fn fd_fdstat_set_flags(&self, fd: types::Fd, flags: types::Fdflags) -> Result<(), Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::FDSTAT_SET_FLAGS)?;
        f.set_oflags(OFlags::try_from(&flags)?)?;
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
        let filestat = f.get_filestat()?;
        Ok(filestat.into())
    }

    fn fd_filestat_set_size(&self, fd: types::Fd, size: types::Filesize) -> Result<(), Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::FILESTAT_SET_SIZE)?;
        f.set_filestat_size(size)?;
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
            Some(SystemTimeSpec::Absolute(
                UNIX_EPOCH + Duration::from_nanos(atim),
            ))
        } else if set_atim_now {
            Some(SystemTimeSpec::SymbolicNow)
        } else {
            None
        };
        let mtim = if set_mtim {
            Some(SystemTimeSpec::Absolute(
                UNIX_EPOCH + Duration::from_nanos(mtim),
            ))
        } else if set_mtim_now {
            Some(SystemTimeSpec::SymbolicNow)
        } else {
            None
        };

        f.set_times(atim, mtim)?;
        Ok(())
    }

    fn fd_read(&self, fd: types::Fd, iovs: &types::IovecArray<'_>) -> Result<types::Size, Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::READ)?;

        let mut guest_slices: Vec<wiggle::GuestSliceMut<u8>> = iovs
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Iovec = iov_ptr.read()?;
                Ok(iov.buf.as_array(iov.buf_len).as_slice_mut()?)
            })
            .collect::<Result<_, Error>>()?;

        let mut ioslices: Vec<IoSliceMut> = guest_slices
            .iter_mut()
            .map(|s| IoSliceMut::new(&mut *s))
            .collect();

        let bytes_read = f.read_vectored(&mut ioslices)?;
        Ok(types::Size::try_from(bytes_read)?)
    }

    fn fd_pread(
        &self,
        fd: types::Fd,
        iovs: &types::IovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::READ | FileCaps::SEEK)?;

        let mut guest_slices: Vec<wiggle::GuestSliceMut<u8>> = iovs
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Iovec = iov_ptr.read()?;
                Ok(iov.buf.as_array(iov.buf_len).as_slice_mut()?)
            })
            .collect::<Result<_, Error>>()?;

        let mut ioslices: Vec<IoSliceMut> = guest_slices
            .iter_mut()
            .map(|s| IoSliceMut::new(&mut *s))
            .collect();

        let bytes_read = f.read_vectored_at(&mut ioslices, offset)?;
        Ok(types::Size::try_from(bytes_read)?)
    }

    fn fd_write(
        &self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'_>,
    ) -> Result<types::Size, Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::WRITE)?;

        let guest_slices: Vec<wiggle::GuestSlice<u8>> = ciovs
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Ciovec = iov_ptr.read()?;
                Ok(iov.buf.as_array(iov.buf_len).as_slice()?)
            })
            .collect::<Result<_, Error>>()?;

        let ioslices: Vec<IoSlice> = guest_slices
            .iter()
            .map(|s| IoSlice::new(s.deref()))
            .collect();
        let bytes_written = f.write_vectored(&ioslices)?;

        Ok(types::Size::try_from(bytes_written)?)
    }

    fn fd_pwrite(
        &self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::WRITE | FileCaps::SEEK)?;

        let guest_slices: Vec<wiggle::GuestSlice<u8>> = ciovs
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Ciovec = iov_ptr.read()?;
                Ok(iov.buf.as_array(iov.buf_len).as_slice()?)
            })
            .collect::<Result<_, Error>>()?;

        let ioslices: Vec<IoSlice> = guest_slices
            .iter()
            .map(|s| IoSlice::new(s.deref()))
            .collect();
        let bytes_written = f.write_vectored_at(&ioslices, offset)?;

        Ok(types::Size::try_from(bytes_written)?)
    }

    fn fd_prestat_get(&self, fd: types::Fd) -> Result<types::Prestat, Error> {
        let table = self.table();
        let dir_entry: RefMut<DirEntry> = table.get(u32::from(fd)).map_err(|_| Error::Badf)?;
        if let Some(ref preopen) = dir_entry.preopen_path {
            let path_str = preopen.to_str().ok_or(Error::Notsup)?;
            let pr_name_len =
                u32::try_from(path_str.as_bytes().len()).map_err(|_| Error::Overflow)?;
            Ok(types::Prestat::Dir(types::PrestatDir { pr_name_len }))
        } else {
            Err(Error::Notsup)
        }
    }

    fn fd_prestat_dir_name(
        &self,
        fd: types::Fd,
        path: &GuestPtr<u8>,
        path_max_len: types::Size,
    ) -> Result<(), Error> {
        let table = self.table();
        let dir_entry: RefMut<DirEntry> = table.get(u32::from(fd)).map_err(|_| Error::Notdir)?;
        if let Some(ref preopen) = dir_entry.preopen_path {
            let path_bytes = preopen.to_str().ok_or(Error::Notsup)?.as_bytes();
            let path_len = path_bytes.len();
            if path_len < path_max_len as usize {
                return Err(Error::Nametoolong);
            }
            let mut p_memory = path.as_array(path_len as u32).as_slice_mut()?;
            p_memory.copy_from_slice(path_bytes);
            Ok(())
        } else {
            Err(Error::Notsup)
        }
    }
    fn fd_renumber(&self, from: types::Fd, to: types::Fd) -> Result<(), Error> {
        let mut table = self.table();
        let from = u32::from(from);
        let to = u32::from(to);
        if !table.contains_key(from) {
            return Err(Error::Badf);
        }
        if table.is_preopen(from) {
            return Err(Error::Notsup);
        }
        if table.is_preopen(to) {
            return Err(Error::Notsup);
        }
        let from_entry = table
            .delete(from)
            .expect("we checked that table contains from");
        table.insert_at(to, from_entry);
        Ok(())
    }

    fn fd_seek(
        &self,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize, Error> {
        use std::io::SeekFrom;

        let required_caps = if offset == 0 && whence == types::Whence::Cur {
            FileCaps::TELL
        } else {
            FileCaps::TELL | FileCaps::SEEK
        };

        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(required_caps)?;
        let newoffset = f.seek(match whence {
            types::Whence::Cur => SeekFrom::Current(offset),
            types::Whence::End => SeekFrom::End(offset),
            types::Whence::Set => SeekFrom::Start(offset as u64),
        })?;
        Ok(newoffset)
    }

    fn fd_sync(&self, fd: types::Fd) -> Result<(), Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::SYNC)?;
        f.sync()?;
        Ok(())
    }

    fn fd_tell(&self, fd: types::Fd) -> Result<types::Filesize, Error> {
        let table = self.table();
        let file_entry: RefMut<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::TELL)?;
        let offset = f.seek(std::io::SeekFrom::Current(0))?;
        Ok(offset)
    }

    fn fd_readdir(
        &self,
        dirfd: types::Fd,
        buf: &GuestPtr<u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, Error> {
        let table = self.table();
        let dir_entry: RefMut<DirEntry> = table.get(u32::from(dirfd))?;
        let d = dir_entry.get_cap(DirCaps::READDIR)?;
        for pair in d.readdir(ReaddirCursor::from(cookie))? {
            let (entity, name) = pair?;
            todo!()
        }
        todo!()
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
        let mut table = self.table();
        let dir_entry: RefMut<DirEntry> = table.get(u32::from(dirfd))?;
        let dir = dir_entry.get_cap(DirCaps::OPEN)?;
        let symlink_follow = dirflags.contains(&types::Lookupflags::SYMLINK_FOLLOW);
        let path = path.as_str()?;
        if oflags.contains(&types::Oflags::DIRECTORY) {
            let create = oflags.contains(&types::Oflags::CREAT);
            let child_dir = dir.open_dir(symlink_follow, path.deref(), create)?;

            // XXX go back and check these caps conversions - probably need to validate them
            // against ???
            let base_caps = DirCaps::try_from(&fs_rights_base)?;
            let inheriting_caps = DirCaps::try_from(&fs_rights_inheriting)?;
            drop(dir);
            drop(dir_entry);
            let fd = table.push(DirEntry {
                dir: child_dir,
                base_caps,
                inheriting_caps,
                preopen_path: None,
            })?;
            Ok(types::Fd::from(fd))
        } else {
            let oflags = OFlags::try_from(&oflags)?;
            let fdflags = FdFlags::try_from(&fdflags)?;
            let file = dir.open_file(symlink_follow, path.deref(), oflags, fdflags)?;
            // XXX go back and check these caps conversions - probably need to validate them
            // against ???
            let base_caps = FileCaps::try_from(&fs_rights_base)?;
            let inheriting_caps = FileCaps::try_from(&fs_rights_inheriting)?;
            drop(dir);
            drop(dir_entry);
            let fd = table.push(FileEntry {
                file,
                base_caps,
                inheriting_caps,
            })?;
            Ok(types::Fd::from(fd))
        }
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

impl From<&FdStat> for types::Fdstat {
    fn from(fdstat: &FdStat) -> types::Fdstat {
        types::Fdstat {
            fs_filetype: types::Filetype::from(&fdstat.filetype),
            fs_rights_base: types::Rights::from(&fdstat.base_caps),
            fs_rights_inheriting: types::Rights::from(&fdstat.inheriting_caps),
            fs_flags: types::Fdflags::from(&fdstat.flags),
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

// DirCaps are a subset of wasi Rights - not all Rights have a valid representation as DirCaps
impl TryFrom<&types::Rights> for DirCaps {
    type Error = Error;
    fn try_from(rights: &types::Rights) -> Result<DirCaps, Self::Error> {
        todo!("translate Rights flags to DirCaps flags")
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
impl From<&FdFlags> for types::Fdflags {
    fn from(fdflags: &FdFlags) -> types::Fdflags {
        todo!("translate internal to Fdflags")
    }
}

impl TryFrom<&types::Oflags> for OFlags {
    type Error = Error;
    fn try_from(oflags: &types::Oflags) -> Result<OFlags, Self::Error> {
        if oflags.contains(&types::Oflags::DIRECTORY) {
            return Err(Error::Inval);
        }
        todo!("rest of oflags translation should be trivial - creat excl trunc")
    }
}

impl TryFrom<&types::Fdflags> for FdFlags {
    type Error = Error;
    fn try_from(fdflags: &types::Fdflags) -> Result<FdFlags, Self::Error> {
        todo!()
    }
}

impl TryFrom<&types::Fdflags> for OFlags {
    type Error = Error;
    fn try_from(fdflags: &types::Fdflags) -> Result<OFlags, Self::Error> {
        todo!()
    }
}

impl From<Filestat> for types::Filestat {
    fn from(stat: Filestat) -> types::Filestat {
        todo!()
    }
}

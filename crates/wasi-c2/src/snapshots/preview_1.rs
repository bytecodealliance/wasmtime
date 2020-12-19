#![allow(unused_variables)]
use crate::dir::{DirCaps, DirEntry, DirFdStat, ReaddirCursor, ReaddirEntity, TableDirExt};
use crate::file::{FdFlags, FdStat, FileCaps, FileEntry, FileType, Filestat, OFlags};
use crate::{Error, WasiCtx};
use fs_set_times::SystemTimeSpec;
use std::cell::{Ref, RefMut};
use std::convert::{TryFrom, TryInto};
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
        use std::io::ErrorKind;
        use types::Errno;
        match e {
            Error::Guest(e) => e.into(),
            Error::TryFromInt(_) => Errno::Overflow,
            Error::Utf8(_) => Errno::Ilseq,
            Error::UnexpectedIo(e) => match e.kind() {
                ErrorKind::NotFound => Errno::Noent,
                ErrorKind::PermissionDenied => Errno::Perm,
                ErrorKind::AlreadyExists => Errno::Exist,
                ErrorKind::InvalidInput => Errno::Ilseq,
                ErrorKind::ConnectionRefused
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
                | ErrorKind::AddrInUse
                | ErrorKind::AddrNotAvailable
                | ErrorKind::BrokenPipe
                | ErrorKind::WouldBlock
                | ErrorKind::InvalidData
                | ErrorKind::TimedOut
                | ErrorKind::WriteZero
                | ErrorKind::Interrupted
                | ErrorKind::Other
                | ErrorKind::UnexpectedEof
                | _ => Errno::Io,
            },
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
            InFunc { err, .. } => types::Errno::from(*err),
            InDataField { err, .. } => types::Errno::from(*err),
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
        let file_entry: Ref<FileEntry> = table.get(u32::from(fd))?;
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
        let file_entry: Ref<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::ALLOCATE)?;
        f.allocate(offset, len)?;
        Ok(())
    }

    fn fd_close(&self, fd: types::Fd) -> Result<(), Error> {
        let mut table = self.table();
        let fd = u32::from(fd);

        // Fail fast: If not present in table, Badf
        if !table.contains_key(fd) {
            return Err(Error::Badf);
        }
        // fd_close must close either a File or a Dir handle
        if table.is::<FileEntry>(fd) {
            let _ = table.delete(fd);
        } else if table.is::<DirEntry>(fd) {
            // We cannot close preopened directories
            let dir_entry: Ref<DirEntry> = table.get(fd).unwrap();
            if dir_entry.preopen_path().is_some() {
                return Err(Error::Notsup);
            }
            drop(dir_entry);
            let _ = table.delete(fd);
        } else {
            // XXX do we just table delete other entry types anyway?
            return Err(Error::Badf);
        }

        Ok(())
    }

    fn fd_datasync(&self, fd: types::Fd) -> Result<(), Error> {
        let table = self.table();
        let file_entry: Ref<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::DATASYNC)?;
        f.datasync()?;
        Ok(())
    }

    fn fd_fdstat_get(&self, fd: types::Fd) -> Result<types::Fdstat, Error> {
        let table = self.table();
        let fd = u32::from(fd);
        if table.is::<FileEntry>(fd) {
            let file_entry: Ref<FileEntry> = table.get(fd)?;
            let fdstat = file_entry.get_fdstat()?;
            Ok(types::Fdstat::from(&fdstat))
        } else if table.is::<DirEntry>(fd) {
            let dir_entry: Ref<DirEntry> = table.get(fd)?;
            let dir_fdstat = dir_entry.get_dir_fdstat();
            Ok(types::Fdstat::from(&dir_fdstat))
        } else {
            Err(Error::Badf)
        }
    }

    fn fd_fdstat_set_flags(&self, fd: types::Fd, flags: types::Fdflags) -> Result<(), Error> {
        let table = self.table();
        let file_entry: Ref<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::FDSTAT_SET_FLAGS)?;
        f.set_fdflags(FdFlags::from(&flags))?;
        Ok(())
    }

    fn fd_fdstat_set_rights(
        &self,
        fd: types::Fd,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
    ) -> Result<(), Error> {
        let table = self.table();
        let fd = u32::from(fd);
        if table.is::<FileEntry>(fd) {
            let mut file_entry: RefMut<FileEntry> = table.get_mut(fd)?;
            let file_caps = FileCaps::from(&fs_rights_base);
            file_entry.drop_caps_to(file_caps)
        } else if table.is::<DirEntry>(fd) {
            let mut dir_entry: RefMut<DirEntry> = table.get_mut(fd)?;
            let dir_caps = DirCaps::from(&fs_rights_base);
            let file_caps = FileCaps::from(&fs_rights_inheriting);
            dir_entry.drop_caps_to(dir_caps, file_caps)
        } else {
            Err(Error::Badf)
        }
    }

    fn fd_filestat_get(&self, fd: types::Fd) -> Result<types::Filestat, Error> {
        let table = self.table();
        let fd = u32::from(fd);
        if table.is::<FileEntry>(fd) {
            let file_entry: Ref<FileEntry> = table.get(fd)?;
            let f = file_entry.get_cap(FileCaps::FILESTAT_GET)?;
            let filestat = f.get_filestat()?;
            Ok(filestat.into())
        } else if table.is::<DirEntry>(fd) {
            let dir_entry: Ref<DirEntry> = table.get(fd)?;
            let d = dir_entry.get_cap(DirCaps::FILESTAT_GET)?;
            let filestat = d.get_filestat()?;
            Ok(filestat.into())
        } else {
            Err(Error::Badf)
        }
    }

    fn fd_filestat_set_size(&self, fd: types::Fd, size: types::Filesize) -> Result<(), Error> {
        let table = self.table();
        let file_entry: Ref<FileEntry> = table.get(u32::from(fd))?;
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

        let fd = u32::from(fd);
        let table = self.table();
        if table.is::<FileEntry>(fd) {
            let file_entry: Ref<FileEntry> = table.get(fd).unwrap();
            let f = file_entry.get_cap(FileCaps::FILESTAT_SET_TIMES)?;
            f.set_times(atim, mtim)?;
            Ok(())
        } else if table.is::<DirEntry>(fd) {
            let dir_entry: Ref<DirEntry> = table.get(fd).unwrap();
            let d = dir_entry.get_cap(DirCaps::FILESTAT_SET_TIMES)?;
            todo!("d.set_times(atim, mtim)?;")
        } else {
            Err(Error::Badf)
        }
    }

    fn fd_read(&self, fd: types::Fd, iovs: &types::IovecArray<'_>) -> Result<types::Size, Error> {
        let table = self.table();
        let file_entry: Ref<FileEntry> = table.get(u32::from(fd))?;
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
        let file_entry: Ref<FileEntry> = table.get(u32::from(fd))?;
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
        let file_entry: Ref<FileEntry> = table.get(u32::from(fd))?;
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
        let file_entry: Ref<FileEntry> = table.get(u32::from(fd))?;
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
        let dir_entry: Ref<DirEntry> = table.get(u32::from(fd)).map_err(|_| Error::Badf)?;
        if let Some(ref preopen) = dir_entry.preopen_path() {
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
        let dir_entry: Ref<DirEntry> = table.get(u32::from(fd)).map_err(|_| Error::Notdir)?;
        if let Some(ref preopen) = dir_entry.preopen_path() {
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
        let file_entry: Ref<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(required_caps)?;
        let whence = match whence {
            types::Whence::Cur => SeekFrom::Current(offset),
            types::Whence::End => SeekFrom::End(offset),
            types::Whence::Set => SeekFrom::Start(offset as u64),
        };
        let newoffset = f.seek(whence)?;
        Ok(newoffset)
    }

    fn fd_sync(&self, fd: types::Fd) -> Result<(), Error> {
        let table = self.table();
        let file_entry: Ref<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::SYNC)?;
        f.sync()?;
        Ok(())
    }

    fn fd_tell(&self, fd: types::Fd) -> Result<types::Filesize, Error> {
        let table = self.table();
        let file_entry: Ref<FileEntry> = table.get(u32::from(fd))?;
        let f = file_entry.get_cap(FileCaps::TELL)?;
        let offset = f.seek(std::io::SeekFrom::Current(0))?;
        Ok(offset)
    }

    fn fd_readdir(
        &self,
        fd: types::Fd,
        buf: &GuestPtr<u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, Error> {
        let table = self.table();
        let fd = u32::from(fd);
        debug!(
            "fd_readdir {} is a DirEntry: {}",
            fd,
            table.is::<DirEntry>(fd)
        );
        let dir_entry: Ref<DirEntry> = table.get(fd)?;
        let d = dir_entry.get_cap(DirCaps::READDIR)?;

        let mut bufused = 0;
        let mut buf = buf.clone();
        for pair in d.readdir(ReaddirCursor::from(cookie))? {
            let (entity, name) = pair?;
            let dirent_raw = dirent_bytes(types::Dirent::from(&entity));
            let dirent_len: types::Size = dirent_raw.len().try_into()?;
            let name_raw = name.as_bytes();
            let name_len: types::Size = name_raw.len().try_into()?;
            let offset = dirent_len.checked_add(name_len).ok_or(Error::Overflow)?;

            // Copy as many bytes of the dirent as we can, up to the end of the buffer
            let dirent_copy_len = std::cmp::min(dirent_len, buf_len - bufused);
            buf.as_array(dirent_copy_len)
                .copy_from_slice(&dirent_raw[..dirent_copy_len as usize])?;

            // If the dirent struct wasnt compied entirely, return that we filled the buffer, which
            // tells libc that we're not at EOF.
            if dirent_copy_len < dirent_len {
                return Ok(buf_len);
            }

            buf = buf.add(dirent_copy_len)?;

            // Copy as many bytes of the name as we can, up to the end of the buffer
            let name_copy_len = std::cmp::min(name_len, buf_len - bufused);
            buf.as_array(name_copy_len)
                .copy_from_slice(&name_raw[..name_copy_len as usize])?;

            // If the dirent struct wasn't copied entirely, return that we filled the buffer, which
            // tells libc that we're not at EOF

            if name_copy_len < name_len {
                return Ok(buf_len);
            }

            buf = buf.add(name_copy_len)?;
            bufused += offset;
        }
        Ok(bufused)
    }

    fn path_create_directory(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        let table = self.table();
        let dir_entry: Ref<DirEntry> = table.get(u32::from(dirfd))?;
        let dir = dir_entry.get_cap(DirCaps::CREATE_DIRECTORY)?;
        let path = path.as_str()?;
        dir.create_dir(path.deref())
    }

    fn path_filestat_get(
        &self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
    ) -> Result<types::Filestat, Error> {
        let table = self.table();
        let dir_entry: Ref<DirEntry> = table.get(u32::from(dirfd))?;
        let dir = dir_entry.get_cap(DirCaps::PATH_FILESTAT_GET)?;
        todo!()
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
        let table = self.table();
        let dir_entry: Ref<DirEntry> = table.get(u32::from(dirfd))?;
        let dir = dir_entry.get_cap(DirCaps::PATH_FILESTAT_SET_TIMES)?;
        todo!()
    }

    fn path_link(
        &self,
        src_fd: types::Fd,
        src_flags: types::Lookupflags,
        src_path: &GuestPtr<'_, str>,
        dest_fd: types::Fd,
        dest_path: &GuestPtr<'_, str>,
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
        let dir_entry: Ref<DirEntry> = table.get(u32::from(dirfd))?;

        let symlink_follow = dirflags.contains(&types::Lookupflags::SYMLINK_FOLLOW);

        let oflags = OFlags::from(&oflags);
        let fdflags = FdFlags::from(&fdflags);
        let path = path.as_str()?;
        if oflags.contains(&OFlags::DIRECTORY) {
            if oflags.contains(&OFlags::CREATE)
                || oflags.contains(&OFlags::EXCLUSIVE)
                || oflags.contains(&OFlags::TRUNCATE)
            {
                return Err(Error::Inval);
            }
            let dir = dir_entry.get_cap(DirCaps::OPEN)?;
            let child_dir = dir.open_dir(symlink_follow, path.deref())?;
            let dir_caps = dir_entry.child_dir_caps(DirCaps::from(&fs_rights_base));
            let file_caps = dir_entry.child_file_caps(FileCaps::from(&fs_rights_inheriting));
            drop(dir);
            drop(dir_entry);
            let fd = table.push(Box::new(DirEntry::new(
                dir_caps, file_caps, None, child_dir,
            )))?;
            Ok(types::Fd::from(fd))
        } else {
            let mut required_caps = DirCaps::OPEN;
            if oflags.contains(&OFlags::CREATE) {
                required_caps = required_caps | DirCaps::CREATE_FILE;
            }

            let dir = dir_entry.get_cap(required_caps)?;
            let file_caps = dir_entry.child_file_caps(FileCaps::from(&fs_rights_base));
            let file = dir.open_file(symlink_follow, path.deref(), oflags, file_caps, fdflags)?;
            drop(dir);
            drop(dir_entry);
            let fd = table.push(Box::new(FileEntry::new(file_caps, file)))?;
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
        let table = self.table();
        let dir_entry: Ref<DirEntry> = table.get(u32::from(dirfd))?;
        let dir = dir_entry.get_cap(DirCaps::READLINK)?;
        let path = path.as_str()?;
        let link = dir
            .read_link(path.deref())?
            .into_os_string()
            .into_string()
            .map_err(|_| Error::Utf8(todo!()))?;
        let link_bytes = link.as_bytes();
        let link_len = link_bytes.len();
        if link_len > buf_len as usize {
            return Err(Error::Nametoolong);
        }
        let mut buf = buf.as_array(link_len as u32).as_slice_mut()?;
        buf.copy_from_slice(link_bytes);
        Ok(link_len as types::Size)
    }

    fn path_remove_directory(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        let table = self.table();
        let dir_entry: Ref<DirEntry> = table.get(u32::from(dirfd))?;
        let dir = dir_entry.get_cap(DirCaps::REMOVE_DIRECTORY)?;
        let path = path.as_str()?;
        dir.remove_dir(path.deref())
    }

    fn path_rename(
        &self,
        old_fd: types::Fd,
        old_path: &GuestPtr<'_, str>,
        new_fd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        // XXX: Dir::rename requires (to_dir: &Self), but in the table we just have a dyn WasiDir.
        // The downcast isn't possible.
        unimplemented!()
    }

    fn path_symlink(
        &self,
        src_path: &GuestPtr<'_, str>,
        dirfd: types::Fd,
        dest_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        let table = self.table();
        let dir_entry: Ref<DirEntry> = table.get(u32::from(dirfd))?;
        let dir = dir_entry.get_cap(DirCaps::SYMLINK)?;
        let src_path = src_path.as_str()?;
        let dest_path = dest_path.as_str()?;
        dir.symlink(src_path.deref(), dest_path.deref())
    }

    fn path_unlink_file(&self, dirfd: types::Fd, path: &GuestPtr<'_, str>) -> Result<(), Error> {
        let table = self.table();
        let dir_entry: Ref<DirEntry> = table.get(u32::from(dirfd))?;
        let dir = dir_entry.get_cap(DirCaps::UNLINK_FILE)?;
        let path = path.as_str()?;
        dir.unlink_file(path.deref())
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
            fs_rights_base: types::Rights::from(&fdstat.caps),
            fs_rights_inheriting: types::Rights::empty(),
            fs_flags: types::Fdflags::from(&fdstat.flags),
        }
    }
}

impl From<&DirFdStat> for types::Fdstat {
    fn from(dirstat: &DirFdStat) -> types::Fdstat {
        let fs_rights_base = types::Rights::from(&dirstat.dir_caps);
        let fs_rights_inheriting = types::Rights::from(&dirstat.file_caps);
        types::Fdstat {
            fs_filetype: types::Filetype::Directory,
            fs_rights_base,
            fs_rights_inheriting,
            fs_flags: types::Fdflags::empty(),
        }
    }
}

// FileCaps can always be represented as wasi Rights
impl From<&FileCaps> for types::Rights {
    fn from(caps: &FileCaps) -> types::Rights {
        let mut rights = types::Rights::empty();
        if caps.contains(&FileCaps::DATASYNC) {
            rights = rights | types::Rights::FD_DATASYNC;
        }
        if caps.contains(&FileCaps::READ) {
            rights = rights | types::Rights::FD_READ;
        }
        if caps.contains(&FileCaps::SEEK) {
            rights = rights | types::Rights::FD_SEEK;
        }
        if caps.contains(&FileCaps::FDSTAT_SET_FLAGS) {
            rights = rights | types::Rights::FD_FDSTAT_SET_FLAGS;
        }
        if caps.contains(&FileCaps::SYNC) {
            rights = rights | types::Rights::FD_SYNC;
        }
        if caps.contains(&FileCaps::TELL) {
            rights = rights | types::Rights::FD_TELL;
        }
        if caps.contains(&FileCaps::WRITE) {
            rights = rights | types::Rights::FD_WRITE;
        }
        if caps.contains(&FileCaps::ADVISE) {
            rights = rights | types::Rights::FD_ADVISE;
        }
        if caps.contains(&FileCaps::ALLOCATE) {
            rights = rights | types::Rights::FD_ALLOCATE;
        }
        if caps.contains(&FileCaps::FILESTAT_GET) {
            rights = rights | types::Rights::FD_FILESTAT_GET;
        }
        if caps.contains(&FileCaps::FILESTAT_SET_SIZE) {
            rights = rights | types::Rights::FD_FILESTAT_SET_SIZE;
        }
        if caps.contains(&FileCaps::FILESTAT_SET_TIMES) {
            rights = rights | types::Rights::FD_FILESTAT_SET_TIMES;
        }
        rights
    }
}

// FileCaps are a subset of wasi Rights - not all Rights have a valid representation as FileCaps
impl From<&types::Rights> for FileCaps {
    fn from(rights: &types::Rights) -> FileCaps {
        let mut caps = FileCaps::empty();
        if rights.contains(&types::Rights::FD_DATASYNC) {
            caps = caps | FileCaps::DATASYNC;
        }
        if rights.contains(&types::Rights::FD_READ) {
            caps = caps | FileCaps::READ;
        }
        if rights.contains(&types::Rights::FD_SEEK) {
            caps = caps | FileCaps::SEEK;
        }
        if rights.contains(&types::Rights::FD_FDSTAT_SET_FLAGS) {
            caps = caps | FileCaps::FDSTAT_SET_FLAGS;
        }
        if rights.contains(&types::Rights::FD_SYNC) {
            caps = caps | FileCaps::SYNC;
        }
        if rights.contains(&types::Rights::FD_TELL) {
            caps = caps | FileCaps::TELL;
        }
        if rights.contains(&types::Rights::FD_WRITE) {
            caps = caps | FileCaps::WRITE;
        }
        if rights.contains(&types::Rights::FD_ADVISE) {
            caps = caps | FileCaps::ADVISE;
        }
        if rights.contains(&types::Rights::FD_ALLOCATE) {
            caps = caps | FileCaps::ALLOCATE;
        }
        if rights.contains(&types::Rights::FD_FILESTAT_GET) {
            caps = caps | FileCaps::FILESTAT_GET;
        }
        if rights.contains(&types::Rights::FD_FILESTAT_SET_SIZE) {
            caps = caps | FileCaps::FILESTAT_SET_SIZE;
        }
        if rights.contains(&types::Rights::FD_FILESTAT_SET_TIMES) {
            caps = caps | FileCaps::FILESTAT_SET_TIMES;
        }
        caps
    }
}

// DirCaps can always be represented as wasi Rights
impl From<&DirCaps> for types::Rights {
    fn from(caps: &DirCaps) -> types::Rights {
        let mut rights = types::Rights::empty();
        if caps.contains(&DirCaps::CREATE_DIRECTORY) {
            rights = rights | types::Rights::PATH_CREATE_DIRECTORY;
        }
        if caps.contains(&DirCaps::CREATE_FILE) {
            rights = rights | types::Rights::PATH_CREATE_FILE;
        }
        if caps.contains(&DirCaps::LINK_SOURCE) {
            rights = rights | types::Rights::PATH_LINK_SOURCE;
        }
        if caps.contains(&DirCaps::LINK_TARGET) {
            rights = rights | types::Rights::PATH_LINK_TARGET;
        }
        if caps.contains(&DirCaps::OPEN) {
            rights = rights | types::Rights::PATH_OPEN;
        }
        if caps.contains(&DirCaps::READDIR) {
            rights = rights | types::Rights::FD_READDIR;
        }
        if caps.contains(&DirCaps::READLINK) {
            rights = rights | types::Rights::PATH_READLINK;
        }
        if caps.contains(&DirCaps::RENAME_SOURCE) {
            rights = rights | types::Rights::PATH_RENAME_SOURCE;
        }
        if caps.contains(&DirCaps::RENAME_TARGET) {
            rights = rights | types::Rights::PATH_RENAME_TARGET;
        }
        if caps.contains(&DirCaps::SYMLINK) {
            rights = rights | types::Rights::PATH_SYMLINK;
        }
        if caps.contains(&DirCaps::REMOVE_DIRECTORY) {
            rights = rights | types::Rights::PATH_REMOVE_DIRECTORY;
        }
        if caps.contains(&DirCaps::UNLINK_FILE) {
            rights = rights | types::Rights::PATH_UNLINK_FILE;
        }
        if caps.contains(&DirCaps::PATH_FILESTAT_GET) {
            rights = rights | types::Rights::PATH_FILESTAT_GET;
        }
        if caps.contains(&DirCaps::PATH_FILESTAT_SET_TIMES) {
            rights = rights | types::Rights::PATH_FILESTAT_SET_TIMES;
        }
        if caps.contains(&DirCaps::FILESTAT_GET) {
            rights = rights | types::Rights::FD_FILESTAT_GET;
        }
        if caps.contains(&DirCaps::FILESTAT_SET_TIMES) {
            rights = rights | types::Rights::FD_FILESTAT_SET_TIMES;
        }
        rights
    }
}

// DirCaps are a subset of wasi Rights - not all Rights have a valid representation as DirCaps
impl From<&types::Rights> for DirCaps {
    fn from(rights: &types::Rights) -> DirCaps {
        let mut caps = DirCaps::empty();
        if rights.contains(&types::Rights::PATH_CREATE_DIRECTORY) {
            caps = caps | DirCaps::CREATE_DIRECTORY;
        }
        if rights.contains(&types::Rights::PATH_CREATE_FILE) {
            caps = caps | DirCaps::CREATE_FILE;
        }
        if rights.contains(&types::Rights::PATH_LINK_SOURCE) {
            caps = caps | DirCaps::LINK_SOURCE;
        }
        if rights.contains(&types::Rights::PATH_LINK_TARGET) {
            caps = caps | DirCaps::LINK_TARGET;
        }
        if rights.contains(&types::Rights::PATH_OPEN) {
            caps = caps | DirCaps::OPEN;
        }
        if rights.contains(&types::Rights::FD_READDIR) {
            caps = caps | DirCaps::READDIR;
        }
        if rights.contains(&types::Rights::PATH_READLINK) {
            caps = caps | DirCaps::READLINK;
        }
        if rights.contains(&types::Rights::PATH_RENAME_SOURCE) {
            caps = caps | DirCaps::RENAME_SOURCE;
        }
        if rights.contains(&types::Rights::PATH_RENAME_TARGET) {
            caps = caps | DirCaps::RENAME_TARGET;
        }
        if rights.contains(&types::Rights::PATH_SYMLINK) {
            caps = caps | DirCaps::SYMLINK;
        }
        if rights.contains(&types::Rights::PATH_REMOVE_DIRECTORY) {
            caps = caps | DirCaps::REMOVE_DIRECTORY;
        }
        if rights.contains(&types::Rights::PATH_UNLINK_FILE) {
            caps = caps | DirCaps::UNLINK_FILE;
        }
        if rights.contains(&types::Rights::PATH_FILESTAT_GET) {
            caps = caps | DirCaps::PATH_FILESTAT_GET;
        }
        if rights.contains(&types::Rights::PATH_FILESTAT_SET_TIMES) {
            caps = caps | DirCaps::PATH_FILESTAT_SET_TIMES;
        }
        if rights.contains(&types::Rights::FD_FILESTAT_GET) {
            caps = caps | DirCaps::FILESTAT_GET;
        }
        if rights.contains(&types::Rights::FD_FILESTAT_SET_TIMES) {
            caps = caps | DirCaps::FILESTAT_SET_TIMES;
        }
        caps
    }
}

impl From<&FileType> for types::Filetype {
    fn from(ft: &FileType) -> types::Filetype {
        match ft {
            FileType::Directory => types::Filetype::Directory,
            FileType::BlockDevice => types::Filetype::BlockDevice,
            FileType::CharacterDevice => types::Filetype::CharacterDevice,
            FileType::RegularFile => types::Filetype::RegularFile,
            FileType::SocketDgram => types::Filetype::SocketDgram,
            FileType::SocketStream => types::Filetype::SocketStream,
            FileType::SymbolicLink => types::Filetype::SymbolicLink,
            FileType::Unknown => types::Filetype::Unknown,
        }
    }
}
impl From<&FdFlags> for types::Fdflags {
    fn from(fdflags: &FdFlags) -> types::Fdflags {
        let mut out = types::Fdflags::empty();
        if fdflags.contains(&FdFlags::APPEND) {
            out = out | types::Fdflags::APPEND;
        }
        if fdflags.contains(&FdFlags::DSYNC) {
            out = out | types::Fdflags::DSYNC;
        }
        if fdflags.contains(&FdFlags::NONBLOCK) {
            out = out | types::Fdflags::NONBLOCK;
        }
        if fdflags.contains(&FdFlags::RSYNC) {
            out = out | types::Fdflags::RSYNC;
        }
        if fdflags.contains(&FdFlags::SYNC) {
            out = out | types::Fdflags::SYNC;
        }
        out
    }
}

impl From<&types::Fdflags> for FdFlags {
    fn from(fdflags: &types::Fdflags) -> FdFlags {
        let mut out = FdFlags::empty();
        if fdflags.contains(&types::Fdflags::APPEND) {
            out = out | FdFlags::APPEND;
        }
        if fdflags.contains(&types::Fdflags::DSYNC) {
            out = out | FdFlags::DSYNC;
        }
        if fdflags.contains(&types::Fdflags::NONBLOCK) {
            out = out | FdFlags::NONBLOCK;
        }
        if fdflags.contains(&types::Fdflags::RSYNC) {
            out = out | FdFlags::RSYNC;
        }
        if fdflags.contains(&types::Fdflags::SYNC) {
            out = out | FdFlags::SYNC;
        }
        out
    }
}

impl From<&types::Oflags> for OFlags {
    fn from(oflags: &types::Oflags) -> OFlags {
        let mut out = OFlags::empty();
        if oflags.contains(&types::Oflags::CREAT) {
            out = out | OFlags::CREATE;
        }
        if oflags.contains(&types::Oflags::DIRECTORY) {
            out = out | OFlags::DIRECTORY;
        }
        if oflags.contains(&types::Oflags::EXCL) {
            out = out | OFlags::EXCLUSIVE;
        }
        if oflags.contains(&types::Oflags::TRUNC) {
            out = out | OFlags::TRUNCATE;
        }
        out
    }
}

impl From<&OFlags> for types::Oflags {
    fn from(oflags: &OFlags) -> types::Oflags {
        let mut out = types::Oflags::empty();
        if oflags.contains(&OFlags::CREATE) {
            out = out | types::Oflags::CREAT;
        }
        if oflags.contains(&OFlags::DIRECTORY) {
            out = out | types::Oflags::DIRECTORY;
        }
        if oflags.contains(&OFlags::EXCLUSIVE) {
            out = out | types::Oflags::EXCL;
        }
        if oflags.contains(&OFlags::TRUNCATE) {
            out = out | types::Oflags::TRUNC;
        }
        out
    }
}
impl From<Filestat> for types::Filestat {
    fn from(stat: Filestat) -> types::Filestat {
        types::Filestat {
            dev: stat.device_id,
            ino: stat.inode,
            filetype: types::Filetype::from(&stat.filetype),
            nlink: stat.nlink,
            size: stat.size,
            atim: stat
                .atim
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64)
                .unwrap_or(0),
            mtim: stat
                .mtim
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64)
                .unwrap_or(0),
            ctim: stat
                .ctim
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64)
                .unwrap_or(0),
        }
    }
}

impl From<&ReaddirEntity> for types::Dirent {
    fn from(e: &ReaddirEntity) -> types::Dirent {
        types::Dirent {
            d_ino: e.inode,
            d_namlen: e.namelen,
            d_type: types::Filetype::from(&e.filetype),
            d_next: e.next.into(),
        }
    }
}

fn dirent_bytes(dirent: types::Dirent) -> Vec<u8> {
    use wiggle::GuestType;
    assert_eq!(
        types::Dirent::guest_size(),
        std::mem::size_of::<types::Dirent>() as _,
        "Dirent guest repr and host repr should match"
    );
    let size = types::Dirent::guest_size()
        .try_into()
        .expect("Dirent is smaller than 2^32");
    let mut bytes = Vec::with_capacity(size);
    bytes.resize(size, 0);
    let ptr = bytes.as_mut_ptr() as *mut types::Dirent;
    unsafe { ptr.write_unaligned(dirent) };
    bytes
}

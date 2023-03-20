#![allow(unused_variables)]

use crate::command::wasi;
use crate::command::wasi::streams::{InputStream, OutputStream};
use crate::WasiCtx;
use anyhow::anyhow;
use std::{
    io::{IoSlice, IoSliceMut},
    ops::Deref,
    sync::Mutex,
    time::{Duration, SystemTime},
};
use wasi_common::{
    dir::{ReaddirCursor, ReaddirIterator, TableDirExt},
    file::{FdFlags, FileStream, TableFileExt},
    WasiDir, WasiFile,
};

impl From<wasi_common::Error> for wasi::filesystem::Error {
    fn from(error: wasi_common::Error) -> wasi::filesystem::Error {
        use wasi::filesystem::ErrorCode;
        use wasi_common::Errno::*;
        if let Some(errno) = error.downcast_ref() {
            match errno {
                Acces => ErrorCode::Access.into(),
                Again => ErrorCode::WouldBlock.into(),
                Already => ErrorCode::Already.into(),
                Badf => ErrorCode::BadDescriptor.into(),
                Busy => ErrorCode::Busy.into(),
                Deadlk => ErrorCode::Deadlock.into(),
                Dquot => ErrorCode::Quota.into(),
                Exist => ErrorCode::Exist.into(),
                Fbig => ErrorCode::FileTooLarge.into(),
                Ilseq => ErrorCode::IllegalByteSequence.into(),
                Inprogress => ErrorCode::InProgress.into(),
                Intr => ErrorCode::Interrupted.into(),
                Inval => ErrorCode::Invalid.into(),
                Io => ErrorCode::Io.into(),
                Isdir => ErrorCode::IsDirectory.into(),
                Loop => ErrorCode::Loop.into(),
                Mlink => ErrorCode::TooManyLinks.into(),
                Msgsize => ErrorCode::MessageSize.into(),
                Nametoolong => ErrorCode::NameTooLong.into(),
                Nodev => ErrorCode::NoDevice.into(),
                Noent => ErrorCode::NoEntry.into(),
                Nolck => ErrorCode::NoLock.into(),
                Nomem => ErrorCode::InsufficientMemory.into(),
                Nospc => ErrorCode::InsufficientSpace.into(),
                Nosys => ErrorCode::Unsupported.into(),
                Notdir => ErrorCode::NotDirectory.into(),
                Notempty => ErrorCode::NotEmpty.into(),
                Notrecoverable => ErrorCode::NotRecoverable.into(),
                Notsup => ErrorCode::Unsupported.into(),
                Notty => ErrorCode::NoTty.into(),
                Nxio => ErrorCode::NoSuchDevice.into(),
                Overflow => ErrorCode::Overflow.into(),
                Perm => ErrorCode::NotPermitted.into(),
                Pipe => ErrorCode::Pipe.into(),
                Rofs => ErrorCode::ReadOnly.into(),
                Spipe => ErrorCode::InvalidSeek.into(),
                Txtbsy => ErrorCode::TextFileBusy.into(),
                Xdev => ErrorCode::CrossDevice.into(),
                Success | Notsock | Proto | Protonosupport | Prototype | TooBig | Notconn => {
                    wasi::filesystem::Error::trap(anyhow!(error))
                }
                Addrinuse | Addrnotavail | Afnosupport | Badmsg | Canceled | Connaborted
                | Connrefused | Connreset | Destaddrreq | Fault | Hostunreach | Idrm | Isconn
                | Mfile | Multihop | Netdown | Netreset | Netunreach | Nfile | Nobufs | Noexec
                | Nolink | Nomsg | Noprotoopt | Ownerdead | Range | Srch | Stale | Timedout => {
                    wasi::filesystem::Error::trap(anyhow!("Unexpected errno: {:?}", errno))
                }
            }
        } else {
            wasi::filesystem::Error::trap(anyhow!(error))
        }
    }
}

impl From<wasi::filesystem::OpenFlags> for wasi_common::file::OFlags {
    fn from(oflags: wasi::filesystem::OpenFlags) -> Self {
        let mut flags = wasi_common::file::OFlags::empty();
        if oflags.contains(wasi::filesystem::OpenFlags::CREATE) {
            flags |= wasi_common::file::OFlags::CREATE;
        }
        if oflags.contains(wasi::filesystem::OpenFlags::DIRECTORY) {
            flags |= wasi_common::file::OFlags::DIRECTORY;
        }
        if oflags.contains(wasi::filesystem::OpenFlags::EXCLUSIVE) {
            flags |= wasi_common::file::OFlags::EXCLUSIVE;
        }
        if oflags.contains(wasi::filesystem::OpenFlags::TRUNCATE) {
            flags |= wasi_common::file::OFlags::TRUNCATE;
        }
        flags
    }
}

impl From<FdFlags> for wasi::filesystem::DescriptorFlags {
    fn from(fdflags: FdFlags) -> Self {
        let mut flags = wasi::filesystem::DescriptorFlags::empty();
        if fdflags.contains(FdFlags::DSYNC) {
            flags |= wasi::filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC;
        }
        if fdflags.contains(FdFlags::NONBLOCK) {
            flags |= wasi::filesystem::DescriptorFlags::NON_BLOCKING;
        }
        if fdflags.contains(FdFlags::RSYNC) {
            flags |= wasi::filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC;
        }
        if fdflags.contains(FdFlags::SYNC) {
            flags |= wasi::filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC;
        }
        flags
    }
}

impl From<wasi::filesystem::DescriptorFlags> for FdFlags {
    fn from(flags: wasi::filesystem::DescriptorFlags) -> FdFlags {
        let mut fdflags = FdFlags::empty();
        if flags.contains(wasi::filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC) {
            fdflags |= FdFlags::DSYNC;
        }
        if flags.contains(wasi::filesystem::DescriptorFlags::NON_BLOCKING) {
            fdflags |= FdFlags::NONBLOCK;
        }
        if flags.contains(wasi::filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC) {
            fdflags |= FdFlags::RSYNC;
        }
        if flags.contains(wasi::filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC) {
            fdflags |= FdFlags::SYNC;
        }
        fdflags
    }
}

impl From<wasi_common::file::FileType> for wasi::filesystem::DescriptorType {
    fn from(type_: wasi_common::file::FileType) -> Self {
        match type_ {
            wasi_common::file::FileType::Unknown => Self::Unknown,
            wasi_common::file::FileType::BlockDevice => Self::BlockDevice,
            wasi_common::file::FileType::CharacterDevice => Self::CharacterDevice,
            wasi_common::file::FileType::Directory => Self::Directory,
            wasi_common::file::FileType::RegularFile => Self::RegularFile,
            wasi_common::file::FileType::SocketDgram
            | wasi_common::file::FileType::SocketStream => Self::Socket,
            wasi_common::file::FileType::SymbolicLink => Self::SymbolicLink,
            wasi_common::file::FileType::Pipe => Self::Fifo,
        }
    }
}

impl From<wasi_common::file::Filestat> for wasi::filesystem::DescriptorStat {
    fn from(stat: wasi_common::file::Filestat) -> Self {
        fn timestamp(time: Option<std::time::SystemTime>) -> wasi::filesystem::Datetime {
            time.map(|t| {
                let since = t.duration_since(SystemTime::UNIX_EPOCH).unwrap();
                wasi::filesystem::Datetime {
                    seconds: since.as_secs(),
                    nanoseconds: since.subsec_nanos(),
                }
            })
            .unwrap_or(wasi::filesystem::Datetime {
                seconds: 0,
                nanoseconds: 0,
            })
        }

        Self {
            device: stat.device_id,
            inode: stat.inode,
            type_: stat.filetype.into(),
            link_count: stat.nlink,
            size: stat.size,
            data_access_timestamp: timestamp(stat.atim),
            data_modification_timestamp: timestamp(stat.mtim),
            status_change_timestamp: timestamp(stat.ctim),
        }
    }
}

impl From<wasi::filesystem::Advice> for wasi_common::file::Advice {
    fn from(advice: wasi::filesystem::Advice) -> Self {
        match advice {
            wasi::filesystem::Advice::Normal => wasi_common::file::Advice::Normal,
            wasi::filesystem::Advice::Sequential => wasi_common::file::Advice::Sequential,
            wasi::filesystem::Advice::Random => wasi_common::file::Advice::Random,
            wasi::filesystem::Advice::WillNeed => wasi_common::file::Advice::WillNeed,
            wasi::filesystem::Advice::DontNeed => wasi_common::file::Advice::DontNeed,
            wasi::filesystem::Advice::NoReuse => wasi_common::file::Advice::NoReuse,
        }
    }
}

fn system_time_spec_from_timestamp(
    t: wasi::filesystem::NewTimestamp,
) -> Option<wasi_common::SystemTimeSpec> {
    match t {
        wasi::filesystem::NewTimestamp::NoChange => None,
        wasi::filesystem::NewTimestamp::Now => Some(wasi_common::SystemTimeSpec::SymbolicNow),
        wasi::filesystem::NewTimestamp::Timestamp(datetime) => Some(
            wasi_common::SystemTimeSpec::Absolute(cap_std::time::SystemTime::from_std(
                SystemTime::UNIX_EPOCH + Duration::new(datetime.seconds, datetime.nanoseconds),
            )),
        ),
    }
}

#[async_trait::async_trait]
impl wasi::filesystem::Host for WasiCtx {
    async fn advise(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        offset: wasi::filesystem::Filesize,
        len: wasi::filesystem::Filesize,
        advice: wasi::filesystem::Advice,
    ) -> Result<(), wasi::filesystem::Error> {
        let f = self.table_mut().get_file_mut(fd)?;
        f.advise(offset, len, advice.into()).await?;
        Ok(())
    }

    async fn sync_data(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        if table.is::<Box<dyn WasiFile>>(fd) {
            Ok(table.get_file(fd)?.datasync().await?)
        } else if table.is::<Box<dyn WasiDir>>(fd) {
            Ok(table.get_dir(fd)?.datasync().await?)
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn get_flags(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<wasi::filesystem::DescriptorFlags, wasi::filesystem::Error> {
        let table = self.table();
        if table.is::<Box<dyn WasiFile>>(fd) {
            Ok(table.get_file(fd)?.get_fdflags().await?.into())
        } else if table.is::<Box<dyn WasiDir>>(fd) {
            Ok(table.get_dir(fd)?.get_fdflags().await?.into())
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn get_type(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<wasi::filesystem::DescriptorType, wasi::filesystem::Error> {
        let table = self.table();
        if table.is::<Box<dyn WasiFile>>(fd) {
            Ok(table.get_file(fd)?.get_filetype().await?.into())
        } else if table.is::<Box<dyn WasiDir>>(fd) {
            Ok(wasi::filesystem::DescriptorType::Directory)
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn set_flags(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        flags: wasi::filesystem::DescriptorFlags,
    ) -> Result<(), wasi::filesystem::Error> {
        // FIXME
        Err(wasi::filesystem::ErrorCode::Unsupported.into())
    }

    async fn set_size(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        size: wasi::filesystem::Filesize,
    ) -> Result<(), wasi::filesystem::Error> {
        let f = self.table_mut().get_file_mut(fd)?;
        f.set_filestat_size(size).await?;
        Ok(())
    }

    async fn set_times(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        atim: wasi::filesystem::NewTimestamp,
        mtim: wasi::filesystem::NewTimestamp,
    ) -> Result<(), wasi::filesystem::Error> {
        let atim = system_time_spec_from_timestamp(atim);
        let mtim = system_time_spec_from_timestamp(mtim);

        let table = self.table_mut();
        if table.is::<Box<dyn WasiFile>>(fd) {
            Ok(table
                .get_file_mut(fd)
                .expect("checked entry is a file")
                .set_times(atim, mtim)
                .await?)
        } else if table.is::<Box<dyn WasiDir>>(fd) {
            Ok(table
                .get_dir(fd)
                .expect("checked entry is a dir")
                .set_times(".", atim, mtim, false)
                .await?)
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn read(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        len: wasi::filesystem::Filesize,
        offset: wasi::filesystem::Filesize,
    ) -> Result<(Vec<u8>, bool), wasi::filesystem::Error> {
        let f = self.table_mut().get_file_mut(fd)?;

        let mut buffer = vec![0; len.try_into().unwrap_or(usize::MAX)];

        let (bytes_read, end) = f
            .read_vectored_at(&mut [IoSliceMut::new(&mut buffer)], offset)
            .await?;

        buffer.truncate(
            bytes_read
                .try_into()
                .expect("bytes read into memory as u64 fits in usize"),
        );

        Ok((buffer, end))
    }

    async fn write(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        buf: Vec<u8>,
        offset: wasi::filesystem::Filesize,
    ) -> Result<wasi::filesystem::Filesize, wasi::filesystem::Error> {
        let f = self.table_mut().get_file_mut(fd)?;

        let bytes_written = f.write_vectored_at(&[IoSlice::new(&buf)], offset).await?;

        Ok(wasi::filesystem::Filesize::try_from(bytes_written).expect("usize fits in Filesize"))
    }

    async fn read_directory(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<wasi::filesystem::DirectoryEntryStream, wasi::filesystem::Error> {
        let iterator = self
            .table()
            .get_dir(fd)?
            .readdir(ReaddirCursor::from(0))
            .await?;

        Ok(self.table_mut().push(Box::new(Mutex::new(iterator)))?)
    }

    async fn read_directory_entry(
        &mut self,
        stream: wasi::filesystem::DirectoryEntryStream,
    ) -> Result<Option<wasi::filesystem::DirectoryEntry>, wasi::filesystem::Error> {
        let entity = self
            .table()
            .get::<Mutex<ReaddirIterator>>(stream)?
            .lock()
            .expect("readdir iterator is lockable")
            .next()
            .transpose()?;

        Ok(entity.map(|e| wasi::filesystem::DirectoryEntry {
            inode: Some(e.inode),
            type_: e.filetype.into(),
            name: e.name,
        }))
    }

    async fn drop_directory_entry_stream(
        &mut self,
        stream: wasi::filesystem::DirectoryEntryStream,
    ) -> anyhow::Result<()> {
        // Trap if deletion is not possible:
        self.table_mut().delete::<Mutex<ReaddirIterator>>(stream)?;

        Ok(())
    }

    async fn sync(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        if table.is::<Box<dyn WasiFile>>(fd) {
            Ok(table.get_file(fd)?.sync().await?)
        } else if table.is::<Box<dyn WasiDir>>(fd) {
            Ok(table.get_dir(fd)?.sync().await?)
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn create_directory_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        Ok(table.get_dir(fd)?.create_dir(&path).await?)
    }

    async fn stat(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<wasi::filesystem::DescriptorStat, wasi::filesystem::Error> {
        let table = self.table();
        if table.is::<Box<dyn WasiFile>>(fd) {
            Ok(table.get_file(fd)?.get_filestat().await?.into())
        } else if table.is::<Box<dyn WasiDir>>(fd) {
            Ok(table.get_dir(fd)?.get_filestat().await?.into())
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn stat_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        at_flags: wasi::filesystem::PathFlags,
        path: String,
    ) -> Result<wasi::filesystem::DescriptorStat, wasi::filesystem::Error> {
        let table = self.table();
        Ok(table
            .get_dir(fd)?
            .get_path_filestat(
                &path,
                at_flags.contains(wasi::filesystem::PathFlags::SYMLINK_FOLLOW),
            )
            .await?
            .into())
    }

    async fn set_times_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        at_flags: wasi::filesystem::PathFlags,
        path: String,
        atim: wasi::filesystem::NewTimestamp,
        mtim: wasi::filesystem::NewTimestamp,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        Ok(table
            .get_dir(fd)?
            .set_times(
                &path,
                system_time_spec_from_timestamp(atim),
                system_time_spec_from_timestamp(mtim),
                at_flags.contains(wasi::filesystem::PathFlags::SYMLINK_FOLLOW),
            )
            .await?)
    }

    async fn link_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        // TODO delete the at flags from this function
        old_at_flags: wasi::filesystem::PathFlags,
        old_path: String,
        new_descriptor: wasi::filesystem::Descriptor,
        new_path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        let old_dir = table.get_dir(fd)?;
        let new_dir = table.get_dir(new_descriptor)?;
        if old_at_flags.contains(wasi::filesystem::PathFlags::SYMLINK_FOLLOW) {
            return Err(wasi::filesystem::ErrorCode::Invalid.into());
        }
        old_dir
            .hard_link(&old_path, new_dir.deref(), &new_path)
            .await?;
        Ok(())
    }

    async fn open_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        at_flags: wasi::filesystem::PathFlags,
        old_path: String,
        oflags: wasi::filesystem::OpenFlags,
        flags: wasi::filesystem::DescriptorFlags,
        // TODO: How should this be used?
        _mode: wasi::filesystem::Modes,
    ) -> Result<wasi::filesystem::Descriptor, wasi::filesystem::Error> {
        let table = self.table_mut();
        let dir = table.get_dir(fd)?;

        let symlink_follow = at_flags.contains(wasi::filesystem::PathFlags::SYMLINK_FOLLOW);

        if oflags.contains(wasi::filesystem::OpenFlags::DIRECTORY) {
            if oflags.contains(wasi::filesystem::OpenFlags::CREATE)
                || oflags.contains(wasi::filesystem::OpenFlags::EXCLUSIVE)
                || oflags.contains(wasi::filesystem::OpenFlags::TRUNCATE)
            {
                return Err(wasi::filesystem::ErrorCode::Invalid.into());
            }
            let child_dir = dir.open_dir(symlink_follow, &old_path).await?;
            drop(dir);
            Ok(table.push(Box::new(child_dir))?)
        } else {
            let file = dir
                .open_file(
                    symlink_follow,
                    &old_path,
                    oflags.into(),
                    flags.contains(wasi::filesystem::DescriptorFlags::READ),
                    flags.contains(wasi::filesystem::DescriptorFlags::WRITE),
                    flags.into(),
                )
                .await?;
            drop(dir);
            Ok(table.push(Box::new(file))?)
        }
    }

    async fn drop_descriptor(&mut self, fd: wasi::filesystem::Descriptor) -> anyhow::Result<()> {
        let table = self.table_mut();
        if !(table.delete::<Box<dyn WasiFile>>(fd).is_ok()
            || table.delete::<Box<dyn WasiDir>>(fd).is_ok())
        {
            // this will trap:
            anyhow::bail!("{fd} is neither a file nor a directory");
        }
        Ok(())
    }

    async fn readlink_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> Result<String, wasi::filesystem::Error> {
        let table = self.table();
        let dir = table.get_dir(fd)?;
        let link = dir.read_link(&path).await?;
        Ok(link
            .into_os_string()
            .into_string()
            .map_err(|_| wasi::filesystem::ErrorCode::IllegalByteSequence)?)
    }

    async fn remove_directory_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        Ok(table.get_dir(fd)?.remove_dir(&path).await?)
    }

    async fn rename_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        old_path: String,
        new_fd: wasi::filesystem::Descriptor,
        new_path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        let old_dir = table.get_dir(fd)?;
        let new_dir = table.get_dir(new_fd)?;
        old_dir
            .rename(&old_path, new_dir.deref(), &new_path)
            .await?;
        Ok(())
    }

    async fn symlink_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        old_path: String,
        new_path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        Ok(table.get_dir(fd)?.symlink(&old_path, &new_path).await?)
    }

    async fn unlink_file_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        Ok(table.get_dir(fd)?.unlink_file(&path).await?)
    }

    async fn change_file_permissions_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        at_flags: wasi::filesystem::PathFlags,
        path: String,
        mode: wasi::filesystem::Modes,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn change_directory_permissions_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        at_flags: wasi::filesystem::PathFlags,
        path: String,
        mode: wasi::filesystem::Modes,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn lock_shared(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn lock_exclusive(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn try_lock_shared(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn try_lock_exclusive(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn unlock(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn read_via_stream(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        offset: wasi::filesystem::Filesize,
    ) -> anyhow::Result<InputStream> {
        // Trap if fd lookup fails:
        let f = self.table_mut().get_file_mut(fd)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = f.dup();

        // Create a stream view for it.
        let reader = FileStream::new_reader(clone, offset);

        // Box it up.
        let boxed: Box<dyn wasi_common::InputStream> = Box::new(reader);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self.table_mut().push(Box::new(boxed))?;

        Ok(index)
    }

    async fn write_via_stream(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        offset: wasi::filesystem::Filesize,
    ) -> anyhow::Result<OutputStream> {
        // Trap if fd lookup fails:
        let f = self.table_mut().get_file_mut(fd)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = f.dup();

        // Create a stream view for it.
        let writer = FileStream::new_writer(clone, offset);

        // Box it up.
        let boxed: Box<dyn wasi_common::OutputStream> = Box::new(writer);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self.table_mut().push(Box::new(boxed))?;

        Ok(index)
    }

    async fn append_via_stream(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> anyhow::Result<OutputStream> {
        // Trap if fd lookup fails:
        let f = self.table_mut().get_file_mut(fd)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = f.dup();

        // Create a stream view for it.
        let appender = FileStream::new_appender(clone);

        // Box it up.
        let boxed: Box<dyn wasi_common::OutputStream> = Box::new(appender);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self.table_mut().push(Box::new(boxed))?;

        Ok(index)
    }
}

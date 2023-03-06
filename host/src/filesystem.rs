#![allow(unused_variables)]

use crate::wasi;
use crate::wasi::streams::{InputStream, OutputStream};
use crate::{HostResult, WasiCtx};
use std::{
    io::{IoSlice, IoSliceMut},
    ops::{BitAnd, Deref},
    sync::Mutex,
    time::{Duration, SystemTime},
};
use wasi_common::{
    dir::{ReaddirCursor, ReaddirIterator, TableDirExt},
    file::{FdFlags, FileStream, TableFileExt},
    WasiDir, WasiFile,
};

/// TODO: Remove once wasmtime #5589 lands.
fn contains<T: BitAnd<Output = T> + Eq + Copy>(flags: T, flag: T) -> bool {
    (flags & flag) == flag
}

fn convert(error: wasi_common::Error) -> anyhow::Error {
    if let Some(errno) = error.downcast_ref() {
        use wasi::filesystem::ErrorCode;
        use wasi_common::Errno::*;

        match errno {
            Acces => ErrorCode::Access,
            Again => ErrorCode::WouldBlock,
            Already => ErrorCode::Already,
            Badf => ErrorCode::BadDescriptor,
            Busy => ErrorCode::Busy,
            Deadlk => ErrorCode::Deadlock,
            Dquot => ErrorCode::Quota,
            Exist => ErrorCode::Exist,
            Fbig => ErrorCode::FileTooLarge,
            Ilseq => ErrorCode::IllegalByteSequence,
            Inprogress => ErrorCode::InProgress,
            Intr => ErrorCode::Interrupted,
            Inval => ErrorCode::Invalid,
            Io => ErrorCode::Io,
            Isdir => ErrorCode::IsDirectory,
            Loop => ErrorCode::Loop,
            Mlink => ErrorCode::TooManyLinks,
            Msgsize => ErrorCode::MessageSize,
            Nametoolong => ErrorCode::NameTooLong,
            Nodev => ErrorCode::NoDevice,
            Noent => ErrorCode::NoEntry,
            Nolck => ErrorCode::NoLock,
            Nomem => ErrorCode::InsufficientMemory,
            Nospc => ErrorCode::InsufficientSpace,
            Nosys => ErrorCode::Unsupported,
            Notdir => ErrorCode::NotDirectory,
            Notempty => ErrorCode::NotEmpty,
            Notrecoverable => ErrorCode::NotRecoverable,
            Notsup => ErrorCode::Unsupported,
            Notty => ErrorCode::NoTty,
            Nxio => ErrorCode::NoSuchDevice,
            Overflow => ErrorCode::Overflow,
            Perm => ErrorCode::NotPermitted,
            Pipe => ErrorCode::Pipe,
            Rofs => ErrorCode::ReadOnly,
            Spipe => ErrorCode::InvalidSeek,
            Txtbsy => ErrorCode::TextFileBusy,
            Xdev => ErrorCode::CrossDevice,
            Success | Notsock | Proto | Protonosupport | Prototype | TooBig | Notconn => {
                return error.into();
            }
            Addrinuse | Addrnotavail | Afnosupport | Badmsg | Canceled | Connaborted
            | Connrefused | Connreset | Destaddrreq | Fault | Hostunreach | Idrm | Isconn
            | Mfile | Multihop | Netdown | Netreset | Netunreach | Nfile | Nobufs | Noexec
            | Nolink | Nomsg | Noprotoopt | Ownerdead | Range | Srch | Stale | Timedout => {
                panic!("Unexpected errno: {:?}", errno);
            }
        }
        .into()
    } else {
        error.into()
    }
}

impl From<wasi::filesystem::OpenFlags> for wasi_common::file::OFlags {
    fn from(oflags: wasi::filesystem::OpenFlags) -> Self {
        let mut flags = wasi_common::file::OFlags::empty();
        if contains(oflags, wasi::filesystem::OpenFlags::CREATE) {
            flags |= wasi_common::file::OFlags::CREATE;
        }
        if contains(oflags, wasi::filesystem::OpenFlags::DIRECTORY) {
            flags |= wasi_common::file::OFlags::DIRECTORY;
        }
        if contains(oflags, wasi::filesystem::OpenFlags::EXCLUSIVE) {
            flags |= wasi_common::file::OFlags::EXCLUSIVE;
        }
        if contains(oflags, wasi::filesystem::OpenFlags::TRUNCATE) {
            flags |= wasi_common::file::OFlags::TRUNCATE;
        }
        flags
    }
}

impl From<FdFlags> for wasi::filesystem::DescriptorFlags {
    fn from(fdflags: FdFlags) -> Self {
        let mut flags = wasi::filesystem::DescriptorFlags::empty();
        if contains(fdflags, FdFlags::DSYNC) {
            flags |= wasi::filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC;
        }
        if contains(fdflags, FdFlags::NONBLOCK) {
            flags |= wasi::filesystem::DescriptorFlags::NON_BLOCKING;
        }
        if contains(fdflags, FdFlags::RSYNC) {
            flags |= wasi::filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC;
        }
        if contains(fdflags, FdFlags::SYNC) {
            flags |= wasi::filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC;
        }
        flags
    }
}

impl From<wasi::filesystem::DescriptorFlags> for FdFlags {
    fn from(flags: wasi::filesystem::DescriptorFlags) -> FdFlags {
        let mut fdflags = FdFlags::empty();
        if contains(
            flags,
            wasi::filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC,
        ) {
            fdflags |= FdFlags::DSYNC;
        }
        if contains(flags, wasi::filesystem::DescriptorFlags::NON_BLOCKING) {
            fdflags |= FdFlags::NONBLOCK;
        }
        if contains(
            flags,
            wasi::filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC,
        ) {
            fdflags |= FdFlags::RSYNC;
        }
        if contains(
            flags,
            wasi::filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC,
        ) {
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
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;
        f.advise(offset, len, advice.into())
            .await
            .map_err(convert)?;
        Ok(Ok(()))
    }

    async fn sync_data(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        let table = self.table();
        if table.is::<Box<dyn WasiFile>>(fd) {
            Ok(Ok(table
                .get_file(fd)
                .map_err(convert)?
                .datasync()
                .await
                .map_err(convert)?))
        } else if table.is::<Box<dyn WasiDir>>(fd) {
            Ok(Ok(table
                .get_dir(fd)
                .map_err(convert)?
                .datasync()
                .await
                .map_err(convert)?))
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn get_flags(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> HostResult<wasi::filesystem::DescriptorFlags, wasi::filesystem::ErrorCode> {
        let table = self.table();
        if table.is::<Box<dyn WasiFile>>(fd) {
            Ok(Ok(table
                .get_file(fd)
                .map_err(convert)?
                .get_fdflags()
                .await
                .map_err(convert)?
                .into()))
        } else if table.is::<Box<dyn WasiDir>>(fd) {
            Ok(Ok(table
                .get_dir(fd)
                .map_err(convert)?
                .get_fdflags()
                .await
                .map_err(convert)?
                .into()))
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn get_type(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> HostResult<wasi::filesystem::DescriptorType, wasi::filesystem::ErrorCode> {
        let table = self.table();
        if table.is::<Box<dyn WasiFile>>(fd) {
            Ok(Ok(table
                .get_file(fd)
                .map_err(convert)?
                .get_filetype()
                .await
                .map_err(convert)?
                .into()))
        } else if table.is::<Box<dyn WasiDir>>(fd) {
            Ok(Ok(wasi::filesystem::DescriptorType::Directory))
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn set_flags(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        flags: wasi::filesystem::DescriptorFlags,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        // FIXME
        Err(wasi::filesystem::ErrorCode::Unsupported.into())
    }

    async fn set_size(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        size: wasi::filesystem::Filesize,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;
        f.set_filestat_size(size).await.map_err(convert)?;
        Ok(Ok(()))
    }

    async fn set_times(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        atim: wasi::filesystem::NewTimestamp,
        mtim: wasi::filesystem::NewTimestamp,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        let atim = system_time_spec_from_timestamp(atim);
        let mtim = system_time_spec_from_timestamp(mtim);

        let table = self.table_mut();
        if table.is::<Box<dyn WasiFile>>(fd) {
            Ok(Ok(table
                .get_file_mut(fd)
                .expect("checked entry is a file")
                .set_times(atim, mtim)
                .await
                .map_err(convert)?))
        } else if table.is::<Box<dyn WasiDir>>(fd) {
            Ok(Ok(table
                .get_dir(fd)
                .expect("checked entry is a dir")
                .set_times(".", atim, mtim, false)
                .await
                .map_err(convert)?))
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn read(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        len: wasi::filesystem::Filesize,
        offset: wasi::filesystem::Filesize,
    ) -> HostResult<(Vec<u8>, bool), wasi::filesystem::ErrorCode> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;

        let mut buffer = vec![0; len.try_into().unwrap_or(usize::MAX)];

        let (bytes_read, end) = f
            .read_vectored_at(&mut [IoSliceMut::new(&mut buffer)], offset)
            .await
            .map_err(convert)?;

        buffer.truncate(bytes_read.try_into().unwrap());

        Ok(Ok((buffer, end)))
    }

    async fn write(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        buf: Vec<u8>,
        offset: wasi::filesystem::Filesize,
    ) -> HostResult<wasi::filesystem::Filesize, wasi::filesystem::ErrorCode> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;

        let bytes_written = f
            .write_vectored_at(&[IoSlice::new(&buf)], offset)
            .await
            .map_err(convert)?;

        Ok(Ok(
            wasi::filesystem::Filesize::try_from(bytes_written).unwrap()
        ))
    }

    async fn read_directory(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> HostResult<wasi::filesystem::DirectoryEntryStream, wasi::filesystem::ErrorCode> {
        let iterator = self
            .table()
            .get_dir(fd)
            .map_err(convert)?
            .readdir(ReaddirCursor::from(0))
            .await
            .map_err(convert)?;

        self.table_mut()
            .push(Box::new(Mutex::new(iterator)))
            .map(Ok)
            .map_err(convert)
    }

    async fn read_directory_entry(
        &mut self,
        stream: wasi::filesystem::DirectoryEntryStream,
    ) -> HostResult<Option<wasi::filesystem::DirectoryEntry>, wasi::filesystem::ErrorCode> {
        let entity = self
            .table()
            .get::<Mutex<ReaddirIterator>>(stream)
            .map_err(convert)?
            .lock()
            .unwrap()
            .next()
            .transpose()
            .map_err(convert)?;

        Ok(Ok(entity.map(|e| wasi::filesystem::DirectoryEntry {
            inode: Some(e.inode),
            type_: e.filetype.into(),
            name: e.name,
        })))
    }

    async fn drop_directory_entry_stream(
        &mut self,
        stream: wasi::filesystem::DirectoryEntryStream,
    ) -> anyhow::Result<()> {
        self.table_mut()
            .delete::<Mutex<ReaddirIterator>>(stream)
            .map_err(convert)?;

        Ok(())
    }

    async fn sync(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        let table = self.table();
        if table.is::<Box<dyn WasiFile>>(fd) {
            Ok(Ok(table
                .get_file(fd)
                .map_err(convert)?
                .sync()
                .await
                .map_err(convert)?))
        } else if table.is::<Box<dyn WasiDir>>(fd) {
            Ok(Ok(table
                .get_dir(fd)
                .map_err(convert)?
                .sync()
                .await
                .map_err(convert)?))
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn create_directory_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        let table = self.table();
        Ok(Ok(table
            .get_dir(fd)
            .map_err(convert)?
            .create_dir(&path)
            .await
            .map_err(convert)?))
    }

    async fn stat(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> HostResult<wasi::filesystem::DescriptorStat, wasi::filesystem::ErrorCode> {
        let table = self.table();
        if table.is::<Box<dyn WasiFile>>(fd) {
            Ok(Ok(table
                .get_file(fd)
                .map_err(convert)?
                .get_filestat()
                .await
                .map_err(convert)?
                .into()))
        } else if table.is::<Box<dyn WasiDir>>(fd) {
            Ok(Ok(table
                .get_dir(fd)
                .map_err(convert)?
                .get_filestat()
                .await
                .map_err(convert)?
                .into()))
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn stat_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        at_flags: wasi::filesystem::PathFlags,
        path: String,
    ) -> HostResult<wasi::filesystem::DescriptorStat, wasi::filesystem::ErrorCode> {
        let table = self.table();
        Ok(Ok(table
            .get_dir(fd)
            .map_err(convert)?
            .get_path_filestat(
                &path,
                contains(at_flags, wasi::filesystem::PathFlags::SYMLINK_FOLLOW),
            )
            .await
            .map(wasi::filesystem::DescriptorStat::from)
            .map_err(convert)?))
    }

    async fn set_times_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        at_flags: wasi::filesystem::PathFlags,
        path: String,
        atim: wasi::filesystem::NewTimestamp,
        mtim: wasi::filesystem::NewTimestamp,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        let table = self.table();
        Ok(Ok(table
            .get_dir(fd)
            .map_err(convert)?
            .set_times(
                &path,
                system_time_spec_from_timestamp(atim),
                system_time_spec_from_timestamp(mtim),
                contains(at_flags, wasi::filesystem::PathFlags::SYMLINK_FOLLOW),
            )
            .await
            .map_err(convert)?))
    }

    async fn link_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        // TODO delete the at flags from this function
        old_at_flags: wasi::filesystem::PathFlags,
        old_path: String,
        new_descriptor: wasi::filesystem::Descriptor,
        new_path: String,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        let table = self.table();
        let old_dir = table.get_dir(fd).map_err(convert)?;
        let new_dir = table.get_dir(new_descriptor).map_err(convert)?;
        if contains(old_at_flags, wasi::filesystem::PathFlags::SYMLINK_FOLLOW) {
            return Ok(Err(wasi::filesystem::ErrorCode::Invalid));
        }
        old_dir
            .hard_link(&old_path, new_dir.deref(), &new_path)
            .await
            .map_err(convert)?;
        Ok(Ok(()))
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
    ) -> HostResult<wasi::filesystem::Descriptor, wasi::filesystem::ErrorCode> {
        let table = self.table_mut();
        let dir = table.get_dir(fd).map_err(convert)?;

        let symlink_follow = contains(at_flags, wasi::filesystem::PathFlags::SYMLINK_FOLLOW);

        if contains(oflags, wasi::filesystem::OpenFlags::DIRECTORY) {
            if contains(oflags, wasi::filesystem::OpenFlags::CREATE)
                || contains(oflags, wasi::filesystem::OpenFlags::EXCLUSIVE)
                || contains(oflags, wasi::filesystem::OpenFlags::TRUNCATE)
            {
                return Err(wasi::filesystem::ErrorCode::Invalid.into());
            }
            let child_dir = dir
                .open_dir(symlink_follow, &old_path)
                .await
                .map_err(convert)?;
            drop(dir);
            Ok(Ok(table.push(Box::new(child_dir)).map_err(convert)?))
        } else {
            let file = dir
                .open_file(
                    symlink_follow,
                    &old_path,
                    oflags.into(),
                    contains(flags, wasi::filesystem::DescriptorFlags::READ),
                    contains(flags, wasi::filesystem::DescriptorFlags::WRITE),
                    flags.into(),
                )
                .await
                .map_err(convert)?;
            drop(dir);
            Ok(Ok(table.push(Box::new(file)).map_err(convert)?))
        }
    }

    async fn drop_descriptor(&mut self, fd: wasi::filesystem::Descriptor) -> anyhow::Result<()> {
        let table = self.table_mut();
        if !(table.delete::<Box<dyn WasiFile>>(fd).is_ok()
            || table.delete::<Box<dyn WasiDir>>(fd).is_ok())
        {
            anyhow::bail!("{fd} is neither a file nor a directory");
        }
        Ok(())
    }

    async fn readlink_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> HostResult<String, wasi::filesystem::ErrorCode> {
        let table = self.table();
        let dir = table.get_dir(fd).map_err(convert)?;
        let link = dir.read_link(&path).await.map_err(convert)?;
        Ok(link
            .into_os_string()
            .into_string()
            .map_err(|_| wasi::filesystem::ErrorCode::IllegalByteSequence))
    }

    async fn remove_directory_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        let table = self.table();
        Ok(Ok(table
            .get_dir(fd)
            .map_err(convert)?
            .remove_dir(&path)
            .await
            .map_err(convert)?))
    }

    async fn rename_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        old_path: String,
        new_fd: wasi::filesystem::Descriptor,
        new_path: String,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        let table = self.table();
        let old_dir = table.get_dir(fd).map_err(convert)?;
        let new_dir = table.get_dir(new_fd).map_err(convert)?;
        old_dir
            .rename(&old_path, new_dir.deref(), &new_path)
            .await
            .map_err(convert)?;
        Ok(Ok(()))
    }

    async fn symlink_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        old_path: String,
        new_path: String,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        let table = self.table();
        Ok(Ok(table
            .get_dir(fd)
            .map_err(convert)?
            .symlink(&old_path, &new_path)
            .await
            .map_err(convert)?))
    }

    async fn unlink_file_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        let table = self.table();
        Ok(Ok(table
            .get_dir(fd)
            .map_err(convert)?
            .unlink_file(&path)
            .await
            .map_err(convert)?))
    }

    async fn change_file_permissions_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        at_flags: wasi::filesystem::PathFlags,
        path: String,
        mode: wasi::filesystem::Modes,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        todo!()
    }

    async fn change_directory_permissions_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        at_flags: wasi::filesystem::PathFlags,
        path: String,
        mode: wasi::filesystem::Modes,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        todo!()
    }

    async fn lock_shared(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        todo!()
    }

    async fn lock_exclusive(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        todo!()
    }

    async fn try_lock_shared(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        todo!()
    }

    async fn try_lock_exclusive(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        todo!()
    }

    async fn unlock(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> HostResult<(), wasi::filesystem::ErrorCode> {
        todo!()
    }

    async fn read_via_stream(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        offset: wasi::filesystem::Filesize,
    ) -> HostResult<InputStream, wasi::filesystem::ErrorCode> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = f.dup();

        // Create a stream view for it.
        let reader = FileStream::new_reader(clone, offset);

        // Box it up.
        let boxed: Box<dyn wasi_common::InputStream> = Box::new(reader);

        // Insert the stream view into the table.
        let index = self.table_mut().push(Box::new(boxed)).map_err(convert)?;

        Ok(Ok(index))
    }

    async fn write_via_stream(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        offset: wasi::filesystem::Filesize,
    ) -> HostResult<OutputStream, wasi::filesystem::ErrorCode> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = f.dup();

        // Create a stream view for it.
        let writer = FileStream::new_writer(clone, offset);

        // Box it up.
        let boxed: Box<dyn wasi_common::OutputStream> = Box::new(writer);

        // Insert the stream view into the table.
        let index = self.table_mut().push(Box::new(boxed)).map_err(convert)?;

        Ok(Ok(index))
    }

    async fn append_via_stream(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> HostResult<OutputStream, wasi::filesystem::ErrorCode> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = f.dup();

        // Create a stream view for it.
        let appender = FileStream::new_appender(clone);

        // Box it up.
        let boxed: Box<dyn wasi_common::OutputStream> = Box::new(appender);

        // Insert the stream view into the table.
        let index = self.table_mut().push(Box::new(boxed)).map_err(convert)?;

        Ok(Ok(index))
    }
}

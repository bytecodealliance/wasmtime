#![allow(unused_variables)]

use crate::wasi_io::{InputStream, OutputStream};
use crate::{wasi_filesystem, HostResult, WasiCtx};
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
        use wasi_common::Errno::*;
        use wasi_filesystem::Errno;

        match errno {
            Acces => Errno::Access,
            Again => Errno::Again,
            Already => Errno::Already,
            Badf => Errno::Badf,
            Busy => Errno::Busy,
            Deadlk => Errno::Deadlk,
            Dquot => Errno::Dquot,
            Exist => Errno::Exist,
            Fbig => Errno::Fbig,
            Ilseq => Errno::Ilseq,
            Inprogress => Errno::Inprogress,
            Intr => Errno::Intr,
            Inval => Errno::Inval,
            Io => Errno::Io,
            Isdir => Errno::Isdir,
            Loop => Errno::Loop,
            Mlink => Errno::Mlink,
            Msgsize => Errno::Msgsize,
            Nametoolong => Errno::Nametoolong,
            Nodev => Errno::Nodev,
            Noent => Errno::Noent,
            Nolck => Errno::Nolck,
            Nomem => Errno::Nomem,
            Nospc => Errno::Nospc,
            Nosys => Errno::Nosys,
            Notdir => Errno::Notdir,
            Notempty => Errno::Notempty,
            Notrecoverable => Errno::Notrecoverable,
            Notsup => Errno::Notsup,
            Notty => Errno::Notty,
            Nxio => Errno::Nxio,
            Overflow => Errno::Overflow,
            Perm => Errno::Perm,
            Pipe => Errno::Pipe,
            Rofs => Errno::Rofs,
            Spipe => Errno::Spipe,
            Txtbsy => Errno::Txtbsy,
            Xdev => Errno::Xdev,
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

impl From<wasi_filesystem::OFlags> for wasi_common::file::OFlags {
    fn from(oflags: wasi_filesystem::OFlags) -> Self {
        let mut flags = wasi_common::file::OFlags::empty();
        if contains(oflags, wasi_filesystem::OFlags::CREATE) {
            flags |= wasi_common::file::OFlags::CREATE;
        }
        if contains(oflags, wasi_filesystem::OFlags::DIRECTORY) {
            flags |= wasi_common::file::OFlags::DIRECTORY;
        }
        if contains(oflags, wasi_filesystem::OFlags::EXCL) {
            flags |= wasi_common::file::OFlags::EXCLUSIVE;
        }
        if contains(oflags, wasi_filesystem::OFlags::TRUNC) {
            flags |= wasi_common::file::OFlags::TRUNCATE;
        }
        flags
    }
}

impl From<FdFlags> for wasi_filesystem::DescriptorFlags {
    fn from(fdflags: FdFlags) -> Self {
        let mut flags = wasi_filesystem::DescriptorFlags::empty();
        if contains(fdflags, FdFlags::DSYNC) {
            flags |= wasi_filesystem::DescriptorFlags::DSYNC;
        }
        if contains(fdflags, FdFlags::NONBLOCK) {
            flags |= wasi_filesystem::DescriptorFlags::NONBLOCK;
        }
        if contains(fdflags, FdFlags::RSYNC) {
            flags |= wasi_filesystem::DescriptorFlags::RSYNC;
        }
        if contains(fdflags, FdFlags::SYNC) {
            flags |= wasi_filesystem::DescriptorFlags::SYNC;
        }
        flags
    }
}

impl From<wasi_filesystem::DescriptorFlags> for FdFlags {
    fn from(flags: wasi_filesystem::DescriptorFlags) -> FdFlags {
        let mut fdflags = FdFlags::empty();
        if contains(flags, wasi_filesystem::DescriptorFlags::DSYNC) {
            fdflags |= FdFlags::DSYNC;
        }
        if contains(flags, wasi_filesystem::DescriptorFlags::NONBLOCK) {
            fdflags |= FdFlags::NONBLOCK;
        }
        if contains(flags, wasi_filesystem::DescriptorFlags::RSYNC) {
            fdflags |= FdFlags::RSYNC;
        }
        if contains(flags, wasi_filesystem::DescriptorFlags::SYNC) {
            fdflags |= FdFlags::SYNC;
        }
        fdflags
    }
}

impl From<wasi_common::file::FileType> for wasi_filesystem::DescriptorType {
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

impl From<wasi_common::file::Filestat> for wasi_filesystem::DescriptorStat {
    fn from(stat: wasi_common::file::Filestat) -> Self {
        fn timestamp(time: Option<std::time::SystemTime>) -> wasi_filesystem::Datetime {
            time.map(|t| {
                let since = t.duration_since(SystemTime::UNIX_EPOCH).unwrap();
                wasi_filesystem::Datetime {
                    seconds: since.as_secs(),
                    nanoseconds: since.subsec_nanos(),
                }
            })
            .unwrap_or(wasi_filesystem::Datetime {
                seconds: 0,
                nanoseconds: 0,
            })
        }

        Self {
            dev: stat.device_id,
            ino: stat.inode,
            type_: stat.filetype.into(),
            nlink: stat.nlink,
            size: stat.size,
            atim: timestamp(stat.atim),
            mtim: timestamp(stat.mtim),
            ctim: timestamp(stat.ctim),
        }
    }
}

impl From<wasi_filesystem::Advice> for wasi_common::file::Advice {
    fn from(advice: wasi_filesystem::Advice) -> Self {
        match advice {
            wasi_filesystem::Advice::Normal => wasi_common::file::Advice::Normal,
            wasi_filesystem::Advice::Sequential => wasi_common::file::Advice::Sequential,
            wasi_filesystem::Advice::Random => wasi_common::file::Advice::Random,
            wasi_filesystem::Advice::WillNeed => wasi_common::file::Advice::WillNeed,
            wasi_filesystem::Advice::DontNeed => wasi_common::file::Advice::DontNeed,
            wasi_filesystem::Advice::NoReuse => wasi_common::file::Advice::NoReuse,
        }
    }
}

fn system_time_spec_from_timestamp(
    t: wasi_filesystem::NewTimestamp,
) -> Option<wasi_common::SystemTimeSpec> {
    match t {
        wasi_filesystem::NewTimestamp::NoChange => None,
        wasi_filesystem::NewTimestamp::Now => Some(wasi_common::SystemTimeSpec::SymbolicNow),
        wasi_filesystem::NewTimestamp::Timestamp(datetime) => Some(
            wasi_common::SystemTimeSpec::Absolute(cap_std::time::SystemTime::from_std(
                SystemTime::UNIX_EPOCH + Duration::new(datetime.seconds, datetime.nanoseconds),
            )),
        ),
    }
}

#[async_trait::async_trait]
impl wasi_filesystem::WasiFilesystem for WasiCtx {
    async fn fadvise(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        offset: wasi_filesystem::Filesize,
        len: wasi_filesystem::Filesize,
        advice: wasi_filesystem::Advice,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;
        f.advise(offset, len, advice.into())
            .await
            .map_err(convert)?;
        Ok(Ok(()))
    }

    async fn datasync(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
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
            Err(wasi_filesystem::Errno::Badf.into())
        }
    }

    async fn flags(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<wasi_filesystem::DescriptorFlags, wasi_filesystem::Errno> {
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
            Err(wasi_filesystem::Errno::Badf.into())
        }
    }

    async fn todo_type(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<wasi_filesystem::DescriptorType, wasi_filesystem::Errno> {
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
            Ok(Ok(wasi_filesystem::DescriptorType::Directory))
        } else {
            Err(wasi_filesystem::Errno::Badf.into())
        }
    }

    async fn set_flags(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        flags: wasi_filesystem::DescriptorFlags,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        // FIXME
        Err(wasi_filesystem::Errno::Notsup.into())
    }

    async fn set_size(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        size: wasi_filesystem::Filesize,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;
        f.set_filestat_size(size).await.map_err(convert)?;
        Ok(Ok(()))
    }

    async fn set_times(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        atim: wasi_filesystem::NewTimestamp,
        mtim: wasi_filesystem::NewTimestamp,
    ) -> HostResult<(), wasi_filesystem::Errno> {
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
            Err(wasi_filesystem::Errno::Badf.into())
        }
    }

    async fn pread(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        len: wasi_filesystem::Filesize,
        offset: wasi_filesystem::Filesize,
    ) -> HostResult<(Vec<u8>, bool), wasi_filesystem::Errno> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;

        let mut buffer = vec![0; len.try_into().unwrap_or(usize::MAX)];

        let (bytes_read, end) = f
            .read_vectored_at(&mut [IoSliceMut::new(&mut buffer)], offset)
            .await
            .map_err(convert)?;

        buffer.truncate(bytes_read.try_into().unwrap());

        Ok(Ok((buffer, end)))
    }

    async fn pwrite(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        buf: Vec<u8>,
        offset: wasi_filesystem::Filesize,
    ) -> HostResult<wasi_filesystem::Filesize, wasi_filesystem::Errno> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;

        let bytes_written = f
            .write_vectored_at(&[IoSlice::new(&buf)], offset)
            .await
            .map_err(convert)?;

        Ok(Ok(
            wasi_filesystem::Filesize::try_from(bytes_written).unwrap()
        ))
    }

    async fn readdir(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<wasi_filesystem::DirEntryStream, wasi_filesystem::Errno> {
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

    async fn read_dir_entry(
        &mut self,
        stream: wasi_filesystem::DirEntryStream,
    ) -> HostResult<Option<wasi_filesystem::DirEntry>, wasi_filesystem::Errno> {
        let entity = self
            .table()
            .get::<Mutex<ReaddirIterator>>(stream)
            .map_err(convert)?
            .lock()
            .unwrap()
            .next()
            .transpose()
            .map_err(convert)?;

        Ok(Ok(entity.map(|e| wasi_filesystem::DirEntry {
            ino: Some(e.inode),
            type_: e.filetype.into(),
            name: e.name,
        })))
    }

    async fn drop_dir_entry_stream(
        &mut self,
        stream: wasi_filesystem::DirEntryStream,
    ) -> anyhow::Result<()> {
        self.table_mut()
            .delete::<Mutex<ReaddirIterator>>(stream)
            .map_err(convert)?;

        Ok(())
    }

    async fn sync(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
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
            Err(wasi_filesystem::Errno::Badf.into())
        }
    }

    async fn create_directory_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
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
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<wasi_filesystem::DescriptorStat, wasi_filesystem::Errno> {
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
            Err(wasi_filesystem::Errno::Badf.into())
        }
    }

    async fn stat_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
    ) -> HostResult<wasi_filesystem::DescriptorStat, wasi_filesystem::Errno> {
        let table = self.table();
        Ok(Ok(table
            .get_dir(fd)
            .map_err(convert)?
            .get_path_filestat(
                &path,
                contains(at_flags, wasi_filesystem::AtFlags::SYMLINK_FOLLOW),
            )
            .await
            .map(wasi_filesystem::DescriptorStat::from)
            .map_err(convert)?))
    }

    async fn set_times_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
        atim: wasi_filesystem::NewTimestamp,
        mtim: wasi_filesystem::NewTimestamp,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        let table = self.table();
        Ok(Ok(table
            .get_dir(fd)
            .map_err(convert)?
            .set_times(
                &path,
                system_time_spec_from_timestamp(atim),
                system_time_spec_from_timestamp(mtim),
                contains(at_flags, wasi_filesystem::AtFlags::SYMLINK_FOLLOW),
            )
            .await
            .map_err(convert)?))
    }

    async fn link_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        // TODO delete the at flags from this function
        old_at_flags: wasi_filesystem::AtFlags,
        old_path: String,
        new_descriptor: wasi_filesystem::Descriptor,
        new_path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        let table = self.table();
        let old_dir = table.get_dir(fd).map_err(convert)?;
        let new_dir = table.get_dir(new_descriptor).map_err(convert)?;
        if contains(old_at_flags, wasi_filesystem::AtFlags::SYMLINK_FOLLOW) {
            return Ok(Err(wasi_filesystem::Errno::Inval));
        }
        old_dir
            .hard_link(&old_path, new_dir.deref(), &new_path)
            .await
            .map_err(convert)?;
        Ok(Ok(()))
    }

    async fn open_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        old_path: String,
        oflags: wasi_filesystem::OFlags,
        flags: wasi_filesystem::DescriptorFlags,
        // TODO: How should this be used?
        _mode: wasi_filesystem::Mode,
    ) -> HostResult<wasi_filesystem::Descriptor, wasi_filesystem::Errno> {
        let table = self.table_mut();
        let dir = table.get_dir(fd).map_err(convert)?;

        let symlink_follow = contains(at_flags, wasi_filesystem::AtFlags::SYMLINK_FOLLOW);

        if contains(oflags, wasi_filesystem::OFlags::DIRECTORY) {
            if contains(oflags, wasi_filesystem::OFlags::CREATE)
                || contains(oflags, wasi_filesystem::OFlags::EXCL)
                || contains(oflags, wasi_filesystem::OFlags::TRUNC)
            {
                return Err(wasi_filesystem::Errno::Inval.into());
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
                    contains(flags, wasi_filesystem::DescriptorFlags::READ),
                    contains(flags, wasi_filesystem::DescriptorFlags::WRITE),
                    flags.into(),
                )
                .await
                .map_err(convert)?;
            drop(dir);
            Ok(Ok(table.push(Box::new(file)).map_err(convert)?))
        }
    }

    async fn drop_descriptor(&mut self, fd: wasi_filesystem::Descriptor) -> anyhow::Result<()> {
        let table = self.table_mut();
        // TODO: `WasiCtx` no longer keeps track of which directories are preopens, so we currently have no way
        // of preventing them from being closed.  Is that a problem?
        if !(table.delete::<Box<dyn WasiFile>>(fd).is_ok()
            || table.delete::<Box<dyn WasiDir>>(fd).is_ok())
        {
            anyhow::bail!("{fd} is neither a file nor a directory");
        }
        Ok(())
    }

    async fn readlink_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> HostResult<String, wasi_filesystem::Errno> {
        let table = self.table();
        let dir = table.get_dir(fd).map_err(convert)?;
        let link = dir.read_link(&path).await.map_err(convert)?;
        Ok(link
            .into_os_string()
            .into_string()
            .map_err(|_| wasi_filesystem::Errno::Ilseq))
    }

    async fn remove_directory_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
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
        fd: wasi_filesystem::Descriptor,
        old_path: String,
        new_fd: wasi_filesystem::Descriptor,
        new_path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
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
        fd: wasi_filesystem::Descriptor,
        old_path: String,
        new_path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
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
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> HostResult<(), wasi_filesystem::Errno> {
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
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
        mode: wasi_filesystem::Mode,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn change_directory_permissions_at(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
        mode: wasi_filesystem::Mode,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn lock_shared(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn lock_exclusive(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn try_lock_shared(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn try_lock_exclusive(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn unlock(
        &mut self,
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<(), wasi_filesystem::Errno> {
        todo!()
    }

    async fn read_via_stream(
        &mut self,
        fd: wasi_filesystem::Descriptor,
        offset: wasi_filesystem::Filesize,
    ) -> HostResult<InputStream, wasi_filesystem::Errno> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = f.try_clone().await.map_err(convert)?;

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
        fd: wasi_filesystem::Descriptor,
        offset: wasi_filesystem::Filesize,
    ) -> HostResult<OutputStream, wasi_filesystem::Errno> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = f.try_clone().await.map_err(convert)?;

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
        fd: wasi_filesystem::Descriptor,
    ) -> HostResult<OutputStream, wasi_filesystem::Errno> {
        let f = self.table_mut().get_file_mut(fd).map_err(convert)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = f.try_clone().await.map_err(convert)?;

        // Create a stream view for it.
        let appender = FileStream::new_appender(clone);

        // Box it up.
        let boxed: Box<dyn wasi_common::OutputStream> = Box::new(appender);

        // Insert the stream view into the table.
        let index = self.table_mut().push(Box::new(boxed)).map_err(convert)?;

        Ok(Ok(index))
    }
}

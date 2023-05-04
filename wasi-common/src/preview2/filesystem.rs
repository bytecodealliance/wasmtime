#![allow(unused_variables, unreachable_code)]
use crate::{wasi, Dir, DirPerms, File, FilePerms, TableDirExt, TableFileExt, WasiView};

impl From<crate::TableError> for wasi::filesystem::Error {
    fn from(error: crate::TableError) -> wasi::filesystem::Error {
        match error {
            crate::TableError::Full => wasi::filesystem::Error::trap(anyhow::anyhow!(error)),
            crate::TableError::NotPresent | crate::TableError::WrongType => {
                wasi::filesystem::ErrorCode::BadDescriptor.into()
            }
        }
    }
}

#[async_trait::async_trait]
impl<T: WasiView> wasi::filesystem::Host for T {
    async fn advise(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        offset: wasi::filesystem::Filesize,
        len: wasi::filesystem::Filesize,
        advice: wasi::filesystem::Advice,
    ) -> Result<(), wasi::filesystem::Error> {
        use system_interface::fs::{Advice as A, FileIoExt};
        use wasi::filesystem::Advice;

        let advice = match advice {
            Advice::Normal => A::Normal,
            Advice::Sequential => A::Sequential,
            Advice::Random => A::Random,
            Advice::WillNeed => A::WillNeed,
            Advice::DontNeed => A::DontNeed,
            Advice::NoReuse => A::NoReuse,
        };

        let f = self.table().get_file(fd)?;
        f.file.advise(offset, len, advice)?;
        Ok(())
    }

    async fn sync_data(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        if table.is_file(fd) {
            match table.get_file(fd)?.file.sync_data() {
                Ok(()) => Ok(()),
                // On windows, `sync_data` uses `FileFlushBuffers` which fails with
                // `ERROR_ACCESS_DENIED` if the file is not upen for writing. Ignore
                // this error, for POSIX compatibility.
                #[cfg(windows)]
                Err(e)
                    if e.raw_os_error()
                        == Some(windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED as _) =>
                {
                    Ok(())
                }
                Err(e) => Err(e.into()),
            }
        } else if table.is_dir(fd) {
            Ok(table
                .get_dir(fd)?
                .dir
                .open(std::path::Component::CurDir)?
                .sync_data()?)
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn get_flags(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<wasi::filesystem::DescriptorFlags, wasi::filesystem::Error> {
        use cap_std::io_lifetimes::AsFilelike;
        use system_interface::fs::{FdFlags, GetSetFdFlags};
        use wasi::filesystem::DescriptorFlags;

        fn get_from_fdflags(f: impl AsFilelike) -> std::io::Result<DescriptorFlags> {
            let flags = f.as_filelike().get_fd_flags()?;
            let mut out = DescriptorFlags::empty();
            if flags.contains(FdFlags::DSYNC) {
                out |= DescriptorFlags::REQUESTED_WRITE_SYNC;
            }
            if flags.contains(FdFlags::RSYNC) {
                out |= DescriptorFlags::DATA_INTEGRITY_SYNC;
            }
            if flags.contains(FdFlags::SYNC) {
                out |= DescriptorFlags::FILE_INTEGRITY_SYNC;
            }
            Ok(out)
        }

        let table = self.table();
        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            let mut flags = get_from_fdflags(&f.file)?;
            if f.perms.contains(FilePerms::READ) {
                flags |= DescriptorFlags::READ;
            }
            if f.perms.contains(FilePerms::WRITE) {
                flags |= DescriptorFlags::WRITE;
            }
            Ok(flags)
        } else if table.is_dir(fd) {
            let d = table.get_dir(fd)?;
            let mut flags = get_from_fdflags(&d.dir)?;
            if d.perms.contains(DirPerms::READ) {
                flags |= DescriptorFlags::READ;
            }
            if d.perms.contains(DirPerms::MUTATE) {
                flags |= DescriptorFlags::MUTATE_DIRECTORY;
            }
            Ok(flags)
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn get_type(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<wasi::filesystem::DescriptorType, wasi::filesystem::Error> {
        let table = self.table();

        if table.is_file(fd) {
            let meta = table.get_file(fd)?.file.metadata()?;
            Ok(descriptortype_from(meta.file_type()))
        } else if table.is_dir(fd) {
            Ok(wasi::filesystem::DescriptorType::Directory)
        } else {
            Err(wasi::filesystem::ErrorCode::BadDescriptor.into())
        }
    }

    async fn set_size(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        size: wasi::filesystem::Filesize,
    ) -> Result<(), wasi::filesystem::Error> {
        let f = self.table().get_file(fd)?;
        if !f.perms.contains(FilePerms::WRITE) {
            Err(wasi::filesystem::ErrorCode::NotPermitted)?;
        }
        f.file.set_len(size)?;
        Ok(())
    }

    async fn set_times(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        atim: wasi::filesystem::NewTimestamp,
        mtim: wasi::filesystem::NewTimestamp,
    ) -> Result<(), wasi::filesystem::Error> {
        use fs_set_times::SetTimes;

        let table = self.table();
        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            if !f.perms.contains(FilePerms::WRITE) {
                return Err(wasi::filesystem::ErrorCode::NotPermitted.into());
            }
            let atim = systemtimespec_from(atim)?;
            let mtim = systemtimespec_from(mtim)?;
            f.file.set_times(atim, mtim)?;
            Ok(())
        } else if table.is_dir(fd) {
            let d = table.get_dir(fd)?;
            if !d.perms.contains(DirPerms::MUTATE) {
                return Err(wasi::filesystem::ErrorCode::NotPermitted.into());
            }
            let atim = systemtimespec_from(atim)?;
            let mtim = systemtimespec_from(mtim)?;
            d.dir.set_times(atim, mtim)?;
            Ok(())
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
        use std::io::IoSliceMut;
        use system_interface::fs::FileIoExt;

        let table = self.table();

        let f = table.get_file(fd)?;
        if !f.perms.contains(FilePerms::READ) {
            return Err(wasi::filesystem::ErrorCode::NotPermitted.into());
        }

        let mut buffer = vec![0; len.try_into().unwrap_or(usize::MAX)];
        let (bytes_read, end) = match f
            .file
            .read_vectored_at(&mut [IoSliceMut::new(&mut buffer)], offset)
        {
            Ok(0) => (0, true),
            Ok(n) => (n as u64, false),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => (0, false),
            Err(e) => Err(e)?,
        };

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
        use std::io::IoSlice;
        use system_interface::fs::FileIoExt;

        let table = self.table();
        let f = table.get_file(fd)?;
        if !f.perms.contains(FilePerms::WRITE) {
            return Err(wasi::filesystem::ErrorCode::NotPermitted.into());
        }

        let bytes_written = f.file.write_vectored_at(&[IoSlice::new(&buf)], offset)?;

        Ok(wasi::filesystem::Filesize::try_from(bytes_written).expect("usize fits in Filesize"))
    }

    async fn read_directory(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<wasi::filesystem::DirectoryEntryStream, wasi::filesystem::Error> {
        let table = self.table_mut();

        todo!();
        /*
        let iterator = table.get_dir(fd)?.readdir(ReaddirCursor::from(0)).await?;

        Ok(table.push(Box::new(Mutex::new(iterator)))?)
        */
    }

    async fn read_directory_entry(
        &mut self,
        stream: wasi::filesystem::DirectoryEntryStream,
    ) -> Result<Option<wasi::filesystem::DirectoryEntry>, wasi::filesystem::Error> {
        todo!();
        /*
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
        */
    }

    async fn drop_directory_entry_stream(
        &mut self,
        stream: wasi::filesystem::DirectoryEntryStream,
    ) -> anyhow::Result<()> {
        todo!();
        /*
        // Trap if deletion is not possible:
        self.table_mut().delete::<Mutex<ReaddirIterator>>(stream)?;
        */

        Ok(())
    }

    async fn sync(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        if table.is_file(fd) {
            match table.get_file(fd)?.file.sync_all() {
                Ok(()) => Ok(()),
                // On windows, `sync_data` uses `FileFlushBuffers` which fails with
                // `ERROR_ACCESS_DENIED` if the file is not upen for writing. Ignore
                // this error, for POSIX compatibility.
                #[cfg(windows)]
                Err(e)
                    if e.raw_os_error()
                        == Some(windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED as _) =>
                {
                    Ok(())
                }
                Err(e) => Err(e.into()),
            }
        } else if table.is_dir(fd) {
            Ok(table
                .get_dir(fd)?
                .dir
                .open(std::path::Component::CurDir)?
                .sync_all()?)
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
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(wasi::filesystem::ErrorCode::NotPermitted.into());
        }
        d.dir.create_dir(std::path::Path::new(&path))?;
        Ok(())
    }

    async fn stat(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<wasi::filesystem::DescriptorStat, wasi::filesystem::Error> {
        use cap_fs_ext::MetadataExt;

        let table = self.table();
        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            if !f.perms.contains(FilePerms::READ) {
                return Err(wasi::filesystem::ErrorCode::NotPermitted.into());
            }
            let meta = f.file.metadata()?;
            Ok(wasi::filesystem::DescriptorStat {
                device: meta.dev(),
                inode: meta.ino(),
                type_: descriptortype_from(meta.file_type()),
                link_count: meta.nlink(),
                size: meta.len(),
                data_access_timestamp: meta
                    .accessed()
                    .map(|t| datetime_from(t.into_std()))
                    .unwrap_or(wasi::wall_clock::Datetime {
                        seconds: 0,
                        nanoseconds: 0,
                    }),
                data_modification_timestamp: meta
                    .modified()
                    .map(|t| datetime_from(t.into_std()))
                    .unwrap_or(wasi::wall_clock::Datetime {
                        seconds: 0,
                        nanoseconds: 0,
                    }),
                status_change_timestamp: meta
                    .created()
                    .map(|t| datetime_from(t.into_std()))
                    .unwrap_or(wasi::wall_clock::Datetime {
                        seconds: 0,
                        nanoseconds: 0,
                    }),
            })
        } else if table.is_dir(fd) {
            let d = table.get_dir(fd)?;
            if !d.perms.contains(DirPerms::READ) {
                return Err(wasi::filesystem::ErrorCode::NotPermitted.into());
            }

            let meta = d.dir.dir_metadata()?;
            Ok(wasi::filesystem::DescriptorStat {
                device: meta.dev(),
                inode: meta.ino(),
                type_: descriptortype_from(meta.file_type()),
                link_count: meta.nlink(),
                size: meta.len(),
                data_access_timestamp: meta
                    .accessed()
                    .map(|t| datetime_from(t.into_std()))
                    .unwrap_or(wasi::wall_clock::Datetime {
                        seconds: 0,
                        nanoseconds: 0,
                    }),
                data_modification_timestamp: meta
                    .modified()
                    .map(|t| datetime_from(t.into_std()))
                    .unwrap_or(wasi::wall_clock::Datetime {
                        seconds: 0,
                        nanoseconds: 0,
                    }),
                status_change_timestamp: meta
                    .created()
                    .map(|t| datetime_from(t.into_std()))
                    .unwrap_or(wasi::wall_clock::Datetime {
                        seconds: 0,
                        nanoseconds: 0,
                    }),
            })
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
        use cap_fs_ext::MetadataExt;

        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::READ) {
            return Err(wasi::filesystem::ErrorCode::NotPermitted.into());
        }

        let meta = if at_flags.contains(wasi::filesystem::PathFlags::SYMLINK_FOLLOW) {
            d.dir.metadata(std::path::Path::new(&path))?
        } else {
            d.dir.symlink_metadata(std::path::Path::new(&path))?
        };
        Ok(wasi::filesystem::DescriptorStat {
            device: meta.dev(),
            inode: meta.ino(),
            type_: descriptortype_from(meta.file_type()),
            link_count: meta.nlink(),
            size: meta.len(),
            data_access_timestamp: meta
                .accessed()
                .map(|t| datetime_from(t.into_std()))
                .unwrap_or(wasi::wall_clock::Datetime {
                    seconds: 0,
                    nanoseconds: 0,
                }),
            data_modification_timestamp: meta
                .modified()
                .map(|t| datetime_from(t.into_std()))
                .unwrap_or(wasi::wall_clock::Datetime {
                    seconds: 0,
                    nanoseconds: 0,
                }),
            status_change_timestamp: meta
                .created()
                .map(|t| datetime_from(t.into_std()))
                .unwrap_or(wasi::wall_clock::Datetime {
                    seconds: 0,
                    nanoseconds: 0,
                }),
        })
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
        todo!();
        /*
        Ok(table
            .get_dir(fd)?
            .set_times(
                &path,
                system_time_spec_from_timestamp(atim),
                system_time_spec_from_timestamp(mtim),
                at_flags.contains(wasi::filesystem::PathFlags::SYMLINK_FOLLOW),
            )
            .await?)
        */
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
        todo!();
        /*
        let old_dir = table.get_dir(fd)?;
        let new_dir = table.get_dir(new_descriptor)?;
        if old_at_flags.contains(wasi::filesystem::PathFlags::SYMLINK_FOLLOW) {
            return Err(wasi::filesystem::ErrorCode::Invalid.into());
        }
        old_dir
            .hard_link(&old_path, new_dir.deref(), &new_path)
            .await?;
        Ok(())
        */
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
        todo!();
        /*
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
        */
    }

    async fn drop_descriptor(&mut self, fd: wasi::filesystem::Descriptor) -> anyhow::Result<()> {
        let table = self.table_mut();
        todo!();
        /*
        if !(table.delete::<Box<dyn WasiFile>>(fd).is_ok()
            || table.delete::<Box<dyn WasiDir>>(fd).is_ok())
        {
            // this will trap:
            anyhow::bail!("{fd} is neither a file nor a directory");
        }
        */
        Ok(())
    }

    async fn readlink_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> Result<String, wasi::filesystem::Error> {
        let table = self.table();
        todo!();
        /*
        let dir = table.get_dir(fd)?;
        let link = dir.read_link(&path).await?;
        Ok(link
            .into_os_string()
            .into_string()
            .map_err(|_| wasi::filesystem::ErrorCode::IllegalByteSequence)?)
        */
    }

    async fn remove_directory_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        todo!();
        /*
        Ok(table.get_dir(fd)?.remove_dir(&path).await?)
        */
    }

    async fn rename_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        old_path: String,
        new_fd: wasi::filesystem::Descriptor,
        new_path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        todo!();
        /*
        let old_dir = table.get_dir(fd)?;
        let new_dir = table.get_dir(new_fd)?;
        old_dir
            .rename(&old_path, new_dir.deref(), &new_path)
            .await?;
        Ok(())
        */
    }

    async fn symlink_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        old_path: String,
        new_path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        todo!();
        /*
        Ok(table.get_dir(fd)?.symlink(&old_path, &new_path).await?)
        */
    }

    async fn unlink_file_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        todo!();
        /*
        Ok(table.get_dir(fd)?.unlink_file(&path).await?)
        */
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
    ) -> anyhow::Result<wasi::streams::InputStream> {
        todo!();
        /*
        // Trap if fd lookup fails:
        let f = self.table_mut().get_file_mut(fd)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = f.dup();

        // Create a stream view for it.
        let reader = FileStream::new_reader(clone, offset);

        // Box it up.
        let boxed: Box<dyn crate::InputStream> = Box::new(reader);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self.table_mut().push(Box::new(boxed))?;

        Ok(index)
        */
    }

    async fn write_via_stream(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        offset: wasi::filesystem::Filesize,
    ) -> anyhow::Result<wasi::streams::OutputStream> {
        todo!();
        /*
        // Trap if fd lookup fails:
        let f = self.table_mut().get_file_mut(fd)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = f.dup();

        // Create a stream view for it.
        let writer = FileStream::new_writer(clone, offset);

        // Box it up.
        let boxed: Box<dyn crate::OutputStream> = Box::new(writer);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self.table_mut().push(Box::new(boxed))?;

        Ok(index)
        */
    }

    async fn append_via_stream(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> anyhow::Result<wasi::streams::OutputStream> {
        todo!();
        /*
        // Trap if fd lookup fails:
        let f = self.table_mut().get_file_mut(fd)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = f.dup();

        // Create a stream view for it.
        let appender = FileStream::new_appender(clone);

        // Box it up.
        let boxed: Box<dyn crate::OutputStream> = Box::new(appender);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self.table_mut().push(Box::new(boxed))?;

        Ok(index)
        */
    }
}

#[cfg(unix)]
fn from_raw_os_error(err: Option<i32>) -> Option<wasi::filesystem::Error> {
    use rustix::io::Errno as RustixErrno;
    use wasi::filesystem::ErrorCode;
    if err.is_none() {
        return None;
    }
    Some(match RustixErrno::from_raw_os_error(err.unwrap()) {
        RustixErrno::PIPE => ErrorCode::Pipe.into(),
        RustixErrno::PERM => ErrorCode::NotPermitted.into(),
        RustixErrno::NOENT => ErrorCode::NoEntry.into(),
        RustixErrno::NOMEM => ErrorCode::InsufficientMemory.into(),
        RustixErrno::IO => ErrorCode::Io.into(),
        RustixErrno::BADF => ErrorCode::BadDescriptor.into(),
        RustixErrno::BUSY => ErrorCode::Busy.into(),
        RustixErrno::ACCESS => ErrorCode::Access.into(),
        RustixErrno::NOTDIR => ErrorCode::NotDirectory.into(),
        RustixErrno::ISDIR => ErrorCode::IsDirectory.into(),
        RustixErrno::INVAL => ErrorCode::Invalid.into(),
        RustixErrno::EXIST => ErrorCode::Exist.into(),
        RustixErrno::FBIG => ErrorCode::FileTooLarge.into(),
        RustixErrno::NOSPC => ErrorCode::InsufficientSpace.into(),
        RustixErrno::SPIPE => ErrorCode::InvalidSeek.into(),
        RustixErrno::MLINK => ErrorCode::TooManyLinks.into(),
        RustixErrno::NAMETOOLONG => ErrorCode::NameTooLong.into(),
        RustixErrno::NOTEMPTY => ErrorCode::NotEmpty.into(),
        RustixErrno::LOOP => ErrorCode::Loop.into(),
        RustixErrno::OVERFLOW => ErrorCode::Overflow.into(),
        RustixErrno::ILSEQ => ErrorCode::IllegalByteSequence.into(),
        RustixErrno::NOTSUP => ErrorCode::Unsupported.into(),
        RustixErrno::ALREADY => ErrorCode::Already.into(),
        RustixErrno::INPROGRESS => ErrorCode::InProgress.into(),
        RustixErrno::INTR => ErrorCode::Interrupted.into(),

        // On some platforms.into(), these have the same value as other errno values.
        #[allow(unreachable_patterns)]
        RustixErrno::OPNOTSUPP => ErrorCode::Unsupported.into(),

        _ => return None,
    })
}
#[cfg(windows)]
fn from_raw_os_error(raw_os_error: Option<i32>) -> Option<Error> {
    use wasi::filesystem::ErrorCode;
    use windows_sys::Win32::Foundation;
    use windows_sys::Win32::Networking::WinSock;

    match raw_os_error.map(|code| code as u32) {
        Some(Foundation::ERROR_BAD_ENVIRONMENT) => return Some(ErrorCode::TooBig.into()),
        Some(Foundation::ERROR_FILE_NOT_FOUND) => return Some(ErrorCode::Noent.into()),
        Some(Foundation::ERROR_PATH_NOT_FOUND) => return Some(ErrorCode::Noent.into()),
        Some(Foundation::ERROR_TOO_MANY_OPEN_FILES) => return Some(ErrorCode::Nfile.into()),
        Some(Foundation::ERROR_ACCESS_DENIED) => return Some(ErrorCode::Acces.into()),
        Some(Foundation::ERROR_SHARING_VIOLATION) => return Some(ErrorCode::Acces.into()),
        Some(Foundation::ERROR_PRIVILEGE_NOT_HELD) => return Some(ErrorCode::Perm.into()),
        Some(Foundation::ERROR_INVALID_HANDLE) => return Some(ErrorCode::Badf.into()),
        Some(Foundation::ERROR_INVALID_NAME) => return Some(ErrorCode::Noent.into()),
        Some(Foundation::ERROR_NOT_ENOUGH_MEMORY) => return Some(ErrorCode::Nomem.into()),
        Some(Foundation::ERROR_OUTOFMEMORY) => return Some(ErrorCode::Nomem.into()),
        Some(Foundation::ERROR_DIR_NOT_EMPTY) => return Some(ErrorCode::Notempty.into()),
        Some(Foundation::ERROR_NOT_READY) => return Some(ErrorCode::Busy.into()),
        Some(Foundation::ERROR_BUSY) => return Some(ErrorCode::Busy.into()),
        Some(Foundation::ERROR_NOT_SUPPORTED) => return Some(ErrorCode::Notsup.into()),
        Some(Foundation::ERROR_FILE_EXISTS) => return Some(ErrorCode::Exist.into()),
        Some(Foundation::ERROR_BROKEN_PIPE) => return Some(ErrorCode::Pipe.into()),
        Some(Foundation::ERROR_BUFFER_OVERFLOW) => return Some(ErrorCode::Nametoolong.into()),
        Some(Foundation::ERROR_NOT_A_REPARSE_POINT) => return Some(ErrorCode::Inval.into()),
        Some(Foundation::ERROR_NEGATIVE_SEEK) => return Some(ErrorCode::Inval.into()),
        Some(Foundation::ERROR_DIRECTORY) => return Some(ErrorCode::Notdir.into()),
        Some(Foundation::ERROR_ALREADY_EXISTS) => return Some(ErrorCode::Exist.into()),
        Some(Foundation::ERROR_STOPPED_ON_SYMLINK) => return Some(ErrorCode::Loop.into()),
        Some(Foundation::ERROR_DIRECTORY_NOT_SUPPORTED) => return Some(ErrorCode::Isdir.into()),
        _ => {}
    }

    match raw_os_error {
        Some(WinSock::WSAEWOULDBLOCK) => Some(ErrorCode::Again.into()),
        Some(WinSock::WSAECANCELLED) => Some(ErrorCode::Canceled.into()),
        Some(WinSock::WSA_E_CANCELLED) => Some(ErrorCode::Canceled.into()),
        Some(WinSock::WSAEBADF) => Some(ErrorCode::Badf.into()),
        Some(WinSock::WSAEFAULT) => Some(ErrorCode::Fault.into()),
        Some(WinSock::WSAEINVAL) => Some(ErrorCode::Inval.into()),
        Some(WinSock::WSAEMFILE) => Some(ErrorCode::Mfile.into()),
        Some(WinSock::WSAENAMETOOLONG) => Some(ErrorCode::Nametoolong.into()),
        Some(WinSock::WSAENOTEMPTY) => Some(ErrorCode::Notempty.into()),
        Some(WinSock::WSAELOOP) => Some(ErrorCode::Loop.into()),
        Some(WinSock::WSAEOPNOTSUPP) => Some(ErrorCode::Notsup.into()),
        Some(WinSock::WSAEADDRINUSE) => Some(ErrorCode::Addrinuse.into()),
        Some(WinSock::WSAEACCES) => Some(ErrorCode::Acces.into()),
        Some(WinSock::WSAEADDRNOTAVAIL) => Some(ErrorCode::Addrnotavail.into()),
        Some(WinSock::WSAEAFNOSUPPORT) => Some(ErrorCode::Afnosupport.into()),
        Some(WinSock::WSAEALREADY) => Some(ErrorCode::Already.into()),
        Some(WinSock::WSAECONNABORTED) => Some(ErrorCode::Connaborted.into()),
        Some(WinSock::WSAECONNREFUSED) => Some(ErrorCode::Connrefused.into()),
        Some(WinSock::WSAECONNRESET) => Some(ErrorCode::Connreset.into()),
        Some(WinSock::WSAEDESTADDRREQ) => Some(ErrorCode::Destaddrreq.into()),
        Some(WinSock::WSAEDQUOT) => Some(ErrorCode::Dquot.into()),
        Some(WinSock::WSAEHOSTUNREACH) => Some(ErrorCode::Hostunreach.into()),
        Some(WinSock::WSAEINPROGRESS) => Some(ErrorCode::Inprogress.into()),
        Some(WinSock::WSAEINTR) => Some(ErrorCode::Intr.into()),
        Some(WinSock::WSAEISCONN) => Some(ErrorCode::Isconn.into()),
        Some(WinSock::WSAEMSGSIZE) => Some(ErrorCode::Msgsize.into()),
        Some(WinSock::WSAENETDOWN) => Some(ErrorCode::Netdown.into()),
        Some(WinSock::WSAENETRESET) => Some(ErrorCode::Netreset.into()),
        Some(WinSock::WSAENETUNREACH) => Some(ErrorCode::Netunreach.into()),
        Some(WinSock::WSAENOBUFS) => Some(ErrorCode::Nobufs.into()),
        Some(WinSock::WSAENOPROTOOPT) => Some(ErrorCode::Noprotoopt.into()),
        Some(WinSock::WSAENOTCONN) => Some(ErrorCode::Notconn.into()),
        Some(WinSock::WSAENOTSOCK) => Some(ErrorCode::Notsock.into()),
        Some(WinSock::WSAEPROTONOSUPPORT) => Some(ErrorCode::Protonosupport.into()),
        Some(WinSock::WSAEPROTOTYPE) => Some(ErrorCode::Prototype.into()),
        Some(WinSock::WSAESTALE) => Some(ErrorCode::Stale.into()),
        Some(WinSock::WSAETIMEDOUT) => Some(ErrorCode::Timedout.into()),
        _ => None,
    }
}

impl From<std::io::Error> for wasi::filesystem::Error {
    fn from(err: std::io::Error) -> wasi::filesystem::Error {
        use wasi::filesystem::ErrorCode;
        match from_raw_os_error(err.raw_os_error()) {
            Some(errno) => errno,
            None => match err.kind() {
                std::io::ErrorKind::NotFound => ErrorCode::NoEntry.into(),
                std::io::ErrorKind::PermissionDenied => ErrorCode::NotPermitted.into(),
                std::io::ErrorKind::AlreadyExists => ErrorCode::Exist.into(),
                std::io::ErrorKind::InvalidInput => ErrorCode::Invalid.into(),
                _ => {
                    wasi::filesystem::Error::trap(anyhow::anyhow!(err).context("Unknown OS error"))
                }
            },
        }
    }
}

impl From<cap_rand::Error> for wasi::filesystem::Error {
    fn from(err: cap_rand::Error) -> wasi::filesystem::Error {
        // I picked Error::Io as a 'reasonable default', FIXME dan is this ok?
        from_raw_os_error(err.raw_os_error())
            .unwrap_or_else(|| wasi::filesystem::Error::from(wasi::filesystem::ErrorCode::Io))
    }
}

impl From<std::num::TryFromIntError> for wasi::filesystem::Error {
    fn from(_err: std::num::TryFromIntError) -> wasi::filesystem::Error {
        wasi::filesystem::ErrorCode::Overflow.into()
    }
}

fn descriptortype_from(ft: cap_std::fs::FileType) -> wasi::filesystem::DescriptorType {
    use cap_fs_ext::FileTypeExt;
    use wasi::filesystem::DescriptorType;
    if ft.is_dir() {
        DescriptorType::Directory
    } else if ft.is_symlink() {
        DescriptorType::SymbolicLink
    } else if ft.is_block_device() {
        DescriptorType::BlockDevice
    } else if ft.is_char_device() {
        DescriptorType::CharacterDevice
    } else if ft.is_file() {
        DescriptorType::RegularFile
    } else {
        DescriptorType::Unknown
    }
}

fn systemtimespec_from(
    t: wasi::filesystem::NewTimestamp,
) -> Result<Option<fs_set_times::SystemTimeSpec>, wasi::filesystem::Error> {
    use fs_set_times::SystemTimeSpec;
    use wasi::filesystem::NewTimestamp;
    match t {
        NewTimestamp::NoChange => Ok(None),
        NewTimestamp::Now => Ok(Some(SystemTimeSpec::SymbolicNow)),
        NewTimestamp::Timestamp(st) => Ok(Some(SystemTimeSpec::Absolute(systemtime_from(st)?))),
    }
}

fn systemtime_from(
    t: wasi::wall_clock::Datetime,
) -> Result<std::time::SystemTime, wasi::filesystem::Error> {
    use std::time::{Duration, SystemTime};
    SystemTime::UNIX_EPOCH
        .checked_add(Duration::new(t.seconds, t.nanoseconds))
        .ok_or_else(|| wasi::filesystem::ErrorCode::Overflow.into())
}

fn datetime_from(t: std::time::SystemTime) -> wasi::filesystem::Datetime {
    todo!()
}

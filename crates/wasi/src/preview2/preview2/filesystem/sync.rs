use crate::preview2::bindings::filesystem::filesystem as async_filesystem;
use crate::preview2::bindings::sync_io::filesystem::filesystem as sync_filesystem;
use crate::preview2::block_on;

impl<T: async_filesystem::Host> sync_filesystem::Host for T {
    fn advise(
        &mut self,
        fd: sync_filesystem::Descriptor,
        offset: sync_filesystem::Filesize,
        len: sync_filesystem::Filesize,
        advice: sync_filesystem::Advice,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(block_on(async {
            async_filesystem::Host::advise(self, fd, offset, len, advice.into()).await
        })?)
    }

    fn sync_data(&mut self, fd: sync_filesystem::Descriptor) -> Result<(), sync_filesystem::Error> {
        Ok(block_on(async {
            async_filesystem::Host::sync_data(self, fd).await
        })?)
    }

    fn get_flags(
        &mut self,
        fd: sync_filesystem::Descriptor,
    ) -> Result<sync_filesystem::DescriptorFlags, sync_filesystem::Error> {
        Ok(block_on(async { async_filesystem::Host::get_flags(self, fd).await })?.into())
    }

    fn get_type(
        &mut self,
        fd: sync_filesystem::Descriptor,
    ) -> Result<sync_filesystem::DescriptorType, sync_filesystem::Error> {
        Ok(block_on(async { async_filesystem::Host::get_type(self, fd).await })?.into())
    }

    fn set_size(
        &mut self,
        fd: sync_filesystem::Descriptor,
        size: sync_filesystem::Filesize,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(block_on(async {
            async_filesystem::Host::set_size(self, fd, size).await
        })?)
    }

    /*
    async fn set_times(
        &mut self,
        fd: filesystem::Descriptor,
        atim: filesystem::NewTimestamp,
        mtim: filesystem::NewTimestamp,
    ) -> Result<(), filesystem::Error> {
        use fs_set_times::SetTimes;

        let table = self.table();
        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            if !f.perms.contains(FilePerms::WRITE) {
                return Err(ErrorCode::NotPermitted.into());
            }
            let atim = systemtimespec_from(atim)?;
            let mtim = systemtimespec_from(mtim)?;
            f.block(|f| f.set_times(atim, mtim)).await?;
            Ok(())
        } else if table.is_dir(fd) {
            let d = table.get_dir(fd)?;
            if !d.perms.contains(DirPerms::MUTATE) {
                return Err(ErrorCode::NotPermitted.into());
            }
            let atim = systemtimespec_from(atim)?;
            let mtim = systemtimespec_from(mtim)?;
            d.block(|d| d.set_times(atim, mtim)).await?;
            Ok(())
        } else {
            Err(ErrorCode::BadDescriptor.into())
        }
    }

    async fn read(
        &mut self,
        fd: filesystem::Descriptor,
        len: filesystem::Filesize,
        offset: filesystem::Filesize,
    ) -> Result<(Vec<u8>, bool), filesystem::Error> {
        use std::io::IoSliceMut;
        use system_interface::fs::FileIoExt;

        let table = self.table();

        let f = table.get_file(fd)?;
        if !f.perms.contains(FilePerms::READ) {
            return Err(ErrorCode::NotPermitted.into());
        }

        let (mut buffer, r) = f
            .block(move |f| {
                let mut buffer = vec![0; len.try_into().unwrap_or(usize::MAX)];
                let r = f.read_vectored_at(&mut [IoSliceMut::new(&mut buffer)], offset);
                (buffer, r)
            })
            .await;

        let (bytes_read, state) = crate::preview2::filesystem::read_result(r)?;

        buffer.truncate(
            bytes_read
                .try_into()
                .expect("bytes read into memory as u64 fits in usize"),
        );

        Ok((buffer, state.is_closed()))
    }

    async fn write(
        &mut self,
        fd: filesystem::Descriptor,
        buf: Vec<u8>,
        offset: filesystem::Filesize,
    ) -> Result<filesystem::Filesize, filesystem::Error> {
        use std::io::IoSlice;
        use system_interface::fs::FileIoExt;

        let table = self.table();
        let f = table.get_file(fd)?;
        if !f.perms.contains(FilePerms::WRITE) {
            return Err(ErrorCode::NotPermitted.into());
        }

        let bytes_written = f
            .block(move |f| f.write_vectored_at(&[IoSlice::new(&buf)], offset))
            .await?;

        Ok(filesystem::Filesize::try_from(bytes_written).expect("usize fits in Filesize"))
    }

    async fn read_directory(
        &mut self,
        fd: filesystem::Descriptor,
    ) -> Result<filesystem::DirectoryEntryStream, filesystem::Error> {
        use cap_fs_ext::{DirEntryExt, MetadataExt};

        let table = self.table_mut();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::READ) {
            return Err(ErrorCode::NotPermitted.into());
        }

        enum ReaddirError {
            Io(std::io::Error),
            IllegalSequence,
        }
        impl From<std::io::Error> for ReaddirError {
            fn from(e: std::io::Error) -> ReaddirError {
                ReaddirError::Io(e)
            }
        }

        let entries = d
            .block(|d| {
                // Both `entries` and `full_metadata` perform syscalls, which is why they are done
                // within this `block` call, rather than delay calculating the full metadata
                // for entries when they're demanded later in the iterator chain.
                Ok::<_, std::io::Error>(
                    d.entries()?
                        .map(|entry| {
                            let entry = entry?;
                            let meta = entry.full_metadata()?;
                            let inode = Some(meta.ino());
                            let type_ = descriptortype_from(meta.file_type());
                            let name = entry
                                .file_name()
                                .into_string()
                                .map_err(|_| ReaddirError::IllegalSequence)?;
                            Ok(filesystem::DirectoryEntry { inode, type_, name })
                        })
                        .collect::<Vec<Result<filesystem::DirectoryEntry, ReaddirError>>>(),
                )
            })
            .await?
            .into_iter();

        // On windows, filter out files like `C:\DumpStack.log.tmp` which we
        // can't get full metadata for.
        #[cfg(windows)]
        let entries = entries.filter(|entry| {
            use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION};
            if let Err(ReaddirError::Io(err)) = entry {
                if err.raw_os_error() == Some(ERROR_SHARING_VIOLATION as i32)
                    || err.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32)
                {
                    return false;
                }
            }
            true
        });
        let entries = entries.map(|r| match r {
            Ok(r) => Ok(r),
            Err(ReaddirError::Io(e)) => Err(filesystem::Error::from(e)),
            Err(ReaddirError::IllegalSequence) => Err(ErrorCode::IllegalByteSequence.into()),
        });
        Ok(table.push_readdir(ReaddirIterator::new(entries))?)
    }

    async fn read_directory_entry(
        &mut self,
        stream: filesystem::DirectoryEntryStream,
    ) -> Result<Option<filesystem::DirectoryEntry>, filesystem::Error> {
        let table = self.table();
        let readdir = table.get_readdir(stream)?;
        readdir.next()
    }

    async fn drop_directory_entry_stream(
        &mut self,
        stream: filesystem::DirectoryEntryStream,
    ) -> anyhow::Result<()> {
        self.table_mut().delete_readdir(stream)?;
        Ok(())
    }

    async fn sync(&mut self, fd: filesystem::Descriptor) -> Result<(), filesystem::Error> {
        let table = self.table();
        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            match f.block(|f| f.sync_all()).await {
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
            let d = table.get_dir(fd)?;
            d.block(|d| Ok(d.open(std::path::Component::CurDir)?.sync_all()?))
                .await
        } else {
            Err(ErrorCode::BadDescriptor.into())
        }
    }

    async fn create_directory_at(
        &mut self,
        fd: filesystem::Descriptor,
        path: String,
    ) -> Result<(), filesystem::Error> {
        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        d.block(move |d| d.create_dir(&path)).await?;
        Ok(())
    }

    async fn stat(
        &mut self,
        fd: filesystem::Descriptor,
    ) -> Result<filesystem::DescriptorStat, filesystem::Error> {
        let table = self.table();
        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            // No permissions check on stat: if opened, allowed to stat it
            let meta = f.block(|f| f.metadata()).await?;
            Ok(descriptorstat_from(meta))
        } else if table.is_dir(fd) {
            let d = table.get_dir(fd)?;
            // No permissions check on stat: if opened, allowed to stat it
            let meta = d.block(|d| d.dir_metadata()).await?;
            Ok(descriptorstat_from(meta))
        } else {
            Err(ErrorCode::BadDescriptor.into())
        }
    }

    async fn stat_at(
        &mut self,
        fd: filesystem::Descriptor,
        path_flags: filesystem::PathFlags,
        path: String,
    ) -> Result<filesystem::DescriptorStat, filesystem::Error> {
        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::READ) {
            return Err(ErrorCode::NotPermitted.into());
        }

        let meta = if symlink_follow(path_flags) {
            d.block(move |d| d.metadata(&path)).await?
        } else {
            d.block(move |d| d.symlink_metadata(&path)).await?
        };
        Ok(descriptorstat_from(meta))
    }

    async fn set_times_at(
        &mut self,
        fd: filesystem::Descriptor,
        path_flags: filesystem::PathFlags,
        path: String,
        atim: filesystem::NewTimestamp,
        mtim: filesystem::NewTimestamp,
    ) -> Result<(), filesystem::Error> {
        use cap_fs_ext::DirExt;

        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        let atim = systemtimespec_from(atim)?;
        let mtim = systemtimespec_from(mtim)?;
        if symlink_follow(path_flags) {
            d.block(move |d| {
                d.set_times(
                    &path,
                    atim.map(cap_fs_ext::SystemTimeSpec::from_std),
                    mtim.map(cap_fs_ext::SystemTimeSpec::from_std),
                )
            })
            .await?;
        } else {
            d.block(move |d| {
                d.set_symlink_times(
                    &path,
                    atim.map(cap_fs_ext::SystemTimeSpec::from_std),
                    mtim.map(cap_fs_ext::SystemTimeSpec::from_std),
                )
            })
            .await?;
        }
        Ok(())
    }

    async fn link_at(
        &mut self,
        fd: filesystem::Descriptor,
        // TODO delete the path flags from this function
        old_path_flags: filesystem::PathFlags,
        old_path: String,
        new_descriptor: filesystem::Descriptor,
        new_path: String,
    ) -> Result<(), filesystem::Error> {
        let table = self.table();
        let old_dir = table.get_dir(fd)?;
        if !old_dir.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        let new_dir = table.get_dir(new_descriptor)?;
        if !new_dir.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        if symlink_follow(old_path_flags) {
            return Err(ErrorCode::Invalid.into());
        }
        let new_dir_handle = std::sync::Arc::clone(&new_dir.dir);
        old_dir
            .block(move |d| d.hard_link(&old_path, &new_dir_handle, &new_path))
            .await?;
        Ok(())
    }

    async fn open_at(
        &mut self,
        fd: filesystem::Descriptor,
        path_flags: filesystem::PathFlags,
        path: String,
        oflags: filesystem::OpenFlags,
        flags: filesystem::DescriptorFlags,
        // TODO: These are the permissions to use when creating a new file.
        // Not implemented yet.
        _mode: filesystem::Modes,
    ) -> Result<filesystem::Descriptor, filesystem::Error> {
        use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt, OpenOptionsMaybeDirExt};
        use filesystem::{DescriptorFlags, OpenFlags};
        use system_interface::fs::{FdFlags, GetSetFdFlags};

        let table = self.table_mut();
        if table.is_file(fd) {
            Err(ErrorCode::NotDirectory)?;
        }
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::READ) {
            Err(ErrorCode::NotPermitted)?;
        }

        if !d.perms.contains(DirPerms::MUTATE) {
            if oflags.contains(OpenFlags::CREATE) || oflags.contains(OpenFlags::TRUNCATE) {
                Err(ErrorCode::NotPermitted)?;
            }
            if flags.contains(DescriptorFlags::WRITE) {
                Err(ErrorCode::NotPermitted)?;
            }
        }

        let mut opts = cap_std::fs::OpenOptions::new();
        opts.maybe_dir(true);

        if oflags.contains(OpenFlags::CREATE | OpenFlags::EXCLUSIVE) {
            opts.create_new(true);
            opts.write(true);
        } else if oflags.contains(OpenFlags::CREATE) {
            opts.create(true);
            opts.write(true);
        }
        if oflags.contains(OpenFlags::TRUNCATE) {
            opts.truncate(true);
        }
        if flags.contains(DescriptorFlags::READ) {
            opts.read(true);
        }
        if flags.contains(DescriptorFlags::WRITE) {
            opts.write(true);
        } else {
            // If not opened write, open read. This way the OS lets us open
            // the file, but we can use perms to reject use of the file later.
            opts.read(true);
        }
        if symlink_follow(path_flags) {
            opts.follow(FollowSymlinks::Yes);
        } else {
            opts.follow(FollowSymlinks::No);
        }

        // These flags are not yet supported in cap-std:
        if flags.contains(DescriptorFlags::FILE_INTEGRITY_SYNC)
            | flags.contains(DescriptorFlags::DATA_INTEGRITY_SYNC)
            | flags.contains(DescriptorFlags::REQUESTED_WRITE_SYNC)
        {
            Err(ErrorCode::Unsupported)?;
        }

        if oflags.contains(OpenFlags::DIRECTORY) {
            if oflags.contains(OpenFlags::CREATE)
                || oflags.contains(OpenFlags::EXCLUSIVE)
                || oflags.contains(OpenFlags::TRUNCATE)
            {
                Err(ErrorCode::Invalid)?;
            }
        }

        enum OpenResult {
            Dir(cap_std::fs::Dir),
            File(cap_std::fs::File),
            NotDir,
        }

        let opened = d
            .block::<_, std::io::Result<OpenResult>>(move |d| {
                let mut opened = d.open_with(&path, &opts)?;
                if opened.metadata()?.is_dir() {
                    Ok(OpenResult::Dir(cap_std::fs::Dir::from_std_file(
                        opened.into_std(),
                    )))
                } else if oflags.contains(OpenFlags::DIRECTORY) {
                    Ok(OpenResult::NotDir)
                } else {
                    // FIXME cap-std needs a nonblocking open option so that files reads and writes
                    // are nonblocking. Instead we set it after opening here:
                    let set_fd_flags = opened.new_set_fd_flags(FdFlags::NONBLOCK)?;
                    opened.set_fd_flags(set_fd_flags)?;
                    Ok(OpenResult::File(opened))
                }
            })
            .await?;

        match opened {
            OpenResult::Dir(dir) => Ok(table.push_dir(Dir::new(dir, d.perms, d.file_perms))?),

            OpenResult::File(file) => {
                Ok(table.push_file(File::new(file, mask_file_perms(d.file_perms, flags)))?)
            }

            OpenResult::NotDir => Err(ErrorCode::NotDirectory.into()),
        }
    }

    async fn drop_descriptor(&mut self, fd: filesystem::Descriptor) -> anyhow::Result<()> {
        let table = self.table_mut();

        // Table operations don't need to go in the background thread.
        if table.delete_file(fd).is_err() {
            table.delete_dir(fd)?;
        }

        Ok(())
    }

    async fn readlink_at(
        &mut self,
        fd: filesystem::Descriptor,
        path: String,
    ) -> Result<String, filesystem::Error> {
        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::READ) {
            return Err(ErrorCode::NotPermitted.into());
        }
        let link = d.block(move |d| d.read_link(&path)).await?;
        Ok(link
            .into_os_string()
            .into_string()
            .map_err(|_| ErrorCode::IllegalByteSequence)?)
    }

    async fn remove_directory_at(
        &mut self,
        fd: filesystem::Descriptor,
        path: String,
    ) -> Result<(), filesystem::Error> {
        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        Ok(d.block(move |d| d.remove_dir(&path)).await?)
    }

    async fn rename_at(
        &mut self,
        fd: filesystem::Descriptor,
        old_path: String,
        new_fd: filesystem::Descriptor,
        new_path: String,
    ) -> Result<(), filesystem::Error> {
        let table = self.table();
        let old_dir = table.get_dir(fd)?;
        if !old_dir.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        let new_dir = table.get_dir(new_fd)?;
        if !new_dir.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        let new_dir_handle = std::sync::Arc::clone(&new_dir.dir);
        Ok(old_dir
            .block(move |d| d.rename(&old_path, &new_dir_handle, &new_path))
            .await?)
    }

    async fn symlink_at(
        &mut self,
        fd: filesystem::Descriptor,
        src_path: String,
        dest_path: String,
    ) -> Result<(), filesystem::Error> {
        // On windows, Dir.symlink is provided by DirExt
        #[cfg(windows)]
        use cap_fs_ext::DirExt;

        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        Ok(d.block(move |d| d.symlink(&src_path, &dest_path)).await?)
    }

    async fn unlink_file_at(
        &mut self,
        fd: filesystem::Descriptor,
        path: String,
    ) -> Result<(), filesystem::Error> {
        use cap_fs_ext::DirExt;

        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        Ok(d.block(move |d| d.remove_file_or_symlink(&path)).await?)
    }

    async fn access_at(
        &mut self,
        _fd: filesystem::Descriptor,
        _path_flags: filesystem::PathFlags,
        _path: String,
        _access: filesystem::AccessType,
    ) -> Result<(), filesystem::Error> {
        todo!("filesystem access_at is not implemented")
    }

    async fn change_file_permissions_at(
        &mut self,
        _fd: filesystem::Descriptor,
        _path_flags: filesystem::PathFlags,
        _path: String,
        _mode: filesystem::Modes,
    ) -> Result<(), filesystem::Error> {
        todo!("filesystem change_file_permissions_at is not implemented")
    }

    async fn change_directory_permissions_at(
        &mut self,
        _fd: filesystem::Descriptor,
        _path_flags: filesystem::PathFlags,
        _path: String,
        _mode: filesystem::Modes,
    ) -> Result<(), filesystem::Error> {
        todo!("filesystem change_directory_permissions_at is not implemented")
    }

    async fn lock_shared(&mut self, _fd: filesystem::Descriptor) -> Result<(), filesystem::Error> {
        todo!("filesystem lock_shared is not implemented")
    }

    async fn lock_exclusive(
        &mut self,
        _fd: filesystem::Descriptor,
    ) -> Result<(), filesystem::Error> {
        todo!("filesystem lock_exclusive is not implemented")
    }

    async fn try_lock_shared(
        &mut self,
        _fd: filesystem::Descriptor,
    ) -> Result<(), filesystem::Error> {
        todo!("filesystem try_lock_shared is not implemented")
    }

    async fn try_lock_exclusive(
        &mut self,
        _fd: filesystem::Descriptor,
    ) -> Result<(), filesystem::Error> {
        todo!("filesystem try_lock_exclusive is not implemented")
    }

    async fn unlock(&mut self, _fd: filesystem::Descriptor) -> Result<(), filesystem::Error> {
        todo!("filesystem unlock is not implemented")
    }

    async fn read_via_stream(
        &mut self,
        fd: filesystem::Descriptor,
        offset: filesystem::Filesize,
    ) -> Result<streams::InputStream, filesystem::Error> {
        // Trap if fd lookup fails:
        let f = self.table().get_file(fd)?;

        if !f.perms.contains(FilePerms::READ) {
            Err(filesystem::ErrorCode::BadDescriptor)?;
        }
        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = std::sync::Arc::clone(&f.file);

        // Create a stream view for it.
        let reader = crate::preview2::filesystem::FileInputStream::new(clone, offset);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self.table_mut().push_input_stream(Box::new(reader))?;

        Ok(index)
    }

    async fn write_via_stream(
        &mut self,
        fd: filesystem::Descriptor,
        offset: filesystem::Filesize,
    ) -> Result<streams::OutputStream, filesystem::Error> {
        // Trap if fd lookup fails:
        let f = self.table().get_file(fd)?;

        if !f.perms.contains(FilePerms::WRITE) {
            Err(filesystem::ErrorCode::BadDescriptor)?;
        }

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = std::sync::Arc::clone(&f.file);

        // Create a stream view for it.
        let writer = crate::preview2::filesystem::FileOutputStream::new(clone, offset);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self.table_mut().push_output_stream(Box::new(writer))?;

        Ok(index)
    }

    async fn append_via_stream(
        &mut self,
        fd: filesystem::Descriptor,
    ) -> Result<streams::OutputStream, filesystem::Error> {
        // Trap if fd lookup fails:
        let f = self.table().get_file(fd)?;

        if !f.perms.contains(FilePerms::WRITE) {
            Err(filesystem::ErrorCode::BadDescriptor)?;
        }
        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = std::sync::Arc::clone(&f.file);

        // Create a stream view for it.
        let appender = crate::preview2::filesystem::FileAppendStream::new(clone);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self.table_mut().push_output_stream(Box::new(appender))?;

        Ok(index)
    }
    */
}

impl From<async_filesystem::ErrorCode> for sync_filesystem::ErrorCode {
    fn from(other: async_filesystem::ErrorCode) -> Self {
        use async_filesystem::ErrorCode;
        match other {
            ErrorCode::Access => Self::Access,
            ErrorCode::WouldBlock => Self::WouldBlock,
            ErrorCode::Already => Self::Already,
            ErrorCode::BadDescriptor => Self::BadDescriptor,
            ErrorCode::Busy => Self::Busy,
            ErrorCode::Deadlock => Self::Deadlock,
            ErrorCode::Quota => Self::Quota,
            ErrorCode::Exist => Self::Exist,
            ErrorCode::FileTooLarge => Self::FileTooLarge,
            ErrorCode::IllegalByteSequence => Self::IllegalByteSequence,
            ErrorCode::InProgress => Self::InProgress,
            ErrorCode::Interrupted => Self::Interrupted,
            ErrorCode::Invalid => Self::Invalid,
            ErrorCode::Io => Self::Io,
            ErrorCode::IsDirectory => Self::IsDirectory,
            ErrorCode::Loop => Self::Loop,
            ErrorCode::TooManyLinks => Self::TooManyLinks,
            ErrorCode::MessageSize => Self::MessageSize,
            ErrorCode::NameTooLong => Self::NameTooLong,
            ErrorCode::NoDevice => Self::NoDevice,
            ErrorCode::NoEntry => Self::NoEntry,
            ErrorCode::NoLock => Self::NoLock,
            ErrorCode::InsufficientMemory => Self::InsufficientMemory,
            ErrorCode::InsufficientSpace => Self::InsufficientSpace,
            ErrorCode::NotDirectory => Self::NotDirectory,
            ErrorCode::NotEmpty => Self::NotEmpty,
            ErrorCode::NotRecoverable => Self::NotRecoverable,
            ErrorCode::Unsupported => Self::Unsupported,
            ErrorCode::NoTty => Self::NoTty,
            ErrorCode::NoSuchDevice => Self::NoSuchDevice,
            ErrorCode::Overflow => Self::Overflow,
            ErrorCode::NotPermitted => Self::NotPermitted,
            ErrorCode::Pipe => Self::Pipe,
            ErrorCode::ReadOnly => Self::ReadOnly,
            ErrorCode::InvalidSeek => Self::InvalidSeek,
            ErrorCode::TextFileBusy => Self::TextFileBusy,
            ErrorCode::CrossDevice => Self::CrossDevice,
        }
    }
}

impl From<async_filesystem::Error> for sync_filesystem::Error {
    fn from(other: async_filesystem::Error) -> Self {
        match other.downcast() {
            Ok(errorcode) => Self::from(sync_filesystem::ErrorCode::from(errorcode)),
            Err(other) => Self::trap(other),
        }
    }
}

impl From<sync_filesystem::Advice> for async_filesystem::Advice {
    fn from(other: sync_filesystem::Advice) -> Self {
        use sync_filesystem::Advice;
        match other {
            Advice::Normal => Self::Normal,
            Advice::Sequential => Self::Sequential,
            Advice::Random => Self::Random,
            Advice::WillNeed => Self::WillNeed,
            Advice::DontNeed => Self::DontNeed,
            Advice::NoReuse => Self::NoReuse,
        }
    }
}

impl From<async_filesystem::DescriptorFlags> for sync_filesystem::DescriptorFlags {
    fn from(other: async_filesystem::DescriptorFlags) -> Self {
        let mut out = Self::empty();
        if other.contains(async_filesystem::DescriptorFlags::READ) {
            out |= Self::READ;
        }
        if other.contains(async_filesystem::DescriptorFlags::WRITE) {
            out |= Self::WRITE;
        }
        if other.contains(async_filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC) {
            out |= Self::DATA_INTEGRITY_SYNC;
        }
        if other.contains(async_filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC) {
            out |= Self::REQUESTED_WRITE_SYNC;
        }
        if other.contains(async_filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC) {
            out |= Self::FILE_INTEGRITY_SYNC;
        }
        out
    }
}

impl From<async_filesystem::DescriptorType> for sync_filesystem::DescriptorType {
    fn from(other: async_filesystem::DescriptorType) -> Self {
        use async_filesystem::DescriptorType;
        match other {
            DescriptorType::RegularFile => Self::RegularFile,
            DescriptorType::Directory => Self::Directory,
            DescriptorType::BlockDevice => Self::BlockDevice,
            DescriptorType::CharacterDevice => Self::CharacterDevice,
            DescriptorType::Fifo => Self::Fifo,
            DescriptorType::Socket => Self::Socket,
        }
    }
}

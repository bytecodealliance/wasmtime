use crate::preview2::bindings::clocks::wall_clock;
use crate::preview2::bindings::filesystem::{preopens, types};
use crate::preview2::bindings::io::streams;
use crate::preview2::filesystem::{Dir, File, TableFsExt};
use crate::preview2::{DirPerms, FilePerms, Table, TableError, WasiView};

use types::ErrorCode;

mod sync;

impl From<TableError> for types::Error {
    fn from(error: TableError) -> Self {
        Self::trap(error.into())
    }
}

impl<T: WasiView> preopens::Host for T {
    fn get_directories(&mut self) -> Result<Vec<(types::Descriptor, String)>, anyhow::Error> {
        Ok(self.ctx().preopens.clone())
    }
}

#[async_trait::async_trait]
impl<T: WasiView> types::Host for T {
    async fn advise(
        &mut self,
        fd: types::Descriptor,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<(), types::Error> {
        use system_interface::fs::{Advice as A, FileIoExt};
        use types::Advice;

        let advice = match advice {
            Advice::Normal => A::Normal,
            Advice::Sequential => A::Sequential,
            Advice::Random => A::Random,
            Advice::WillNeed => A::WillNeed,
            Advice::DontNeed => A::DontNeed,
            Advice::NoReuse => A::NoReuse,
        };

        let f = self.table().get_file(fd)?;
        f.spawn_blocking(move |f| f.advise(offset, len, advice))
            .await?;
        Ok(())
    }

    async fn sync_data(&mut self, fd: types::Descriptor) -> Result<(), types::Error> {
        let table = self.table();
        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            match f.spawn_blocking(|f| f.sync_data()).await {
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
            d.spawn_blocking(|d| Ok(d.open(std::path::Component::CurDir)?.sync_data()?))
                .await
        } else {
            Err(ErrorCode::BadDescriptor.into())
        }
    }

    async fn get_flags(
        &mut self,
        fd: types::Descriptor,
    ) -> Result<types::DescriptorFlags, types::Error> {
        use system_interface::fs::{FdFlags, GetSetFdFlags};
        use types::DescriptorFlags;

        fn get_from_fdflags(flags: FdFlags) -> DescriptorFlags {
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
            out
        }

        let table = self.table();
        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            let flags = f.spawn_blocking(|f| f.get_fd_flags()).await?;
            let mut flags = get_from_fdflags(flags);
            if f.perms.contains(FilePerms::READ) {
                flags |= DescriptorFlags::READ;
            }
            if f.perms.contains(FilePerms::WRITE) {
                flags |= DescriptorFlags::WRITE;
            }
            Ok(flags)
        } else if table.is_dir(fd) {
            let d = table.get_dir(fd)?;
            let flags = d.spawn_blocking(|d| d.get_fd_flags()).await?;
            let mut flags = get_from_fdflags(flags);
            if d.perms.contains(DirPerms::READ) {
                flags |= DescriptorFlags::READ;
            }
            if d.perms.contains(DirPerms::MUTATE) {
                flags |= DescriptorFlags::MUTATE_DIRECTORY;
            }
            Ok(flags)
        } else {
            Err(ErrorCode::BadDescriptor.into())
        }
    }

    async fn get_type(
        &mut self,
        fd: types::Descriptor,
    ) -> Result<types::DescriptorType, types::Error> {
        let table = self.table();

        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            let meta = f.spawn_blocking(|f| f.metadata()).await?;
            Ok(descriptortype_from(meta.file_type()))
        } else if table.is_dir(fd) {
            Ok(types::DescriptorType::Directory)
        } else {
            Err(ErrorCode::BadDescriptor.into())
        }
    }

    async fn set_size(
        &mut self,
        fd: types::Descriptor,
        size: types::Filesize,
    ) -> Result<(), types::Error> {
        let f = self.table().get_file(fd)?;
        if !f.perms.contains(FilePerms::WRITE) {
            Err(ErrorCode::NotPermitted)?;
        }
        f.spawn_blocking(move |f| f.set_len(size)).await?;
        Ok(())
    }

    async fn set_times(
        &mut self,
        fd: types::Descriptor,
        atim: types::NewTimestamp,
        mtim: types::NewTimestamp,
    ) -> Result<(), types::Error> {
        use fs_set_times::SetTimes;

        let table = self.table();
        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            if !f.perms.contains(FilePerms::WRITE) {
                return Err(ErrorCode::NotPermitted.into());
            }
            let atim = systemtimespec_from(atim)?;
            let mtim = systemtimespec_from(mtim)?;
            f.spawn_blocking(|f| f.set_times(atim, mtim)).await?;
            Ok(())
        } else if table.is_dir(fd) {
            let d = table.get_dir(fd)?;
            if !d.perms.contains(DirPerms::MUTATE) {
                return Err(ErrorCode::NotPermitted.into());
            }
            let atim = systemtimespec_from(atim)?;
            let mtim = systemtimespec_from(mtim)?;
            d.spawn_blocking(|d| d.set_times(atim, mtim)).await?;
            Ok(())
        } else {
            Err(ErrorCode::BadDescriptor.into())
        }
    }

    async fn read(
        &mut self,
        fd: types::Descriptor,
        len: types::Filesize,
        offset: types::Filesize,
    ) -> Result<(Vec<u8>, bool), types::Error> {
        use std::io::IoSliceMut;
        use system_interface::fs::FileIoExt;

        let table = self.table();

        let f = table.get_file(fd)?;
        if !f.perms.contains(FilePerms::READ) {
            return Err(ErrorCode::NotPermitted.into());
        }

        let (mut buffer, r) = f
            .spawn_blocking(move |f| {
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
        fd: types::Descriptor,
        buf: Vec<u8>,
        offset: types::Filesize,
    ) -> Result<types::Filesize, types::Error> {
        use std::io::IoSlice;
        use system_interface::fs::FileIoExt;

        let table = self.table();
        let f = table.get_file(fd)?;
        if !f.perms.contains(FilePerms::WRITE) {
            return Err(ErrorCode::NotPermitted.into());
        }

        let bytes_written = f
            .spawn_blocking(move |f| f.write_vectored_at(&[IoSlice::new(&buf)], offset))
            .await?;

        Ok(types::Filesize::try_from(bytes_written).expect("usize fits in Filesize"))
    }

    async fn read_directory(
        &mut self,
        fd: types::Descriptor,
    ) -> Result<types::DirectoryEntryStream, types::Error> {
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
            .spawn_blocking(|d| {
                // Both `entries` and `metadata` perform syscalls, which is why they are done
                // within this `block` call, rather than delay calculating the metadata
                // for entries when they're demanded later in the iterator chain.
                Ok::<_, std::io::Error>(
                    d.entries()?
                        .map(|entry| {
                            let entry = entry?;
                            let meta = entry.metadata()?;
                            let type_ = descriptortype_from(meta.file_type());
                            let name = entry
                                .file_name()
                                .into_string()
                                .map_err(|_| ReaddirError::IllegalSequence)?;
                            Ok(types::DirectoryEntry { type_, name })
                        })
                        .collect::<Vec<Result<types::DirectoryEntry, ReaddirError>>>(),
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
            Err(ReaddirError::Io(e)) => Err(types::Error::from(e)),
            Err(ReaddirError::IllegalSequence) => Err(ErrorCode::IllegalByteSequence.into()),
        });
        Ok(table.push_readdir(ReaddirIterator::new(entries))?)
    }

    async fn read_directory_entry(
        &mut self,
        stream: types::DirectoryEntryStream,
    ) -> Result<Option<types::DirectoryEntry>, types::Error> {
        let table = self.table();
        let readdir = table.get_readdir(stream)?;
        readdir.next()
    }

    async fn drop_directory_entry_stream(
        &mut self,
        stream: types::DirectoryEntryStream,
    ) -> anyhow::Result<()> {
        self.table_mut().delete_readdir(stream)?;
        Ok(())
    }

    async fn sync(&mut self, fd: types::Descriptor) -> Result<(), types::Error> {
        let table = self.table();
        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            match f.spawn_blocking(|f| f.sync_all()).await {
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
            d.spawn_blocking(|d| Ok(d.open(std::path::Component::CurDir)?.sync_all()?))
                .await
        } else {
            Err(ErrorCode::BadDescriptor.into())
        }
    }

    async fn create_directory_at(
        &mut self,
        fd: types::Descriptor,
        path: String,
    ) -> Result<(), types::Error> {
        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        d.spawn_blocking(move |d| d.create_dir(&path)).await?;
        Ok(())
    }

    async fn stat(&mut self, fd: types::Descriptor) -> Result<types::DescriptorStat, types::Error> {
        let table = self.table();
        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            // No permissions check on stat: if opened, allowed to stat it
            let meta = f.spawn_blocking(|f| f.metadata()).await?;
            Ok(descriptorstat_from(meta))
        } else if table.is_dir(fd) {
            let d = table.get_dir(fd)?;
            // No permissions check on stat: if opened, allowed to stat it
            let meta = d.spawn_blocking(|d| d.dir_metadata()).await?;
            Ok(descriptorstat_from(meta))
        } else {
            Err(ErrorCode::BadDescriptor.into())
        }
    }

    async fn stat_at(
        &mut self,
        fd: types::Descriptor,
        path_flags: types::PathFlags,
        path: String,
    ) -> Result<types::DescriptorStat, types::Error> {
        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::READ) {
            return Err(ErrorCode::NotPermitted.into());
        }

        let meta = if symlink_follow(path_flags) {
            d.spawn_blocking(move |d| d.metadata(&path)).await?
        } else {
            d.spawn_blocking(move |d| d.symlink_metadata(&path)).await?
        };
        Ok(descriptorstat_from(meta))
    }

    async fn set_times_at(
        &mut self,
        fd: types::Descriptor,
        path_flags: types::PathFlags,
        path: String,
        atim: types::NewTimestamp,
        mtim: types::NewTimestamp,
    ) -> Result<(), types::Error> {
        use cap_fs_ext::DirExt;

        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        let atim = systemtimespec_from(atim)?;
        let mtim = systemtimespec_from(mtim)?;
        if symlink_follow(path_flags) {
            d.spawn_blocking(move |d| {
                d.set_times(
                    &path,
                    atim.map(cap_fs_ext::SystemTimeSpec::from_std),
                    mtim.map(cap_fs_ext::SystemTimeSpec::from_std),
                )
            })
            .await?;
        } else {
            d.spawn_blocking(move |d| {
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
        fd: types::Descriptor,
        // TODO delete the path flags from this function
        old_path_flags: types::PathFlags,
        old_path: String,
        new_descriptor: types::Descriptor,
        new_path: String,
    ) -> Result<(), types::Error> {
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
            .spawn_blocking(move |d| d.hard_link(&old_path, &new_dir_handle, &new_path))
            .await?;
        Ok(())
    }

    async fn open_at(
        &mut self,
        fd: types::Descriptor,
        path_flags: types::PathFlags,
        path: String,
        oflags: types::OpenFlags,
        flags: types::DescriptorFlags,
        // TODO: These are the permissions to use when creating a new file.
        // Not implemented yet.
        _mode: types::Modes,
    ) -> Result<types::Descriptor, types::Error> {
        use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt, OpenOptionsMaybeDirExt};
        use system_interface::fs::{FdFlags, GetSetFdFlags};
        use types::{DescriptorFlags, OpenFlags};

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

        // Represents each possible outcome from the spawn_blocking operation.
        // This makes sure we don't have to give spawn_blocking any way to
        // manipulate the table.
        enum OpenResult {
            Dir(cap_std::fs::Dir),
            File(cap_std::fs::File),
            NotDir,
        }

        let opened = d
            .spawn_blocking::<_, std::io::Result<OpenResult>>(move |d| {
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

    async fn drop_descriptor(&mut self, fd: types::Descriptor) -> anyhow::Result<()> {
        let table = self.table_mut();

        // The Drop will close the file/dir, but if the close syscall
        // blocks the thread, I will face god and walk backwards into hell.
        // tokio::fs::File just uses std::fs::File's Drop impl to close, so
        // it doesn't appear anyone else has found this to be a problem.
        // (Not that they could solve it without async drop...)
        if table.delete_file(fd).is_err() {
            table.delete_dir(fd)?;
        }

        Ok(())
    }

    async fn readlink_at(
        &mut self,
        fd: types::Descriptor,
        path: String,
    ) -> Result<String, types::Error> {
        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::READ) {
            return Err(ErrorCode::NotPermitted.into());
        }
        let link = d.spawn_blocking(move |d| d.read_link(&path)).await?;
        Ok(link
            .into_os_string()
            .into_string()
            .map_err(|_| ErrorCode::IllegalByteSequence)?)
    }

    async fn remove_directory_at(
        &mut self,
        fd: types::Descriptor,
        path: String,
    ) -> Result<(), types::Error> {
        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        Ok(d.spawn_blocking(move |d| d.remove_dir(&path)).await?)
    }

    async fn rename_at(
        &mut self,
        fd: types::Descriptor,
        old_path: String,
        new_fd: types::Descriptor,
        new_path: String,
    ) -> Result<(), types::Error> {
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
            .spawn_blocking(move |d| d.rename(&old_path, &new_dir_handle, &new_path))
            .await?)
    }

    async fn symlink_at(
        &mut self,
        fd: types::Descriptor,
        src_path: String,
        dest_path: String,
    ) -> Result<(), types::Error> {
        // On windows, Dir.symlink is provided by DirExt
        #[cfg(windows)]
        use cap_fs_ext::DirExt;

        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        Ok(d.spawn_blocking(move |d| d.symlink(&src_path, &dest_path))
            .await?)
    }

    async fn unlink_file_at(
        &mut self,
        fd: types::Descriptor,
        path: String,
    ) -> Result<(), types::Error> {
        use cap_fs_ext::DirExt;

        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        Ok(d.spawn_blocking(move |d| d.remove_file_or_symlink(&path))
            .await?)
    }

    async fn access_at(
        &mut self,
        _fd: types::Descriptor,
        _path_flags: types::PathFlags,
        _path: String,
        _access: types::AccessType,
    ) -> Result<(), types::Error> {
        todo!("filesystem access_at is not implemented")
    }

    async fn change_file_permissions_at(
        &mut self,
        _fd: types::Descriptor,
        _path_flags: types::PathFlags,
        _path: String,
        _mode: types::Modes,
    ) -> Result<(), types::Error> {
        todo!("filesystem change_file_permissions_at is not implemented")
    }

    async fn change_directory_permissions_at(
        &mut self,
        _fd: types::Descriptor,
        _path_flags: types::PathFlags,
        _path: String,
        _mode: types::Modes,
    ) -> Result<(), types::Error> {
        todo!("filesystem change_directory_permissions_at is not implemented")
    }

    async fn lock_shared(&mut self, _fd: types::Descriptor) -> Result<(), types::Error> {
        todo!("filesystem lock_shared is not implemented")
    }

    async fn lock_exclusive(&mut self, _fd: types::Descriptor) -> Result<(), types::Error> {
        todo!("filesystem lock_exclusive is not implemented")
    }

    async fn try_lock_shared(&mut self, _fd: types::Descriptor) -> Result<(), types::Error> {
        todo!("filesystem try_lock_shared is not implemented")
    }

    async fn try_lock_exclusive(&mut self, _fd: types::Descriptor) -> Result<(), types::Error> {
        todo!("filesystem try_lock_exclusive is not implemented")
    }

    async fn unlock(&mut self, _fd: types::Descriptor) -> Result<(), types::Error> {
        todo!("filesystem unlock is not implemented")
    }

    async fn read_via_stream(
        &mut self,
        fd: types::Descriptor,
        offset: types::Filesize,
    ) -> Result<streams::InputStream, types::Error> {
        use crate::preview2::{
            filesystem::FileInputStream,
            stream::{InternalInputStream, InternalTableStreamExt},
        };

        // Trap if fd lookup fails:
        let f = self.table().get_file(fd)?;

        if !f.perms.contains(FilePerms::READ) {
            Err(types::ErrorCode::BadDescriptor)?;
        }
        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = std::sync::Arc::clone(&f.file);

        // Create a stream view for it.
        let reader = FileInputStream::new(clone, offset);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self
            .table_mut()
            .push_internal_input_stream(InternalInputStream::File(reader))?;

        Ok(index)
    }

    async fn write_via_stream(
        &mut self,
        fd: types::Descriptor,
        offset: types::Filesize,
    ) -> Result<streams::OutputStream, types::Error> {
        use crate::preview2::{
            filesystem::FileOutputStream,
            stream::{InternalOutputStream, InternalTableStreamExt},
        };

        // Trap if fd lookup fails:
        let f = self.table().get_file(fd)?;

        if !f.perms.contains(FilePerms::WRITE) {
            Err(types::ErrorCode::BadDescriptor)?;
        }

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = std::sync::Arc::clone(&f.file);

        // Create a stream view for it.
        let writer = FileOutputStream::write_at(clone, offset);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self
            .table_mut()
            .push_internal_output_stream(InternalOutputStream::File(writer))?;

        Ok(index)
    }

    async fn append_via_stream(
        &mut self,
        fd: types::Descriptor,
    ) -> Result<streams::OutputStream, types::Error> {
        use crate::preview2::{
            filesystem::FileOutputStream,
            stream::{InternalOutputStream, InternalTableStreamExt},
        };

        // Trap if fd lookup fails:
        let f = self.table().get_file(fd)?;

        if !f.perms.contains(FilePerms::WRITE) {
            Err(types::ErrorCode::BadDescriptor)?;
        }
        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = std::sync::Arc::clone(&f.file);

        // Create a stream view for it.
        let appender = FileOutputStream::append(clone);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self
            .table_mut()
            .push_internal_output_stream(InternalOutputStream::File(appender))?;

        Ok(index)
    }

    async fn is_same_object(
        &mut self,
        a: types::Descriptor,
        b: types::Descriptor,
    ) -> anyhow::Result<bool> {
        use cap_fs_ext::MetadataExt;
        let table = self.table();
        let meta_a = get_descriptor_metadata(table, a).await?;
        let meta_b = get_descriptor_metadata(table, b).await?;
        if meta_a.dev() == meta_b.dev() && meta_a.ino() == meta_b.ino() {
            // MetadataHashValue does not derive eq, so use a pair of
            // comparisons to check equality:
            debug_assert_eq!(
                calculate_metadata_hash(&meta_a).upper,
                calculate_metadata_hash(&meta_b).upper
            );
            debug_assert_eq!(
                calculate_metadata_hash(&meta_a).lower,
                calculate_metadata_hash(&meta_b).lower
            );
            Ok(true)
        } else {
            // Hash collisions are possible, so don't assert the negative here
            Ok(false)
        }
    }
    async fn metadata_hash(
        &mut self,
        fd: types::Descriptor,
    ) -> Result<types::MetadataHashValue, types::Error> {
        let table = self.table();
        let meta = get_descriptor_metadata(table, fd).await?;
        Ok(calculate_metadata_hash(&meta))
    }
    async fn metadata_hash_at(
        &mut self,
        fd: types::Descriptor,
        path_flags: types::PathFlags,
        path: String,
    ) -> Result<types::MetadataHashValue, types::Error> {
        let table = self.table();
        let d = table.get_dir(fd)?;
        // No permissions check on metadata: if dir opened, allowed to stat it
        let meta = d
            .spawn_blocking(move |d| {
                if symlink_follow(path_flags) {
                    d.metadata(path)
                } else {
                    d.symlink_metadata(path)
                }
            })
            .await?;
        Ok(calculate_metadata_hash(&meta))
    }
}

async fn get_descriptor_metadata(
    table: &Table,
    fd: types::Descriptor,
) -> Result<cap_std::fs::Metadata, types::Error> {
    if table.is_file(fd) {
        let f = table.get_file(fd)?;
        // No permissions check on metadata: if opened, allowed to stat it
        Ok(f.spawn_blocking(|f| f.metadata()).await?)
    } else if table.is_dir(fd) {
        let d = table.get_dir(fd)?;
        // No permissions check on metadata: if opened, allowed to stat it
        Ok(d.spawn_blocking(|d| d.dir_metadata()).await?)
    } else {
        Err(ErrorCode::BadDescriptor.into())
    }
}

fn calculate_metadata_hash(meta: &cap_std::fs::Metadata) -> types::MetadataHashValue {
    use cap_fs_ext::MetadataExt;
    // Without incurring any deps, std provides us with a 64 bit hash
    // function:
    use std::hash::Hasher;
    // Note that this means that the metadata hash (which becomes a preview1 ino) may
    // change when a different rustc release is used to build this host implementation:
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write_u64(meta.dev());
    hasher.write_u64(meta.ino());
    let lower = hasher.finish();
    // MetadataHashValue has a pair of 64-bit members for representing a
    // single 128-bit number. However, we only have 64 bits of entropy. To
    // synthesize the upper 64 bits, lets xor the lower half with an arbitrary
    // constant, in this case the 64 bit integer corresponding to the IEEE
    // double representation of (a number as close as possible to) pi.
    // This seems better than just repeating the same bits in the upper and
    // lower parts outright, which could make folks wonder if the struct was
    // mangled in the ABI, or worse yet, lead to consumers of this interface
    // expecting them to be equal.
    let upper = lower ^ 4614256656552045848u64;
    types::MetadataHashValue { lower, upper }
}

#[cfg(unix)]
fn from_raw_os_error(err: Option<i32>) -> Option<types::Error> {
    use rustix::io::Errno as RustixErrno;
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
fn from_raw_os_error(raw_os_error: Option<i32>) -> Option<types::Error> {
    use windows_sys::Win32::Foundation;
    Some(match raw_os_error.map(|code| code as u32) {
        Some(Foundation::ERROR_FILE_NOT_FOUND) => ErrorCode::NoEntry.into(),
        Some(Foundation::ERROR_PATH_NOT_FOUND) => ErrorCode::NoEntry.into(),
        Some(Foundation::ERROR_ACCESS_DENIED) => ErrorCode::Access.into(),
        Some(Foundation::ERROR_SHARING_VIOLATION) => ErrorCode::Access.into(),
        Some(Foundation::ERROR_PRIVILEGE_NOT_HELD) => ErrorCode::NotPermitted.into(),
        Some(Foundation::ERROR_INVALID_HANDLE) => ErrorCode::BadDescriptor.into(),
        Some(Foundation::ERROR_INVALID_NAME) => ErrorCode::NoEntry.into(),
        Some(Foundation::ERROR_NOT_ENOUGH_MEMORY) => ErrorCode::InsufficientMemory.into(),
        Some(Foundation::ERROR_OUTOFMEMORY) => ErrorCode::InsufficientMemory.into(),
        Some(Foundation::ERROR_DIR_NOT_EMPTY) => ErrorCode::NotEmpty.into(),
        Some(Foundation::ERROR_NOT_READY) => ErrorCode::Busy.into(),
        Some(Foundation::ERROR_BUSY) => ErrorCode::Busy.into(),
        Some(Foundation::ERROR_NOT_SUPPORTED) => ErrorCode::Unsupported.into(),
        Some(Foundation::ERROR_FILE_EXISTS) => ErrorCode::Exist.into(),
        Some(Foundation::ERROR_BROKEN_PIPE) => ErrorCode::Pipe.into(),
        Some(Foundation::ERROR_BUFFER_OVERFLOW) => ErrorCode::NameTooLong.into(),
        Some(Foundation::ERROR_NOT_A_REPARSE_POINT) => ErrorCode::Invalid.into(),
        Some(Foundation::ERROR_NEGATIVE_SEEK) => ErrorCode::Invalid.into(),
        Some(Foundation::ERROR_DIRECTORY) => ErrorCode::NotDirectory.into(),
        Some(Foundation::ERROR_ALREADY_EXISTS) => ErrorCode::Exist.into(),
        Some(Foundation::ERROR_STOPPED_ON_SYMLINK) => ErrorCode::Loop.into(),
        Some(Foundation::ERROR_DIRECTORY_NOT_SUPPORTED) => ErrorCode::IsDirectory.into(),
        _ => return None,
    })
}

impl From<std::io::Error> for types::Error {
    fn from(err: std::io::Error) -> types::Error {
        match from_raw_os_error(err.raw_os_error()) {
            Some(errno) => errno,
            None => match err.kind() {
                std::io::ErrorKind::NotFound => ErrorCode::NoEntry.into(),
                std::io::ErrorKind::PermissionDenied => ErrorCode::NotPermitted.into(),
                std::io::ErrorKind::AlreadyExists => ErrorCode::Exist.into(),
                std::io::ErrorKind::InvalidInput => ErrorCode::Invalid.into(),
                _ => types::Error::trap(anyhow::anyhow!(err).context("Unknown OS error")),
            },
        }
    }
}

impl From<cap_rand::Error> for types::Error {
    fn from(err: cap_rand::Error) -> types::Error {
        // I picked Error::Io as a 'reasonable default', FIXME dan is this ok?
        from_raw_os_error(err.raw_os_error()).unwrap_or_else(|| types::Error::from(ErrorCode::Io))
    }
}

impl From<std::num::TryFromIntError> for types::Error {
    fn from(_err: std::num::TryFromIntError) -> types::Error {
        ErrorCode::Overflow.into()
    }
}

fn descriptortype_from(ft: cap_std::fs::FileType) -> types::DescriptorType {
    use cap_fs_ext::FileTypeExt;
    use types::DescriptorType;
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
    t: types::NewTimestamp,
) -> Result<Option<fs_set_times::SystemTimeSpec>, types::Error> {
    use fs_set_times::SystemTimeSpec;
    use types::NewTimestamp;
    match t {
        NewTimestamp::NoChange => Ok(None),
        NewTimestamp::Now => Ok(Some(SystemTimeSpec::SymbolicNow)),
        NewTimestamp::Timestamp(st) => Ok(Some(SystemTimeSpec::Absolute(systemtime_from(st)?))),
    }
}

fn systemtime_from(t: wall_clock::Datetime) -> Result<std::time::SystemTime, types::Error> {
    use std::time::{Duration, SystemTime};
    SystemTime::UNIX_EPOCH
        .checked_add(Duration::new(t.seconds, t.nanoseconds))
        .ok_or_else(|| ErrorCode::Overflow.into())
}

fn datetime_from(t: std::time::SystemTime) -> wall_clock::Datetime {
    // FIXME make this infallible or handle errors properly
    wall_clock::Datetime::try_from(cap_std::time::SystemTime::from_std(t)).unwrap()
}

fn descriptorstat_from(meta: cap_std::fs::Metadata) -> types::DescriptorStat {
    use cap_fs_ext::MetadataExt;
    types::DescriptorStat {
        type_: descriptortype_from(meta.file_type()),
        link_count: meta.nlink(),
        size: meta.len(),
        // FIXME change the wit to make these timestamps optional
        data_access_timestamp: meta
            .accessed()
            .map(|t| datetime_from(t.into_std()))
            .unwrap_or(wall_clock::Datetime {
                seconds: 0,
                nanoseconds: 0,
            }),
        data_modification_timestamp: meta
            .modified()
            .map(|t| datetime_from(t.into_std()))
            .unwrap_or(wall_clock::Datetime {
                seconds: 0,
                nanoseconds: 0,
            }),
        status_change_timestamp: meta
            .created()
            .map(|t| datetime_from(t.into_std()))
            .unwrap_or(wall_clock::Datetime {
                seconds: 0,
                nanoseconds: 0,
            }),
    }
}

fn symlink_follow(path_flags: types::PathFlags) -> bool {
    path_flags.contains(types::PathFlags::SYMLINK_FOLLOW)
}

pub(crate) struct ReaddirIterator(
    std::sync::Mutex<
        Box<dyn Iterator<Item = Result<types::DirectoryEntry, types::Error>> + Send + 'static>,
    >,
);

impl ReaddirIterator {
    fn new(
        i: impl Iterator<Item = Result<types::DirectoryEntry, types::Error>> + Send + 'static,
    ) -> Self {
        ReaddirIterator(std::sync::Mutex::new(Box::new(i)))
    }
    fn next(&self) -> Result<Option<types::DirectoryEntry>, types::Error> {
        self.0.lock().unwrap().next().transpose()
    }
}

impl IntoIterator for ReaddirIterator {
    type Item = Result<types::DirectoryEntry, types::Error>;
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + Send>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_inner().unwrap()
    }
}

pub(crate) trait TableReaddirExt {
    fn push_readdir(&mut self, readdir: ReaddirIterator) -> Result<u32, TableError>;
    fn delete_readdir(&mut self, fd: u32) -> Result<ReaddirIterator, TableError>;
    fn get_readdir(&self, fd: u32) -> Result<&ReaddirIterator, TableError>;
}

impl TableReaddirExt for Table {
    fn push_readdir(&mut self, readdir: ReaddirIterator) -> Result<u32, TableError> {
        self.push(Box::new(readdir))
    }
    fn delete_readdir(&mut self, fd: u32) -> Result<ReaddirIterator, TableError> {
        self.delete(fd)
    }

    fn get_readdir(&self, fd: u32) -> Result<&ReaddirIterator, TableError> {
        self.get(fd)
    }
}

fn mask_file_perms(p: FilePerms, flags: types::DescriptorFlags) -> FilePerms {
    use types::DescriptorFlags;
    let mut out = FilePerms::empty();
    if p.contains(FilePerms::READ) && flags.contains(DescriptorFlags::READ) {
        out |= FilePerms::READ;
    }
    if p.contains(FilePerms::WRITE) && flags.contains(DescriptorFlags::WRITE) {
        out |= FilePerms::WRITE;
    }
    out
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn table_readdir_works() {
        let mut table = Table::new();
        let ix = table
            .push_readdir(ReaddirIterator::new(std::iter::empty()))
            .unwrap();
        let _ = table.get_readdir(ix).unwrap();
        table.delete_readdir(ix).unwrap();
        let _ = table.get_readdir(ix).err().unwrap();
    }
}

use crate::preview2::filesystem::{Dir, File, TableFsExt};
use crate::preview2::stream::TableStreamExt;
use crate::preview2::{wasi, DirPerms, FilePerms, Table, TableError, WasiView};

use wasi::filesystem::ErrorCode;

impl From<TableError> for wasi::filesystem::Error {
    fn from(error: TableError) -> wasi::filesystem::Error {
        match error {
            TableError::Full => wasi::filesystem::Error::trap(anyhow::anyhow!(error)),
            TableError::NotPresent | TableError::WrongType => ErrorCode::BadDescriptor.into(),
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
            Err(ErrorCode::BadDescriptor.into())
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
            let mut flags = get_from_fdflags(&*f.file)?;
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
            Err(ErrorCode::BadDescriptor.into())
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
            Err(ErrorCode::BadDescriptor.into())
        }
    }

    async fn set_size(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        size: wasi::filesystem::Filesize,
    ) -> Result<(), wasi::filesystem::Error> {
        let f = self.table().get_file(fd)?;
        if !f.perms.contains(FilePerms::WRITE) {
            Err(ErrorCode::NotPermitted)?;
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
                return Err(ErrorCode::NotPermitted.into());
            }
            let atim = systemtimespec_from(atim)?;
            let mtim = systemtimespec_from(mtim)?;
            f.file.set_times(atim, mtim)?;
            Ok(())
        } else if table.is_dir(fd) {
            let d = table.get_dir(fd)?;
            if !d.perms.contains(DirPerms::MUTATE) {
                return Err(ErrorCode::NotPermitted.into());
            }
            let atim = systemtimespec_from(atim)?;
            let mtim = systemtimespec_from(mtim)?;
            d.dir.set_times(atim, mtim)?;
            Ok(())
        } else {
            Err(ErrorCode::BadDescriptor.into())
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
            return Err(ErrorCode::NotPermitted.into());
        }

        let mut buffer = vec![0; len.try_into().unwrap_or(usize::MAX)];
        let (bytes_read, end) = crate::preview2::filesystem::read_result(
            f.file
                .read_vectored_at(&mut [IoSliceMut::new(&mut buffer)], offset),
        )?;

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
            return Err(ErrorCode::NotPermitted.into());
        }

        let bytes_written = f.file.write_vectored_at(&[IoSlice::new(&buf)], offset)?;

        Ok(wasi::filesystem::Filesize::try_from(bytes_written).expect("usize fits in Filesize"))
    }

    async fn read_directory(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<wasi::filesystem::DirectoryEntryStream, wasi::filesystem::Error> {
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

        let entries = d.dir.entries()?.map(|entry| {
            let entry = entry?;
            let meta = entry.full_metadata()?;
            let inode = Some(meta.ino());
            let type_ = descriptortype_from(meta.file_type());
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| ReaddirError::IllegalSequence)?;
            Ok(wasi::filesystem::DirectoryEntry { inode, type_, name })
        });
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
            Err(ReaddirError::Io(e)) => Err(wasi::filesystem::Error::from(e)),
            Err(ReaddirError::IllegalSequence) => Err(ErrorCode::IllegalByteSequence.into()),
        });
        Ok(table.push_readdir(ReaddirIterator::new(entries))?)
    }

    async fn read_directory_entry(
        &mut self,
        stream: wasi::filesystem::DirectoryEntryStream,
    ) -> Result<Option<wasi::filesystem::DirectoryEntry>, wasi::filesystem::Error> {
        let table = self.table();
        let readdir = table.get_readdir(stream)?;
        readdir.next()
    }

    async fn drop_directory_entry_stream(
        &mut self,
        stream: wasi::filesystem::DirectoryEntryStream,
    ) -> anyhow::Result<()> {
        self.table_mut().delete_readdir(stream)?;
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
            Err(ErrorCode::BadDescriptor.into())
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
            return Err(ErrorCode::NotPermitted.into());
        }
        d.dir.create_dir(&path)?;
        Ok(())
    }

    async fn stat(
        &mut self,
        fd: wasi::filesystem::Descriptor,
    ) -> Result<wasi::filesystem::DescriptorStat, wasi::filesystem::Error> {
        let table = self.table();
        if table.is_file(fd) {
            let f = table.get_file(fd)?;
            if !f.perms.contains(FilePerms::READ) {
                return Err(ErrorCode::NotPermitted.into());
            }
            let meta = f.file.metadata()?;
            Ok(descriptorstat_from(meta))
        } else if table.is_dir(fd) {
            let d = table.get_dir(fd)?;
            if !d.perms.contains(DirPerms::READ) {
                return Err(ErrorCode::NotPermitted.into());
            }
            let meta = d.dir.dir_metadata()?;
            Ok(descriptorstat_from(meta))
        } else {
            Err(ErrorCode::BadDescriptor.into())
        }
    }

    async fn stat_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path_flags: wasi::filesystem::PathFlags,
        path: String,
    ) -> Result<wasi::filesystem::DescriptorStat, wasi::filesystem::Error> {
        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::READ) {
            return Err(ErrorCode::NotPermitted.into());
        }

        let meta = if symlink_follow(path_flags) {
            d.dir.metadata(&path)?
        } else {
            d.dir.symlink_metadata(&path)?
        };
        Ok(descriptorstat_from(meta))
    }

    async fn set_times_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path_flags: wasi::filesystem::PathFlags,
        path: String,
        atim: wasi::filesystem::NewTimestamp,
        mtim: wasi::filesystem::NewTimestamp,
    ) -> Result<(), wasi::filesystem::Error> {
        use cap_fs_ext::DirExt;

        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        let atim = systemtimespec_from(atim)?;
        let mtim = systemtimespec_from(mtim)?;
        if symlink_follow(path_flags) {
            d.dir.set_times(
                &path,
                atim.map(cap_fs_ext::SystemTimeSpec::from_std),
                mtim.map(cap_fs_ext::SystemTimeSpec::from_std),
            )?;
        } else {
            d.dir.set_symlink_times(
                &path,
                atim.map(cap_fs_ext::SystemTimeSpec::from_std),
                mtim.map(cap_fs_ext::SystemTimeSpec::from_std),
            )?;
        }
        Ok(())
    }

    async fn link_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        // TODO delete the path flags from this function
        old_path_flags: wasi::filesystem::PathFlags,
        old_path: String,
        new_descriptor: wasi::filesystem::Descriptor,
        new_path: String,
    ) -> Result<(), wasi::filesystem::Error> {
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
        old_dir.dir.hard_link(&old_path, &new_dir.dir, &new_path)?;
        Ok(())
    }

    async fn open_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path_flags: wasi::filesystem::PathFlags,
        path: String,
        oflags: wasi::filesystem::OpenFlags,
        flags: wasi::filesystem::DescriptorFlags,
        // TODO: These are the permissions to use when creating a new file.
        // Not implemented yet.
        _mode: wasi::filesystem::Modes,
    ) -> Result<wasi::filesystem::Descriptor, wasi::filesystem::Error> {
        use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt, OpenOptionsMaybeDirExt};
        use system_interface::fs::{FdFlags, GetSetFdFlags};
        use wasi::filesystem::{DescriptorFlags, OpenFlags};

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
        let mut opened = d.dir.open_with(&path, &opts)?;

        if opened.metadata()?.is_dir() {
            Ok(table.push_dir(Dir::new(
                cap_std::fs::Dir::from_std_file(opened.into_std()),
                d.perms,
                d.file_perms,
            ))?)
        } else if oflags.contains(OpenFlags::DIRECTORY) {
            Err(ErrorCode::NotDirectory)?
        } else {
            // FIXME cap-std needs a nonblocking open option so that files reads and writes
            // are nonblocking. Instead we set it after opening here:
            let set_fd_flags = opened.new_set_fd_flags(FdFlags::NONBLOCK)?;
            opened.set_fd_flags(set_fd_flags)?;

            Ok(table.push_file(File::new(opened, d.file_perms))?)
        }
    }

    async fn drop_descriptor(&mut self, fd: wasi::filesystem::Descriptor) -> anyhow::Result<()> {
        let table = self.table_mut();
        if table.delete_file(fd).is_err() {
            table.delete_dir(fd)?;
        }
        Ok(())
    }

    async fn readlink_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> Result<String, wasi::filesystem::Error> {
        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::READ) {
            return Err(ErrorCode::NotPermitted.into());
        }
        let link = d.dir.read_link(&path)?;
        Ok(link
            .into_os_string()
            .into_string()
            .map_err(|_| ErrorCode::IllegalByteSequence)?)
    }

    async fn remove_directory_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        Ok(d.dir.remove_dir(&path)?)
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
        if !old_dir.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        let new_dir = table.get_dir(new_fd)?;
        if !new_dir.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        old_dir.dir.rename(&old_path, &new_dir.dir, &new_path)?;
        Ok(())
    }

    async fn symlink_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        src_path: String,
        dest_path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        // On windows, Dir.symlink is provided by DirExt
        #[cfg(windows)]
        use cap_fs_ext::DirExt;

        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        d.dir.symlink(&src_path, &dest_path)?;
        Ok(())
    }

    async fn unlink_file_at(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        path: String,
    ) -> Result<(), wasi::filesystem::Error> {
        use cap_fs_ext::DirExt;

        let table = self.table();
        let d = table.get_dir(fd)?;
        if !d.perms.contains(DirPerms::MUTATE) {
            return Err(ErrorCode::NotPermitted.into());
        }
        d.dir.remove_file_or_symlink(&path)?;
        Ok(())
    }

    async fn access_at(
        &mut self,
        _fd: wasi::filesystem::Descriptor,
        _path_flags: wasi::filesystem::PathFlags,
        _path: String,
        _access: wasi::filesystem::AccessType,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn change_file_permissions_at(
        &mut self,
        _fd: wasi::filesystem::Descriptor,
        _path_flags: wasi::filesystem::PathFlags,
        _path: String,
        _mode: wasi::filesystem::Modes,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn change_directory_permissions_at(
        &mut self,
        _fd: wasi::filesystem::Descriptor,
        _path_flags: wasi::filesystem::PathFlags,
        _path: String,
        _mode: wasi::filesystem::Modes,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn lock_shared(
        &mut self,
        _fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn lock_exclusive(
        &mut self,
        _fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn try_lock_shared(
        &mut self,
        _fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn try_lock_exclusive(
        &mut self,
        _fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn unlock(
        &mut self,
        _fd: wasi::filesystem::Descriptor,
    ) -> Result<(), wasi::filesystem::Error> {
        todo!()
    }

    async fn read_via_stream(
        &mut self,
        fd: wasi::filesystem::Descriptor,
        offset: wasi::filesystem::Filesize,
    ) -> anyhow::Result<wasi::streams::InputStream> {
        // FIXME: this skips the perm check. We can't return a NotPermitted
        // error code here. Do we need to change the interface?

        // Trap if fd lookup fails:
        let f = self.table().get_file(fd)?;

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
        fd: wasi::filesystem::Descriptor,
        offset: wasi::filesystem::Filesize,
    ) -> anyhow::Result<wasi::streams::OutputStream> {
        // Trap if fd lookup fails:
        let f = self.table().get_file(fd)?;

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
        fd: wasi::filesystem::Descriptor,
    ) -> anyhow::Result<wasi::streams::OutputStream> {
        // Trap if fd lookup fails:
        let f = self.table().get_file(fd)?;

        // Duplicate the file descriptor so that we get an indepenent lifetime.
        let clone = std::sync::Arc::clone(&f.file);

        // Create a stream view for it.
        let appender = crate::preview2::filesystem::FileAppendStream::new(clone);

        // Insert the stream view into the table. Trap if the table is full.
        let index = self.table_mut().push_output_stream(Box::new(appender))?;

        Ok(index)
    }
}

#[cfg(unix)]
fn from_raw_os_error(err: Option<i32>) -> Option<wasi::filesystem::Error> {
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
fn from_raw_os_error(raw_os_error: Option<i32>) -> Option<wasi::filesystem::Error> {
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

impl From<std::io::Error> for wasi::filesystem::Error {
    fn from(err: std::io::Error) -> wasi::filesystem::Error {
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
            .unwrap_or_else(|| wasi::filesystem::Error::from(ErrorCode::Io))
    }
}

impl From<std::num::TryFromIntError> for wasi::filesystem::Error {
    fn from(_err: std::num::TryFromIntError) -> wasi::filesystem::Error {
        ErrorCode::Overflow.into()
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
        .ok_or_else(|| ErrorCode::Overflow.into())
}

fn datetime_from(t: std::time::SystemTime) -> wasi::wall_clock::Datetime {
    // FIXME make this infallible or handle errors properly
    wasi::wall_clock::Datetime::try_from(cap_std::time::SystemTime::from_std(t)).unwrap()
}

fn descriptorstat_from(meta: cap_std::fs::Metadata) -> wasi::filesystem::DescriptorStat {
    use cap_fs_ext::MetadataExt;
    wasi::filesystem::DescriptorStat {
        // FIXME didn't we agree that the wit could be changed to make the device and ino fields
        // optional?
        device: meta.dev(),
        inode: meta.ino(),
        type_: descriptortype_from(meta.file_type()),
        link_count: meta.nlink(),
        size: meta.len(),
        // FIXME change the wit to make these timestamps optional
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
    }
}

fn symlink_follow(path_flags: wasi::filesystem::PathFlags) -> bool {
    path_flags.contains(wasi::filesystem::PathFlags::SYMLINK_FOLLOW)
}

pub(crate) struct ReaddirIterator(
    std::sync::Mutex<
        Box<
            dyn Iterator<Item = Result<wasi::filesystem::DirectoryEntry, wasi::filesystem::Error>>
                + Send
                + 'static,
        >,
    >,
);

impl ReaddirIterator {
    fn new(
        i: impl Iterator<Item = Result<wasi::filesystem::DirectoryEntry, wasi::filesystem::Error>>
            + Send
            + 'static,
    ) -> Self {
        ReaddirIterator(std::sync::Mutex::new(Box::new(i)))
    }
    fn next(&self) -> Result<Option<wasi::filesystem::DirectoryEntry>, wasi::filesystem::Error> {
        self.0.lock().unwrap().next().transpose()
    }
}

impl IntoIterator for ReaddirIterator {
    type Item = Result<wasi::filesystem::DirectoryEntry, wasi::filesystem::Error>;
    type IntoIter = Box<dyn Iterator<Item = Self::Item>>;

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

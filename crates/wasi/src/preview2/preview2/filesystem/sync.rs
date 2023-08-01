use crate::preview2::bindings::filesystem::filesystem as async_filesystem;
use crate::preview2::bindings::sync_io::filesystem::filesystem as sync_filesystem;
use crate::preview2::bindings::sync_io::io::streams;
use crate::preview2::in_tokio;

impl<T: async_filesystem::Host> sync_filesystem::Host for T {
    fn advise(
        &mut self,
        fd: sync_filesystem::Descriptor,
        offset: sync_filesystem::Filesize,
        len: sync_filesystem::Filesize,
        advice: sync_filesystem::Advice,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::advise(self, fd, offset, len, advice.into()).await
        })?)
    }

    fn sync_data(&mut self, fd: sync_filesystem::Descriptor) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::sync_data(self, fd).await
        })?)
    }

    fn get_flags(
        &mut self,
        fd: sync_filesystem::Descriptor,
    ) -> Result<sync_filesystem::DescriptorFlags, sync_filesystem::Error> {
        Ok(in_tokio(async { async_filesystem::Host::get_flags(self, fd).await })?.into())
    }

    fn get_type(
        &mut self,
        fd: sync_filesystem::Descriptor,
    ) -> Result<sync_filesystem::DescriptorType, sync_filesystem::Error> {
        Ok(in_tokio(async { async_filesystem::Host::get_type(self, fd).await })?.into())
    }

    fn set_size(
        &mut self,
        fd: sync_filesystem::Descriptor,
        size: sync_filesystem::Filesize,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::set_size(self, fd, size).await
        })?)
    }

    fn set_times(
        &mut self,
        fd: sync_filesystem::Descriptor,
        atim: sync_filesystem::NewTimestamp,
        mtim: sync_filesystem::NewTimestamp,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::set_times(self, fd, atim.into(), mtim.into()).await
        })?)
    }

    fn read(
        &mut self,
        fd: sync_filesystem::Descriptor,
        len: sync_filesystem::Filesize,
        offset: sync_filesystem::Filesize,
    ) -> Result<(Vec<u8>, bool), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::read(self, fd, len, offset).await
        })?)
    }

    fn write(
        &mut self,
        fd: sync_filesystem::Descriptor,
        buf: Vec<u8>,
        offset: sync_filesystem::Filesize,
    ) -> Result<sync_filesystem::Filesize, sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::write(self, fd, buf, offset).await
        })?)
    }

    fn read_directory(
        &mut self,
        fd: sync_filesystem::Descriptor,
    ) -> Result<sync_filesystem::DirectoryEntryStream, sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::read_directory(self, fd).await
        })?)
    }

    fn read_directory_entry(
        &mut self,
        stream: sync_filesystem::DirectoryEntryStream,
    ) -> Result<Option<sync_filesystem::DirectoryEntry>, sync_filesystem::Error> {
        Ok(
            in_tokio(async { async_filesystem::Host::read_directory_entry(self, stream).await })?
                .map(|e| e.into()),
        )
    }

    fn drop_directory_entry_stream(
        &mut self,
        stream: sync_filesystem::DirectoryEntryStream,
    ) -> anyhow::Result<()> {
        Ok(in_tokio(async {
            async_filesystem::Host::drop_directory_entry_stream(self, stream).await
        })?)
    }

    fn sync(&mut self, fd: sync_filesystem::Descriptor) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::sync(self, fd).await
        })?)
    }

    fn create_directory_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        path: String,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::create_directory_at(self, fd, path).await
        })?)
    }

    fn stat(
        &mut self,
        fd: sync_filesystem::Descriptor,
    ) -> Result<sync_filesystem::DescriptorStat, sync_filesystem::Error> {
        Ok(in_tokio(async { async_filesystem::Host::stat(self, fd).await })?.into())
    }

    fn stat_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        path_flags: sync_filesystem::PathFlags,
        path: String,
    ) -> Result<sync_filesystem::DescriptorStat, sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::stat_at(self, fd, path_flags.into(), path).await
        })?
        .into())
    }

    fn set_times_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        path_flags: sync_filesystem::PathFlags,
        path: String,
        atim: sync_filesystem::NewTimestamp,
        mtim: sync_filesystem::NewTimestamp,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::set_times_at(
                self,
                fd,
                path_flags.into(),
                path,
                atim.into(),
                mtim.into(),
            )
            .await
        })?)
    }

    fn link_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        // TODO delete the path flags from this function
        old_path_flags: sync_filesystem::PathFlags,
        old_path: String,
        new_descriptor: sync_filesystem::Descriptor,
        new_path: String,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::link_at(
                self,
                fd,
                old_path_flags.into(),
                old_path,
                new_descriptor,
                new_path,
            )
            .await
        })?)
    }

    fn open_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        path_flags: sync_filesystem::PathFlags,
        path: String,
        oflags: sync_filesystem::OpenFlags,
        flags: sync_filesystem::DescriptorFlags,
        mode: sync_filesystem::Modes,
    ) -> Result<sync_filesystem::Descriptor, sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::open_at(
                self,
                fd,
                path_flags.into(),
                path,
                oflags.into(),
                flags.into(),
                mode.into(),
            )
            .await
        })?)
    }

    fn drop_descriptor(&mut self, fd: sync_filesystem::Descriptor) -> anyhow::Result<()> {
        Ok(in_tokio(async {
            async_filesystem::Host::drop_descriptor(self, fd).await
        })?)
    }

    fn readlink_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        path: String,
    ) -> Result<String, sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::readlink_at(self, fd, path).await
        })?)
    }

    fn remove_directory_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        path: String,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::remove_directory_at(self, fd, path).await
        })?)
    }

    fn rename_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        old_path: String,
        new_fd: sync_filesystem::Descriptor,
        new_path: String,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::rename_at(self, fd, old_path, new_fd, new_path).await
        })?)
    }

    fn symlink_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        src_path: String,
        dest_path: String,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::symlink_at(self, fd, src_path, dest_path).await
        })?)
    }

    fn unlink_file_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        path: String,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::unlink_file_at(self, fd, path).await
        })?)
    }

    fn access_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        path_flags: sync_filesystem::PathFlags,
        path: String,
        access: sync_filesystem::AccessType,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::access_at(self, fd, path_flags.into(), path, access.into())
                .await
        })?)
    }

    fn change_file_permissions_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        path_flags: sync_filesystem::PathFlags,
        path: String,
        mode: sync_filesystem::Modes,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::change_file_permissions_at(
                self,
                fd,
                path_flags.into(),
                path,
                mode.into(),
            )
            .await
        })?)
    }

    fn change_directory_permissions_at(
        &mut self,
        fd: sync_filesystem::Descriptor,
        path_flags: sync_filesystem::PathFlags,
        path: String,
        mode: sync_filesystem::Modes,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::change_directory_permissions_at(
                self,
                fd,
                path_flags.into(),
                path,
                mode.into(),
            )
            .await
        })?)
    }

    fn lock_shared(
        &mut self,
        fd: sync_filesystem::Descriptor,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::lock_shared(self, fd).await
        })?)
    }

    fn lock_exclusive(
        &mut self,
        fd: sync_filesystem::Descriptor,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::lock_exclusive(self, fd).await
        })?)
    }

    fn try_lock_shared(
        &mut self,
        fd: sync_filesystem::Descriptor,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::try_lock_shared(self, fd).await
        })?)
    }

    fn try_lock_exclusive(
        &mut self,
        fd: sync_filesystem::Descriptor,
    ) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::try_lock_exclusive(self, fd).await
        })?)
    }

    fn unlock(&mut self, fd: sync_filesystem::Descriptor) -> Result<(), sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::unlock(self, fd).await
        })?)
    }

    fn read_via_stream(
        &mut self,
        fd: sync_filesystem::Descriptor,
        offset: sync_filesystem::Filesize,
    ) -> Result<streams::InputStream, sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::read_via_stream(self, fd, offset).await
        })?)
    }

    fn write_via_stream(
        &mut self,
        fd: sync_filesystem::Descriptor,
        offset: sync_filesystem::Filesize,
    ) -> Result<streams::OutputStream, sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::write_via_stream(self, fd, offset).await
        })?)
    }

    fn append_via_stream(
        &mut self,
        fd: sync_filesystem::Descriptor,
    ) -> Result<streams::OutputStream, sync_filesystem::Error> {
        Ok(in_tokio(async {
            async_filesystem::Host::append_via_stream(self, fd).await
        })?)
    }
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
        if other.contains(async_filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC) {
            out |= Self::FILE_INTEGRITY_SYNC;
        }
        if other.contains(async_filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC) {
            out |= Self::DATA_INTEGRITY_SYNC;
        }
        if other.contains(async_filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC) {
            out |= Self::REQUESTED_WRITE_SYNC;
        }
        if other.contains(async_filesystem::DescriptorFlags::MUTATE_DIRECTORY) {
            out |= Self::MUTATE_DIRECTORY;
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
            DescriptorType::SymbolicLink => Self::SymbolicLink,
            DescriptorType::Unknown => Self::Unknown,
        }
    }
}

impl From<async_filesystem::DirectoryEntry> for sync_filesystem::DirectoryEntry {
    fn from(other: async_filesystem::DirectoryEntry) -> Self {
        Self {
            inode: other.inode,
            type_: other.type_.into(),
            name: other.name,
        }
    }
}

impl From<async_filesystem::DescriptorStat> for sync_filesystem::DescriptorStat {
    fn from(other: async_filesystem::DescriptorStat) -> Self {
        Self {
            device: other.device,
            inode: other.inode,
            type_: other.type_.into(),
            link_count: other.link_count,
            size: other.size,
            data_access_timestamp: other.data_access_timestamp,
            data_modification_timestamp: other.data_modification_timestamp,
            status_change_timestamp: other.status_change_timestamp,
        }
    }
}

impl From<sync_filesystem::PathFlags> for async_filesystem::PathFlags {
    fn from(other: sync_filesystem::PathFlags) -> Self {
        let mut out = Self::empty();
        if other.contains(sync_filesystem::PathFlags::SYMLINK_FOLLOW) {
            out |= Self::SYMLINK_FOLLOW;
        }
        out
    }
}

impl From<sync_filesystem::NewTimestamp> for async_filesystem::NewTimestamp {
    fn from(other: sync_filesystem::NewTimestamp) -> Self {
        use sync_filesystem::NewTimestamp;
        match other {
            NewTimestamp::NoChange => Self::NoChange,
            NewTimestamp::Now => Self::Now,
            NewTimestamp::Timestamp(datetime) => Self::Timestamp(datetime),
        }
    }
}

impl From<sync_filesystem::OpenFlags> for async_filesystem::OpenFlags {
    fn from(other: sync_filesystem::OpenFlags) -> Self {
        let mut out = Self::empty();
        if other.contains(sync_filesystem::OpenFlags::CREATE) {
            out |= Self::CREATE;
        }
        if other.contains(sync_filesystem::OpenFlags::DIRECTORY) {
            out |= Self::DIRECTORY;
        }
        if other.contains(sync_filesystem::OpenFlags::EXCLUSIVE) {
            out |= Self::EXCLUSIVE;
        }
        if other.contains(sync_filesystem::OpenFlags::TRUNCATE) {
            out |= Self::TRUNCATE;
        }
        out
    }
}
impl From<sync_filesystem::DescriptorFlags> for async_filesystem::DescriptorFlags {
    fn from(other: sync_filesystem::DescriptorFlags) -> Self {
        let mut out = Self::empty();
        if other.contains(sync_filesystem::DescriptorFlags::READ) {
            out |= Self::READ;
        }
        if other.contains(sync_filesystem::DescriptorFlags::WRITE) {
            out |= Self::WRITE;
        }
        if other.contains(sync_filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC) {
            out |= Self::FILE_INTEGRITY_SYNC;
        }
        if other.contains(sync_filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC) {
            out |= Self::DATA_INTEGRITY_SYNC;
        }
        if other.contains(sync_filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC) {
            out |= Self::REQUESTED_WRITE_SYNC;
        }
        if other.contains(sync_filesystem::DescriptorFlags::MUTATE_DIRECTORY) {
            out |= Self::MUTATE_DIRECTORY;
        }
        out
    }
}
impl From<sync_filesystem::Modes> for async_filesystem::Modes {
    fn from(other: sync_filesystem::Modes) -> Self {
        let mut out = Self::empty();
        if other.contains(sync_filesystem::Modes::READABLE) {
            out |= Self::READABLE;
        }
        if other.contains(sync_filesystem::Modes::WRITABLE) {
            out |= Self::WRITABLE;
        }
        if other.contains(sync_filesystem::Modes::EXECUTABLE) {
            out |= Self::EXECUTABLE;
        }
        out
    }
}
impl From<sync_filesystem::AccessType> for async_filesystem::AccessType {
    fn from(other: sync_filesystem::AccessType) -> Self {
        use sync_filesystem::AccessType;
        match other {
            AccessType::Access(modes) => Self::Access(modes.into()),
            AccessType::Exists => Self::Exists,
        }
    }
}

use crate::filesystem::WasiFilesystemCtxView;
use crate::p2::bindings::filesystem::types as async_filesystem;
use crate::p2::bindings::sync::filesystem::types as sync_filesystem;
use crate::p2::bindings::sync::io::streams;
use crate::p2::{FsError, FsResult};
use crate::runtime::in_tokio;
use wasmtime::component::Resource;

impl sync_filesystem::Host for WasiFilesystemCtxView<'_> {
    fn convert_error_code(&mut self, err: FsError) -> wasmtime::Result<sync_filesystem::ErrorCode> {
        Ok(async_filesystem::Host::convert_error_code(self, err)?.into())
    }

    fn filesystem_error_code(
        &mut self,
        err: Resource<streams::Error>,
    ) -> wasmtime::Result<Option<sync_filesystem::ErrorCode>> {
        Ok(async_filesystem::Host::filesystem_error_code(self, err)?.map(|e| e.into()))
    }
}

impl sync_filesystem::HostDescriptor for WasiFilesystemCtxView<'_> {
    fn advise(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        offset: sync_filesystem::Filesize,
        len: sync_filesystem::Filesize,
        advice: sync_filesystem::Advice,
    ) -> FsResult<()> {
        in_tokio(async {
            async_filesystem::HostDescriptor::advise(self, fd, offset, len, advice.into()).await
        })
    }

    fn sync_data(&mut self, fd: Resource<sync_filesystem::Descriptor>) -> FsResult<()> {
        in_tokio(async { async_filesystem::HostDescriptor::sync_data(self, fd).await })
    }

    fn get_flags(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
    ) -> FsResult<sync_filesystem::DescriptorFlags> {
        Ok(in_tokio(async { async_filesystem::HostDescriptor::get_flags(self, fd).await })?.into())
    }

    fn get_type(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
    ) -> FsResult<sync_filesystem::DescriptorType> {
        Ok(in_tokio(async { async_filesystem::HostDescriptor::get_type(self, fd).await })?.into())
    }

    fn set_size(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        size: sync_filesystem::Filesize,
    ) -> FsResult<()> {
        in_tokio(async { async_filesystem::HostDescriptor::set_size(self, fd, size).await })
    }

    fn set_times(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        atim: sync_filesystem::NewTimestamp,
        mtim: sync_filesystem::NewTimestamp,
    ) -> FsResult<()> {
        in_tokio(async {
            async_filesystem::HostDescriptor::set_times(self, fd, atim.into(), mtim.into()).await
        })
    }

    fn read(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        len: sync_filesystem::Filesize,
        offset: sync_filesystem::Filesize,
    ) -> FsResult<(Vec<u8>, bool)> {
        in_tokio(async { async_filesystem::HostDescriptor::read(self, fd, len, offset).await })
    }

    fn write(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        buf: Vec<u8>,
        offset: sync_filesystem::Filesize,
    ) -> FsResult<sync_filesystem::Filesize> {
        in_tokio(async { async_filesystem::HostDescriptor::write(self, fd, buf, offset).await })
    }

    fn read_directory(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
    ) -> FsResult<Resource<sync_filesystem::DirectoryEntryStream>> {
        in_tokio(async { async_filesystem::HostDescriptor::read_directory(self, fd).await })
    }

    fn sync(&mut self, fd: Resource<sync_filesystem::Descriptor>) -> FsResult<()> {
        in_tokio(async { async_filesystem::HostDescriptor::sync(self, fd).await })
    }

    fn create_directory_at(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        path: String,
    ) -> FsResult<()> {
        in_tokio(async {
            async_filesystem::HostDescriptor::create_directory_at(self, fd, path).await
        })
    }

    fn stat(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
    ) -> FsResult<sync_filesystem::DescriptorStat> {
        Ok(in_tokio(async { async_filesystem::HostDescriptor::stat(self, fd).await })?.into())
    }

    fn stat_at(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        path_flags: sync_filesystem::PathFlags,
        path: String,
    ) -> FsResult<sync_filesystem::DescriptorStat> {
        Ok(in_tokio(async {
            async_filesystem::HostDescriptor::stat_at(self, fd, path_flags.into(), path).await
        })?
        .into())
    }

    fn set_times_at(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        path_flags: sync_filesystem::PathFlags,
        path: String,
        atim: sync_filesystem::NewTimestamp,
        mtim: sync_filesystem::NewTimestamp,
    ) -> FsResult<()> {
        in_tokio(async {
            async_filesystem::HostDescriptor::set_times_at(
                self,
                fd,
                path_flags.into(),
                path,
                atim.into(),
                mtim.into(),
            )
            .await
        })
    }

    fn link_at(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        // TODO delete the path flags from this function
        old_path_flags: sync_filesystem::PathFlags,
        old_path: String,
        new_descriptor: Resource<sync_filesystem::Descriptor>,
        new_path: String,
    ) -> FsResult<()> {
        in_tokio(async {
            async_filesystem::HostDescriptor::link_at(
                self,
                fd,
                old_path_flags.into(),
                old_path,
                new_descriptor,
                new_path,
            )
            .await
        })
    }

    fn open_at(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        path_flags: sync_filesystem::PathFlags,
        path: String,
        oflags: sync_filesystem::OpenFlags,
        flags: sync_filesystem::DescriptorFlags,
    ) -> FsResult<Resource<sync_filesystem::Descriptor>> {
        in_tokio(async {
            async_filesystem::HostDescriptor::open_at(
                self,
                fd,
                path_flags.into(),
                path,
                oflags.into(),
                flags.into(),
            )
            .await
        })
    }

    fn drop(&mut self, fd: Resource<sync_filesystem::Descriptor>) -> wasmtime::Result<()> {
        async_filesystem::HostDescriptor::drop(self, fd)
    }

    fn readlink_at(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        path: String,
    ) -> FsResult<String> {
        in_tokio(async { async_filesystem::HostDescriptor::readlink_at(self, fd, path).await })
    }

    fn remove_directory_at(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        path: String,
    ) -> FsResult<()> {
        in_tokio(async {
            async_filesystem::HostDescriptor::remove_directory_at(self, fd, path).await
        })
    }

    fn rename_at(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        old_path: String,
        new_fd: Resource<sync_filesystem::Descriptor>,
        new_path: String,
    ) -> FsResult<()> {
        in_tokio(async {
            async_filesystem::HostDescriptor::rename_at(self, fd, old_path, new_fd, new_path).await
        })
    }

    fn symlink_at(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        src_path: String,
        dest_path: String,
    ) -> FsResult<()> {
        in_tokio(async {
            async_filesystem::HostDescriptor::symlink_at(self, fd, src_path, dest_path).await
        })
    }

    fn unlink_file_at(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        path: String,
    ) -> FsResult<()> {
        in_tokio(async { async_filesystem::HostDescriptor::unlink_file_at(self, fd, path).await })
    }

    fn read_via_stream(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        offset: sync_filesystem::Filesize,
    ) -> FsResult<Resource<streams::InputStream>> {
        Ok(async_filesystem::HostDescriptor::read_via_stream(
            self, fd, offset,
        )?)
    }

    fn write_via_stream(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        offset: sync_filesystem::Filesize,
    ) -> FsResult<Resource<streams::OutputStream>> {
        Ok(async_filesystem::HostDescriptor::write_via_stream(
            self, fd, offset,
        )?)
    }

    fn append_via_stream(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
    ) -> FsResult<Resource<streams::OutputStream>> {
        Ok(async_filesystem::HostDescriptor::append_via_stream(
            self, fd,
        )?)
    }

    fn is_same_object(
        &mut self,
        a: Resource<sync_filesystem::Descriptor>,
        b: Resource<sync_filesystem::Descriptor>,
    ) -> wasmtime::Result<bool> {
        in_tokio(async { async_filesystem::HostDescriptor::is_same_object(self, a, b).await })
    }
    fn metadata_hash(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
    ) -> FsResult<sync_filesystem::MetadataHashValue> {
        Ok(
            in_tokio(async { async_filesystem::HostDescriptor::metadata_hash(self, fd).await })?
                .into(),
        )
    }
    fn metadata_hash_at(
        &mut self,
        fd: Resource<sync_filesystem::Descriptor>,
        path_flags: sync_filesystem::PathFlags,
        path: String,
    ) -> FsResult<sync_filesystem::MetadataHashValue> {
        Ok(in_tokio(async {
            async_filesystem::HostDescriptor::metadata_hash_at(self, fd, path_flags.into(), path)
                .await
        })?
        .into())
    }
}

impl sync_filesystem::HostDirectoryEntryStream for WasiFilesystemCtxView<'_> {
    fn read_directory_entry(
        &mut self,
        stream: Resource<sync_filesystem::DirectoryEntryStream>,
    ) -> FsResult<Option<sync_filesystem::DirectoryEntry>> {
        Ok(in_tokio(async {
            async_filesystem::HostDirectoryEntryStream::read_directory_entry(self, stream).await
        })?
        .map(|e| e.into()))
    }

    fn drop(
        &mut self,
        stream: Resource<sync_filesystem::DirectoryEntryStream>,
    ) -> wasmtime::Result<()> {
        async_filesystem::HostDirectoryEntryStream::drop(self, stream)
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
            type_: other.type_.into(),
            name: other.name,
        }
    }
}

impl From<async_filesystem::DescriptorStat> for sync_filesystem::DescriptorStat {
    fn from(other: async_filesystem::DescriptorStat) -> Self {
        Self {
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
impl From<async_filesystem::MetadataHashValue> for sync_filesystem::MetadataHashValue {
    fn from(other: async_filesystem::MetadataHashValue) -> Self {
        Self {
            lower: other.lower,
            upper: other.upper,
        }
    }
}

mod named {
    use super::*;
    use crate::filesystem::WasiFilesystemNamedView;
    use crate::p2::bindings::sync::named_imports::wasi::filesystem::types as named_types;
    use crate::{NamedId, WasiCtxNamedView};

    impl<T> named_types::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiFilesystemNamedView,
    {
        fn convert_error_code(
            &mut self,
            id: NamedId,
            err: FsError,
        ) -> wasmtime::Result<sync_filesystem::ErrorCode> {
            sync_filesystem::Host::convert_error_code(&mut self.0.filesystem(id), err)
        }

        fn filesystem_error_code(
            &mut self,
            id: NamedId,
            err: Resource<streams::Error>,
        ) -> wasmtime::Result<Option<sync_filesystem::ErrorCode>> {
            sync_filesystem::Host::filesystem_error_code(&mut self.0.filesystem(id), err)
        }
    }

    impl<T> named_types::HostDescriptor for WasiCtxNamedView<'_, T>
    where
        T: WasiFilesystemNamedView,
    {
        fn advise(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            offset: sync_filesystem::Filesize,
            len: sync_filesystem::Filesize,
            advice: sync_filesystem::Advice,
        ) -> FsResult<()> {
            sync_filesystem::HostDescriptor::advise(
                &mut self.0.filesystem(id),
                fd,
                offset,
                len,
                advice,
            )
        }

        fn sync_data(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
        ) -> FsResult<()> {
            sync_filesystem::HostDescriptor::sync_data(&mut self.0.filesystem(id), fd)
        }

        fn get_flags(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
        ) -> FsResult<sync_filesystem::DescriptorFlags> {
            sync_filesystem::HostDescriptor::get_flags(&mut self.0.filesystem(id), fd)
        }

        fn get_type(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
        ) -> FsResult<sync_filesystem::DescriptorType> {
            sync_filesystem::HostDescriptor::get_type(&mut self.0.filesystem(id), fd)
        }

        fn set_size(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            size: sync_filesystem::Filesize,
        ) -> FsResult<()> {
            sync_filesystem::HostDescriptor::set_size(&mut self.0.filesystem(id), fd, size)
        }

        fn set_times(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            atim: sync_filesystem::NewTimestamp,
            mtim: sync_filesystem::NewTimestamp,
        ) -> FsResult<()> {
            sync_filesystem::HostDescriptor::set_times(&mut self.0.filesystem(id), fd, atim, mtim)
        }

        fn read(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            len: sync_filesystem::Filesize,
            offset: sync_filesystem::Filesize,
        ) -> FsResult<(Vec<u8>, bool)> {
            sync_filesystem::HostDescriptor::read(&mut self.0.filesystem(id), fd, len, offset)
        }

        fn write(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            buf: Vec<u8>,
            offset: sync_filesystem::Filesize,
        ) -> FsResult<sync_filesystem::Filesize> {
            sync_filesystem::HostDescriptor::write(&mut self.0.filesystem(id), fd, buf, offset)
        }

        fn read_directory(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
        ) -> FsResult<Resource<sync_filesystem::DirectoryEntryStream>> {
            sync_filesystem::HostDescriptor::read_directory(&mut self.0.filesystem(id), fd)
        }

        fn sync(&mut self, id: NamedId, fd: Resource<sync_filesystem::Descriptor>) -> FsResult<()> {
            sync_filesystem::HostDescriptor::sync(&mut self.0.filesystem(id), fd)
        }

        fn create_directory_at(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            path: String,
        ) -> FsResult<()> {
            sync_filesystem::HostDescriptor::create_directory_at(
                &mut self.0.filesystem(id),
                fd,
                path,
            )
        }

        fn stat(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
        ) -> FsResult<sync_filesystem::DescriptorStat> {
            sync_filesystem::HostDescriptor::stat(&mut self.0.filesystem(id), fd)
        }

        fn stat_at(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            path_flags: sync_filesystem::PathFlags,
            path: String,
        ) -> FsResult<sync_filesystem::DescriptorStat> {
            sync_filesystem::HostDescriptor::stat_at(
                &mut self.0.filesystem(id),
                fd,
                path_flags,
                path,
            )
        }

        fn set_times_at(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            path_flags: sync_filesystem::PathFlags,
            path: String,
            atim: sync_filesystem::NewTimestamp,
            mtim: sync_filesystem::NewTimestamp,
        ) -> FsResult<()> {
            sync_filesystem::HostDescriptor::set_times_at(
                &mut self.0.filesystem(id),
                fd,
                path_flags,
                path,
                atim,
                mtim,
            )
        }

        fn link_at(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>, // TODO delete the path flags from this function
            old_path_flags: sync_filesystem::PathFlags,
            old_path: String,
            new_descriptor: Resource<sync_filesystem::Descriptor>,
            new_path: String,
        ) -> FsResult<()> {
            sync_filesystem::HostDescriptor::link_at(
                &mut self.0.filesystem(id),
                fd, // TODO delete the path flags from this function
                old_path_flags,
                old_path,
                new_descriptor,
                new_path,
            )
        }

        fn open_at(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            path_flags: sync_filesystem::PathFlags,
            path: String,
            oflags: sync_filesystem::OpenFlags,
            flags: sync_filesystem::DescriptorFlags,
        ) -> FsResult<Resource<sync_filesystem::Descriptor>> {
            sync_filesystem::HostDescriptor::open_at(
                &mut self.0.filesystem(id),
                fd,
                path_flags,
                path,
                oflags,
                flags,
            )
        }

        fn drop(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
        ) -> wasmtime::Result<()> {
            sync_filesystem::HostDescriptor::drop(&mut self.0.filesystem(id), fd)
        }

        fn readlink_at(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            path: String,
        ) -> FsResult<String> {
            sync_filesystem::HostDescriptor::readlink_at(&mut self.0.filesystem(id), fd, path)
        }

        fn remove_directory_at(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            path: String,
        ) -> FsResult<()> {
            sync_filesystem::HostDescriptor::remove_directory_at(
                &mut self.0.filesystem(id),
                fd,
                path,
            )
        }

        fn rename_at(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            old_path: String,
            new_fd: Resource<sync_filesystem::Descriptor>,
            new_path: String,
        ) -> FsResult<()> {
            sync_filesystem::HostDescriptor::rename_at(
                &mut self.0.filesystem(id),
                fd,
                old_path,
                new_fd,
                new_path,
            )
        }

        fn symlink_at(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            src_path: String,
            dest_path: String,
        ) -> FsResult<()> {
            sync_filesystem::HostDescriptor::symlink_at(
                &mut self.0.filesystem(id),
                fd,
                src_path,
                dest_path,
            )
        }

        fn unlink_file_at(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            path: String,
        ) -> FsResult<()> {
            sync_filesystem::HostDescriptor::unlink_file_at(&mut self.0.filesystem(id), fd, path)
        }

        fn read_via_stream(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            offset: sync_filesystem::Filesize,
        ) -> FsResult<Resource<streams::InputStream>> {
            sync_filesystem::HostDescriptor::read_via_stream(&mut self.0.filesystem(id), fd, offset)
        }

        fn write_via_stream(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            offset: sync_filesystem::Filesize,
        ) -> FsResult<Resource<streams::OutputStream>> {
            sync_filesystem::HostDescriptor::write_via_stream(
                &mut self.0.filesystem(id),
                fd,
                offset,
            )
        }

        fn append_via_stream(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
        ) -> FsResult<Resource<streams::OutputStream>> {
            sync_filesystem::HostDescriptor::append_via_stream(&mut self.0.filesystem(id), fd)
        }

        fn is_same_object(
            &mut self,
            id: NamedId,
            a: Resource<sync_filesystem::Descriptor>,
            b: Resource<sync_filesystem::Descriptor>,
        ) -> wasmtime::Result<bool> {
            sync_filesystem::HostDescriptor::is_same_object(&mut self.0.filesystem(id), a, b)
        }

        fn metadata_hash(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
        ) -> FsResult<sync_filesystem::MetadataHashValue> {
            sync_filesystem::HostDescriptor::metadata_hash(&mut self.0.filesystem(id), fd)
        }

        fn metadata_hash_at(
            &mut self,
            id: NamedId,
            fd: Resource<sync_filesystem::Descriptor>,
            path_flags: sync_filesystem::PathFlags,
            path: String,
        ) -> FsResult<sync_filesystem::MetadataHashValue> {
            sync_filesystem::HostDescriptor::metadata_hash_at(
                &mut self.0.filesystem(id),
                fd,
                path_flags,
                path,
            )
        }
    }

    impl<T> named_types::HostDirectoryEntryStream for WasiCtxNamedView<'_, T>
    where
        T: WasiFilesystemNamedView,
    {
        fn read_directory_entry(
            &mut self,
            id: NamedId,
            stream: Resource<sync_filesystem::DirectoryEntryStream>,
        ) -> FsResult<Option<sync_filesystem::DirectoryEntry>> {
            sync_filesystem::HostDirectoryEntryStream::read_directory_entry(
                &mut self.0.filesystem(id),
                stream,
            )
        }

        fn drop(
            &mut self,
            id: NamedId,
            stream: Resource<sync_filesystem::DirectoryEntryStream>,
        ) -> wasmtime::Result<()> {
            sync_filesystem::HostDirectoryEntryStream::drop(&mut self.0.filesystem(id), stream)
        }
    }
}

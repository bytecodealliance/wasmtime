use crate::filesystem::{WasiFilesystem, WasiFilesystemCtxView};
use crate::p3::bindings::filesystem::types::{
    Advice, Descriptor, DescriptorFlags, DescriptorStat, DescriptorType, DirectoryEntry, ErrorCode,
    Filesize, MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
};
use crate::p3::bindings::filesystem::{preopens, types};
use anyhow::{Context as _, bail};
use wasmtime::component::{Accessor, FutureReader, Resource, StreamReader};

impl types::Host for WasiFilesystemCtxView<'_> {}

impl types::HostDescriptorWithStore for WasiFilesystem {
    async fn read_via_stream<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        mut offset: Filesize,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
        bail!("TODO")
    }

    async fn write_via_stream<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        data: StreamReader<u8>,
        mut offset: Filesize,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn append_via_stream<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        data: StreamReader<u8>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn advise<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        offset: Filesize,
        length: Filesize,
        advice: Advice,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn sync_data<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn get_flags<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<DescriptorFlags, ErrorCode>> {
        bail!("TODO")
    }

    async fn get_type<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<DescriptorType, ErrorCode>> {
        bail!("TODO")
    }

    async fn set_size<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        size: Filesize,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn set_times<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        data_access_timestamp: NewTimestamp,
        data_modification_timestamp: NewTimestamp,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn read_directory<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> wasmtime::Result<(
        StreamReader<DirectoryEntry>,
        FutureReader<Result<(), ErrorCode>>,
    )> {
        bail!("TODO")
    }

    async fn sync<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn create_directory_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn stat<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<DescriptorStat, ErrorCode>> {
        bail!("TODO")
    }

    async fn stat_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
    ) -> wasmtime::Result<Result<DescriptorStat, ErrorCode>> {
        bail!("TODO")
    }

    async fn set_times_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
        data_access_timestamp: NewTimestamp,
        data_modification_timestamp: NewTimestamp,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn link_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        old_path_flags: PathFlags,
        old_path: String,
        new_fd: Resource<Descriptor>,
        new_path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn open_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
        open_flags: OpenFlags,
        flags: DescriptorFlags,
    ) -> wasmtime::Result<Result<Resource<Descriptor>, ErrorCode>> {
        bail!("TODO")
    }

    async fn readlink_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path: String,
    ) -> wasmtime::Result<Result<String, ErrorCode>> {
        bail!("TODO")
    }

    async fn remove_directory_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn rename_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        old_path: String,
        new_fd: Resource<Descriptor>,
        new_path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn symlink_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        old_path: String,
        new_path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn unlink_file_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        bail!("TODO")
    }

    async fn is_same_object<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        other: Resource<Descriptor>,
    ) -> wasmtime::Result<bool> {
        bail!("TODO")
    }

    async fn metadata_hash<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<MetadataHashValue, ErrorCode>> {
        bail!("TODO")
    }

    async fn metadata_hash_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
    ) -> wasmtime::Result<Result<MetadataHashValue, ErrorCode>> {
        bail!("TODO")
    }
}

impl types::HostDescriptor for WasiFilesystemCtxView<'_> {
    fn drop(&mut self, fd: Resource<Descriptor>) -> wasmtime::Result<()> {
        self.table
            .delete(fd)
            .context("failed to delete descriptor resource from table")?;
        Ok(())
    }
}

impl preopens::Host for WasiFilesystemCtxView<'_> {
    fn get_directories(&mut self) -> wasmtime::Result<Vec<(Resource<Descriptor>, String)>> {
        bail!("TODO")
    }
}

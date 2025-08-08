use crate::filesystem::{Descriptor, Dir, File, WasiFilesystem, WasiFilesystemCtxView};
use crate::p3::DEFAULT_BUFFER_CAPACITY;
use crate::p3::bindings::filesystem::types::{
    self, Advice, DescriptorFlags, DescriptorStat, DescriptorType, DirectoryEntry, ErrorCode,
    Filesize, MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
};
use crate::p3::filesystem::{FilesystemError, FilesystemResult, preopens};
use crate::{FilePerms, TrappableError};
use anyhow::{Context as _, bail};
use system_interface::fs::FileIoExt as _;
use wasmtime::component::{Accessor, FutureReader, Resource, ResourceTable, StreamReader};

fn get_descriptor<'a>(
    table: &'a ResourceTable,
    fd: &'a Resource<Descriptor>,
) -> FilesystemResult<&'a Descriptor> {
    table
        .get(fd)
        .context("failed to get descriptor resource from table")
        .map_err(TrappableError::trap)
}

trait AccessorExt {
    fn get_descriptor(&self, fd: &Resource<Descriptor>) -> FilesystemResult<Descriptor>;
    fn get_file(&self, fd: &Resource<Descriptor>) -> FilesystemResult<File>;
    fn get_dir(&self, fd: &Resource<Descriptor>) -> FilesystemResult<Dir>;
    fn get_dir_pair(
        &self,
        a: &Resource<Descriptor>,
        b: &Resource<Descriptor>,
    ) -> FilesystemResult<(Dir, Dir)>;
}

impl<T> AccessorExt for Accessor<T, WasiFilesystem> {
    fn get_descriptor(&self, fd: &Resource<Descriptor>) -> FilesystemResult<Descriptor> {
        self.with(|mut view| {
            let fd = get_descriptor(view.get().table, fd)?;
            Ok(fd.clone())
        })
    }

    fn get_file(&self, fd: &Resource<Descriptor>) -> FilesystemResult<File> {
        self.with(|mut view| {
            let file = get_descriptor(view.get().table, fd).map(Descriptor::file)??;
            Ok(file.clone())
        })
    }

    fn get_dir(&self, fd: &Resource<Descriptor>) -> FilesystemResult<Dir> {
        self.with(|mut view| {
            let dir = get_descriptor(view.get().table, fd).map(Descriptor::dir)??;
            Ok(dir.clone())
        })
    }

    fn get_dir_pair(
        &self,
        a: &Resource<Descriptor>,
        b: &Resource<Descriptor>,
    ) -> FilesystemResult<(Dir, Dir)> {
        self.with(|mut view| {
            let table = view.get().table;
            let a = get_descriptor(table, a).map(Descriptor::dir)??;
            let b = get_descriptor(table, b).map(Descriptor::dir)??;
            Ok((a.clone(), b.clone()))
        })
    }
}

impl types::Host for WasiFilesystemCtxView<'_> {
    fn convert_error_code(&mut self, error: FilesystemError) -> wasmtime::Result<ErrorCode> {
        error.downcast()
    }
}

impl types::HostDescriptorWithStore for WasiFilesystem {
    #[expect(unused)]
    async fn read_via_stream<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        offset: Filesize,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
        let file = store.get_file(&fd)?;
        if !file.perms.contains(FilePerms::READ) {
            return Err(types::ErrorCode::NotPermitted.into());
        }
        bail!("TODO: read_via_stream")
    }

    async fn write_via_stream<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        mut data: StreamReader<u8>,
        mut offset: Filesize,
    ) -> FilesystemResult<()> {
        let file = store.get_file(&fd)?;
        if !file.perms.contains(FilePerms::WRITE) {
            return Err(types::ErrorCode::NotPermitted.into());
        }
        let mut buf = Vec::with_capacity(DEFAULT_BUFFER_CAPACITY);
        while !data.is_closed() {
            buf = data.read(store, buf).await;
            buf = file
                .spawn_blocking(move |file| {
                    let mut pos = 0;
                    while pos != buf.len() {
                        let n = file.write_at(&buf[pos..], offset)?;
                        pos = pos.saturating_add(n);
                        let n = n.try_into().or(Err(ErrorCode::Overflow))?;
                        offset = offset.checked_add(n).ok_or(ErrorCode::Overflow)?;
                    }
                    FilesystemResult::Ok(buf)
                })
                .await?;
            buf.clear();
        }
        Ok(())
    }

    async fn append_via_stream<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        mut data: StreamReader<u8>,
    ) -> FilesystemResult<()> {
        let file = store.get_file(&fd)?;
        if !file.perms.contains(FilePerms::WRITE) {
            return Err(types::ErrorCode::NotPermitted.into());
        }
        let mut buf = Vec::with_capacity(DEFAULT_BUFFER_CAPACITY);
        while !data.is_closed() {
            buf = data.read(store, buf).await;
            buf = file
                .spawn_blocking(move |file| {
                    let mut pos = 0;
                    while pos != buf.len() {
                        let n = file.append(&buf[pos..])?;
                        pos = pos.saturating_add(n);
                    }
                    FilesystemResult::Ok(buf)
                })
                .await?;
            buf.clear();
        }
        Ok(())
    }

    async fn advise<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        offset: Filesize,
        length: Filesize,
        advice: Advice,
    ) -> FilesystemResult<()> {
        let file = store.get_file(&fd)?;
        file.advise(offset, length, advice.into()).await?;
        Ok(())
    }

    async fn sync_data<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> FilesystemResult<()> {
        let fd = store.get_descriptor(&fd)?;
        fd.sync_data().await?;
        Ok(())
    }

    async fn get_flags<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> FilesystemResult<DescriptorFlags> {
        let fd = store.get_descriptor(&fd)?;
        let flags = fd.get_flags().await?;
        Ok(flags.into())
    }

    async fn get_type<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> FilesystemResult<DescriptorType> {
        let fd = store.get_descriptor(&fd)?;
        let ty = fd.get_type().await?;
        Ok(ty.into())
    }

    async fn set_size<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        size: Filesize,
    ) -> FilesystemResult<()> {
        let file = store.get_file(&fd)?;
        file.set_size(size).await?;
        Ok(())
    }

    async fn set_times<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        data_access_timestamp: NewTimestamp,
        data_modification_timestamp: NewTimestamp,
    ) -> FilesystemResult<()> {
        let fd = store.get_descriptor(&fd)?;
        fd.set_times(
            data_access_timestamp.into(),
            data_modification_timestamp.into(),
        )
        .await?;
        Ok(())
    }

    #[expect(unused)]
    async fn read_directory<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> wasmtime::Result<(
        StreamReader<DirectoryEntry>,
        FutureReader<Result<(), ErrorCode>>,
    )> {
        let dir = store.get_dir(&fd)?;
        bail!("TODO: read_directory")
    }

    async fn sync<U>(store: &Accessor<U, Self>, fd: Resource<Descriptor>) -> FilesystemResult<()> {
        let fd = store.get_descriptor(&fd)?;
        fd.sync().await?;
        Ok(())
    }

    async fn create_directory_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path: String,
    ) -> FilesystemResult<()> {
        let dir = store.get_dir(&fd)?;
        dir.create_directory_at(path).await?;
        Ok(())
    }

    async fn stat<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> FilesystemResult<DescriptorStat> {
        let fd = store.get_descriptor(&fd)?;
        let stat = fd.stat().await?;
        Ok(stat.into())
    }

    async fn stat_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
    ) -> FilesystemResult<DescriptorStat> {
        let dir = store.get_dir(&fd)?;
        let stat = dir.stat_at(path_flags.into(), path).await?;
        Ok(stat.into())
    }

    async fn set_times_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
        data_access_timestamp: NewTimestamp,
        data_modification_timestamp: NewTimestamp,
    ) -> FilesystemResult<()> {
        let dir = store.get_dir(&fd)?;
        dir.set_times_at(
            path_flags.into(),
            path,
            data_access_timestamp.into(),
            data_modification_timestamp.into(),
        )
        .await?;
        Ok(())
    }

    async fn link_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        old_path_flags: PathFlags,
        old_path: String,
        new_fd: Resource<Descriptor>,
        new_path: String,
    ) -> FilesystemResult<()> {
        let (old_dir, new_dir) = store.get_dir_pair(&fd, &new_fd)?;
        old_dir
            .link_at(old_path_flags.into(), old_path, &new_dir, new_path)
            .await?;
        Ok(())
    }

    async fn open_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
        open_flags: OpenFlags,
        flags: DescriptorFlags,
    ) -> FilesystemResult<Resource<Descriptor>> {
        let (allow_blocking_current_thread, dir) = store.with(|mut view| {
            let view = view.get();
            let fd = get_descriptor(&view.table, &fd)?;
            let dir = fd.dir()?;
            FilesystemResult::Ok((view.ctx.allow_blocking_current_thread, dir.clone()))
        })?;
        let fd = dir
            .open_at(
                path_flags.into(),
                path,
                open_flags.into(),
                flags.into(),
                allow_blocking_current_thread,
            )
            .await?;
        let fd = store.with(|mut view| view.get().table.push(fd))?;
        Ok(fd)
    }

    async fn readlink_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path: String,
    ) -> FilesystemResult<String> {
        let dir = store.get_dir(&fd)?;
        let path = dir.readlink_at(path).await?;
        Ok(path)
    }

    async fn remove_directory_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path: String,
    ) -> FilesystemResult<()> {
        let dir = store.get_dir(&fd)?;
        dir.remove_directory_at(path).await?;
        Ok(())
    }

    async fn rename_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        old_path: String,
        new_fd: Resource<Descriptor>,
        new_path: String,
    ) -> FilesystemResult<()> {
        let (old_dir, new_dir) = store.get_dir_pair(&fd, &new_fd)?;
        old_dir.rename_at(old_path, &new_dir, new_path).await?;
        Ok(())
    }

    async fn symlink_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        old_path: String,
        new_path: String,
    ) -> FilesystemResult<()> {
        let dir = store.get_dir(&fd)?;
        dir.symlink_at(old_path, new_path).await?;
        Ok(())
    }

    async fn unlink_file_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path: String,
    ) -> FilesystemResult<()> {
        let dir = store.get_dir(&fd)?;
        dir.unlink_file_at(path).await?;
        Ok(())
    }

    async fn is_same_object<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        other: Resource<Descriptor>,
    ) -> wasmtime::Result<bool> {
        let (fd, other) = store.with(|mut view| {
            let table = view.get().table;
            let fd = get_descriptor(table, &fd).map(|fd| fd.clone())?;
            let other = get_descriptor(table, &other).map(|other| other.clone())?;
            anyhow::Ok((fd, other))
        })?;
        fd.is_same_object(&other).await
    }

    async fn metadata_hash<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> FilesystemResult<MetadataHashValue> {
        let fd = store.get_descriptor(&fd)?;
        let meta = fd.metadata_hash().await?;
        Ok(meta.into())
    }

    async fn metadata_hash_at<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
    ) -> FilesystemResult<MetadataHashValue> {
        let dir = store.get_dir(&fd)?;
        let meta = dir.metadata_hash_at(path_flags.into(), path).await?;
        Ok(meta.into())
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
        self.get_directories()
    }
}

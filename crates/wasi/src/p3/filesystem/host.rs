use crate::DirPerms;
use crate::filesystem::{Descriptor, Dir, File, WasiFilesystem, WasiFilesystemCtxView};
use crate::p3::DEFAULT_BUFFER_CAPACITY;
use crate::p3::bindings::clocks::wall_clock;
use crate::p3::bindings::filesystem::types::{
    self, Advice, DescriptorFlags, DescriptorStat, DescriptorType, DirectoryEntry, ErrorCode,
    Filesize, MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
};
use crate::p3::filesystem::{FilesystemError, FilesystemResult, preopens};
use crate::{FilePerms, TrappableError};
use anyhow::Context as _;
use bytes::BytesMut;
use std::io::Cursor;
use system_interface::fs::FileIoExt as _;
use wasmtime::component::{
    Accessor, AccessorTask, FutureReader, FutureWriter, GuardedFutureWriter, GuardedStreamWriter,
    Resource, ResourceTable, StreamReader, StreamWriter,
};

fn get_descriptor<'a>(
    table: &'a ResourceTable,
    fd: &'a Resource<Descriptor>,
) -> FilesystemResult<&'a Descriptor> {
    table
        .get(fd)
        .context("failed to get descriptor resource from table")
        .map_err(TrappableError::trap)
}

fn get_file<'a>(
    table: &'a ResourceTable,
    fd: &'a Resource<Descriptor>,
) -> FilesystemResult<&'a File> {
    let file = get_descriptor(table, fd).map(Descriptor::file)??;
    Ok(file)
}

fn get_dir<'a>(
    table: &'a ResourceTable,
    fd: &'a Resource<Descriptor>,
) -> FilesystemResult<&'a Dir> {
    let dir = get_descriptor(table, fd).map(Descriptor::dir)??;
    Ok(dir)
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
        self.with(|mut store| {
            let fd = get_descriptor(store.get().table, fd)?;
            Ok(fd.clone())
        })
    }

    fn get_file(&self, fd: &Resource<Descriptor>) -> FilesystemResult<File> {
        self.with(|mut store| {
            let file = get_file(store.get().table, fd)?;
            Ok(file.clone())
        })
    }

    fn get_dir(&self, fd: &Resource<Descriptor>) -> FilesystemResult<Dir> {
        self.with(|mut store| {
            let dir = get_dir(store.get().table, fd)?;
            Ok(dir.clone())
        })
    }

    fn get_dir_pair(
        &self,
        a: &Resource<Descriptor>,
        b: &Resource<Descriptor>,
    ) -> FilesystemResult<(Dir, Dir)> {
        self.with(|mut store| {
            let table = store.get().table;
            let a = get_dir(table, a)?;
            let b = get_dir(table, b)?;
            Ok((a.clone(), b.clone()))
        })
    }
}

fn systemtime_from(t: wall_clock::Datetime) -> Result<std::time::SystemTime, ErrorCode> {
    std::time::SystemTime::UNIX_EPOCH
        .checked_add(core::time::Duration::new(t.seconds, t.nanoseconds))
        .ok_or(ErrorCode::Overflow)
}

fn systemtimespec_from(t: NewTimestamp) -> Result<Option<fs_set_times::SystemTimeSpec>, ErrorCode> {
    use fs_set_times::SystemTimeSpec;
    match t {
        NewTimestamp::NoChange => Ok(None),
        NewTimestamp::Now => Ok(Some(SystemTimeSpec::SymbolicNow)),
        NewTimestamp::Timestamp(st) => {
            let st = systemtime_from(st)?;
            Ok(Some(SystemTimeSpec::Absolute(st)))
        }
    }
}

struct ReadFileTask {
    file: File,
    offset: u64,
    data_tx: StreamWriter<u8>,
    result_tx: FutureWriter<Result<(), ErrorCode>>,
}

impl<T> AccessorTask<T, WasiFilesystem, wasmtime::Result<()>> for ReadFileTask {
    async fn run(self, store: &Accessor<T, WasiFilesystem>) -> wasmtime::Result<()> {
        let mut buf = BytesMut::zeroed(DEFAULT_BUFFER_CAPACITY);
        let mut offset = self.offset;
        let mut data_tx = GuardedStreamWriter::new(store, self.data_tx);
        let result_tx = GuardedFutureWriter::new(store, self.result_tx);
        let res = loop {
            match self
                .file
                .run_blocking(move |file| {
                    let n = file.read_at(&mut buf, offset)?;
                    buf.truncate(n);
                    std::io::Result::Ok(buf)
                })
                .await
            {
                Ok(chunk) if chunk.is_empty() => {
                    break Ok(());
                }
                Ok(chunk) => {
                    let Ok(n) = chunk.len().try_into() else {
                        break Err(ErrorCode::Overflow);
                    };
                    let Some(n) = offset.checked_add(n) else {
                        break Err(ErrorCode::Overflow);
                    };
                    offset = n;
                    buf = data_tx.write_all(Cursor::new(chunk)).await.into_inner();
                    if data_tx.is_closed() {
                        break Ok(());
                    }
                    buf.resize(DEFAULT_BUFFER_CAPACITY, 0);
                }
                Err(err) => {
                    break Err(err.into());
                }
            }
        };
        drop(self.file);
        drop(data_tx);
        result_tx.write(res).await;
        Ok(())
    }
}

struct ReadDirectoryTask {
    dir: Dir,
    data_tx: StreamWriter<DirectoryEntry>,
    result_tx: FutureWriter<Result<(), ErrorCode>>,
}

impl<T> AccessorTask<T, WasiFilesystem, wasmtime::Result<()>> for ReadDirectoryTask {
    async fn run(self, store: &Accessor<T, WasiFilesystem>) -> wasmtime::Result<()> {
        let mut data_tx = GuardedStreamWriter::new(store, self.data_tx);
        let result_tx = GuardedFutureWriter::new(store, self.result_tx);
        let res = loop {
            let mut entries = match self.dir.run_blocking(cap_std::fs::Dir::entries).await {
                Ok(entries) => entries,
                Err(err) => break Err(err.into()),
            };
            if let Err(err) = loop {
                let Some((res, tail)) = self
                    .dir
                    .run_blocking(move |_| entries.next().map(|entry| (entry, entries)))
                    .await
                else {
                    break Ok(());
                };
                entries = tail;
                let entry = match res {
                    Ok(entry) => entry,
                    Err(err) => {
                        // On windows, filter out files like `C:\DumpStack.log.tmp` which we
                        // can't get full metadata for.
                        #[cfg(windows)]
                        {
                            use windows_sys::Win32::Foundation::{
                                ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION,
                            };
                            if err.raw_os_error() == Some(ERROR_SHARING_VIOLATION as i32)
                                || err.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32)
                            {
                                continue;
                            }
                        }
                        break Err(err.into());
                    }
                };
                let meta = match entry.metadata() {
                    Ok(meta) => meta,
                    Err(err) => break Err(err.into()),
                };
                let Ok(name) = entry.file_name().into_string() else {
                    break Err(ErrorCode::IllegalByteSequence);
                };
                data_tx
                    .write(Some(DirectoryEntry {
                        type_: meta.file_type().into(),
                        name,
                    }))
                    .await;
                if data_tx.is_closed() {
                    break Ok(());
                }
            } {
                break Err(err);
            };
        };
        drop(self.dir);
        drop(data_tx);
        result_tx.write(res).await;
        Ok(())
    }
}

impl types::Host for WasiFilesystemCtxView<'_> {
    fn convert_error_code(&mut self, error: FilesystemError) -> wasmtime::Result<ErrorCode> {
        error.downcast()
    }
}

impl types::HostDescriptorWithStore for WasiFilesystem {
    async fn read_via_stream<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        offset: Filesize,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
        let (file, (data_tx, data_rx), (result_tx, result_rx)) = store.with(|mut store| {
            let file = get_file(store.get().table, &fd).cloned()?;
            let instance = store.instance();
            let data = instance
                .stream(&mut store)
                .context("failed to create stream")?;
            let result = if !file.perms.contains(FilePerms::READ) {
                instance.future(&mut store, || Err(types::ErrorCode::NotPermitted))
            } else {
                instance.future(&mut store, || unreachable!())
            }
            .context("failed to create future")?;
            anyhow::Ok((file, data, result))
        })?;
        if !file.perms.contains(FilePerms::READ) {
            return Ok((data_rx, result_rx));
        }
        store.spawn(ReadFileTask {
            file,
            offset,
            data_tx,
            result_tx,
        });
        Ok((data_rx, result_rx))
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
            offset = offset.saturating_add(buf.len() as _);
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
        let atim = systemtimespec_from(data_access_timestamp)?;
        let mtim = systemtimespec_from(data_modification_timestamp)?;
        fd.set_times(atim, mtim).await?;
        Ok(())
    }

    async fn read_directory<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> wasmtime::Result<(
        StreamReader<DirectoryEntry>,
        FutureReader<Result<(), ErrorCode>>,
    )> {
        let (dir, (data_tx, data_rx), (result_tx, result_rx)) = store.with(|mut store| {
            let dir = get_dir(store.get().table, &fd).cloned()?;
            let instance = store.instance();
            let data = instance
                .stream(&mut store)
                .context("failed to create stream")?;
            let result = if !dir.perms.contains(DirPerms::READ) {
                instance.future(&mut store, || Err(types::ErrorCode::NotPermitted))
            } else {
                instance.future(&mut store, || unreachable!())
            }
            .context("failed to create future")?;
            anyhow::Ok((dir, data, result))
        })?;
        if !dir.perms.contains(DirPerms::READ) {
            return Ok((data_rx, result_rx));
        }
        store.spawn(ReadDirectoryTask {
            dir,
            data_tx,
            result_tx,
        });
        Ok((data_rx, result_rx))
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
        let atim = systemtimespec_from(data_access_timestamp)?;
        let mtim = systemtimespec_from(data_modification_timestamp)?;
        dir.set_times_at(path_flags.into(), path, atim, mtim)
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
        let (allow_blocking_current_thread, dir) = store.with(|mut store| {
            let store = store.get();
            let dir = get_dir(&store.table, &fd)?;
            FilesystemResult::Ok((store.ctx.allow_blocking_current_thread, dir.clone()))
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
        let fd = store.with(|mut store| store.get().table.push(fd))?;
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
        let (fd, other) = store.with(|mut store| {
            let table = store.get().table;
            let fd = get_descriptor(table, &fd)?.clone();
            let other = get_descriptor(table, &other)?.clone();
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

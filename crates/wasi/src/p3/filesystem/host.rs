use crate::filesystem::{Descriptor, Dir, File, WasiFilesystem, WasiFilesystemCtxView};
use crate::p3::bindings::clocks::wall_clock;
use crate::p3::bindings::filesystem::types::{
    self, Advice, DescriptorFlags, DescriptorStat, DescriptorType, DirectoryEntry, ErrorCode,
    Filesize, MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
};
use crate::p3::filesystem::{FilesystemError, FilesystemResult, preopens};
use crate::p3::{
    DEFAULT_BUFFER_CAPACITY, FutureOneshotProducer, FutureReadyProducer, MAX_BUFFER_CAPACITY,
    StreamEmptyProducer, write_buffered_bytes,
};
use crate::{DirPerms, FilePerms};
use anyhow::{Context as _, bail};
use bytes::BytesMut;
use core::mem;
use std::io::Cursor;
use system_interface::fs::FileIoExt as _;
use tokio::sync::oneshot;
use wasmtime::component::{
    Accessor, Destination, FutureReader, Resource, ResourceTable, Source, StreamConsumer,
    StreamProducer, StreamReader, StreamState,
};

fn get_descriptor<'a>(
    table: &'a ResourceTable,
    fd: &'a Resource<Descriptor>,
) -> FilesystemResult<&'a Descriptor> {
    table
        .get(fd)
        .context("failed to get descriptor resource from table")
        .map_err(FilesystemError::trap)
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

struct ReadStreamProducer {
    file: File,
    offset: u64,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
    buffer: Cursor<BytesMut>,
}

impl Drop for ReadStreamProducer {
    fn drop(&mut self) {
        self.close(Ok(()))
    }
}

impl ReadStreamProducer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let Some(tx) = self.result.take() {
            _ = tx.send(res);
        }
    }

    async fn read(&mut self, n: usize) -> StreamState {
        let mut buf = mem::take(&mut self.buffer).into_inner();
        buf.resize(n, 0);
        let offset = self.offset;
        let res = 'result: {
            match self
                .file
                .run_blocking(move |file| {
                    let n = file.read_at(&mut buf, offset)?;
                    buf.truncate(n);
                    std::io::Result::Ok(buf)
                })
                .await
            {
                Ok(buf) if buf.is_empty() => break 'result Ok(()),
                Ok(buf) => {
                    let Ok(n) = buf.len().try_into() else {
                        break 'result Err(ErrorCode::Overflow);
                    };
                    let Some(n) = offset.checked_add(n) else {
                        break 'result Err(ErrorCode::Overflow);
                    };
                    self.offset = n;
                    self.buffer = Cursor::new(buf);
                    return StreamState::Open;
                }
                Err(err) => break 'result Err(err.into()),
            }
        };
        self.close(res);
        StreamState::Closed
    }
}

impl<D> StreamProducer<D, u8> for ReadStreamProducer {
    async fn produce(
        &mut self,
        store: &Accessor<D>,
        dst: &mut Destination<u8>,
    ) -> wasmtime::Result<StreamState> {
        if !self.buffer.get_ref().is_empty() {
            write_buffered_bytes(store, &mut self.buffer, dst).await?;
            return Ok(StreamState::Open);
        }
        let n = store
            .with(|store| dst.remaining(store))
            .unwrap_or(DEFAULT_BUFFER_CAPACITY)
            .min(MAX_BUFFER_CAPACITY);
        match self.read(n).await {
            StreamState::Open => {
                write_buffered_bytes(store, &mut self.buffer, dst).await?;
                Ok(StreamState::Open)
            }
            StreamState::Closed => Ok(StreamState::Closed),
        }
    }

    async fn when_ready(&mut self, _: &Accessor<D>) -> wasmtime::Result<StreamState> {
        if !self.buffer.get_ref().is_empty() {
            return Ok(StreamState::Open);
        }
        Ok(self.read(DEFAULT_BUFFER_CAPACITY).await)
    }
}

struct DirectoryStreamProducer {
    dir: Dir,
    entries: Option<cap_std::fs::ReadDir>,
    buffered: Option<DirectoryEntry>,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
}

impl DirectoryStreamProducer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let Some(tx) = self.result.take() {
            _ = tx.send(res);
        }
    }

    async fn next(&mut self) -> Option<DirectoryEntry> {
        let res = 'result: loop {
            let mut entries = if let Some(entries) = self.entries.take() {
                entries
            } else {
                // FIXME: Handle cancellation
                match self.dir.run_blocking(cap_std::fs::Dir::entries).await {
                    Ok(entries) => entries,
                    Err(err) => break 'result Err(err.into()),
                }
            };
            // FIXME: Handle cancellation
            let Some((res, tail)) = self
                .dir
                .run_blocking(move |_| entries.next().map(|entry| (entry, entries)))
                .await
            else {
                break 'result Ok(());
            };
            self.entries = Some(tail);
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
                    break 'result Err(err.into());
                }
            };
            let meta = match entry.metadata() {
                Ok(meta) => meta,
                Err(err) => break 'result Err(err.into()),
            };
            let Ok(name) = entry.file_name().into_string() else {
                break 'result Err(ErrorCode::IllegalByteSequence);
            };
            // FIXME: Handle cancellation
            return Some(DirectoryEntry {
                type_: meta.file_type().into(),
                name,
            });
        };
        self.close(res);
        None
    }
}

impl<D> StreamProducer<D, DirectoryEntry> for DirectoryStreamProducer {
    async fn produce(
        &mut self,
        store: &Accessor<D>,
        dst: &mut Destination<DirectoryEntry>,
    ) -> wasmtime::Result<StreamState> {
        let entry = if let Some(entry) = self.buffered.take() {
            entry
        } else if let Some(entry) = self.next().await {
            entry
        } else {
            return Ok(StreamState::Closed);
        };
        // FIXME: Handle cancellation
        if let Some(_) = dst.write(store, Some(entry)).await? {
            bail!("failed to write entry")
        }
        return Ok(StreamState::Open);
    }

    async fn when_ready(&mut self, _: &Accessor<D>) -> wasmtime::Result<StreamState> {
        if self.buffered.is_none() {
            let Some(entry) = self.next().await else {
                return Ok(StreamState::Closed);
            };
            self.buffered = Some(entry);
        }
        Ok(StreamState::Open)
    }
}

struct WriteStreamConsumer {
    file: File,
    offset: u64,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
    buffer: BytesMut,
}

impl Drop for WriteStreamConsumer {
    fn drop(&mut self) {
        self.close(Ok(()))
    }
}

impl WriteStreamConsumer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let Some(tx) = self.result.take() {
            _ = tx.send(res);
        }
    }

    async fn flush(&mut self) -> StreamState {
        // FIXME: `mem::take` rather than `clone` when we can ensure cancellation-safety
        //let buf = mem::take(&mut self.buffer);
        let buf = self.buffer.clone();
        let mut offset = self.offset;
        match self
            .file
            .spawn_blocking(move |file| {
                let mut pos = 0;
                while pos != buf.len() {
                    let n = file.write_at(&buf[pos..], offset)?;
                    pos = pos.saturating_add(n);
                    let n = n.try_into().or(Err(ErrorCode::Overflow))?;
                    offset = offset.checked_add(n).ok_or(ErrorCode::Overflow)?;
                }
                Ok((buf, offset))
            })
            .await
        {
            Ok((buf, offset)) => {
                self.buffer = buf;
                self.buffer.clear();
                self.offset = offset;
                StreamState::Open
            }
            Err(err) => {
                self.close(Err(err));
                StreamState::Closed
            }
        }
    }
}

impl<D> StreamConsumer<D, u8> for WriteStreamConsumer {
    async fn consume(
        &mut self,
        store: &Accessor<D>,
        src: &mut Source<'_, u8>,
    ) -> wasmtime::Result<StreamState> {
        store.with(|mut store| {
            let n = src.remaining(&mut store).min(MAX_BUFFER_CAPACITY);
            self.buffer.reserve(n);
            src.read(&mut store, &mut self.buffer)
        })?;
        Ok(self.flush().await)
    }

    async fn when_ready(&mut self, _: &Accessor<D>) -> wasmtime::Result<StreamState> {
        if !self.buffer.is_empty() {
            return Ok(self.flush().await);
        }
        Ok(StreamState::Open)
    }
}

struct AppendStreamConsumer {
    file: File,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
    buffer: BytesMut,
}

impl Drop for AppendStreamConsumer {
    fn drop(&mut self) {
        self.close(Ok(()))
    }
}

impl AppendStreamConsumer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let Some(tx) = self.result.take() {
            _ = tx.send(res);
        }
    }

    async fn flush(&mut self) -> StreamState {
        let buf = mem::take(&mut self.buffer);
        // FIXME: Handle cancellation
        match self
            .file
            .spawn_blocking(move |file| {
                let mut pos = 0;
                while pos != buf.len() {
                    let n = file.append(&buf[pos..])?;
                    pos = pos.saturating_add(n);
                }
                Ok(buf)
            })
            .await
        {
            Ok(buf) => {
                self.buffer = buf;
                self.buffer.clear();
                StreamState::Open
            }
            Err(err) => {
                self.close(Err(err));
                StreamState::Closed
            }
        }
    }
}

impl<D> StreamConsumer<D, u8> for AppendStreamConsumer {
    async fn consume(
        &mut self,
        store: &Accessor<D>,
        src: &mut Source<'_, u8>,
    ) -> wasmtime::Result<StreamState> {
        store.with(|mut store| {
            let n = src.remaining(&mut store).min(MAX_BUFFER_CAPACITY);
            self.buffer.reserve(n);
            src.read(&mut store, &mut self.buffer)
        })?;
        Ok(self.flush().await)
    }

    async fn when_ready(&mut self, _: &Accessor<D>) -> wasmtime::Result<StreamState> {
        if !self.buffer.is_empty() {
            return Ok(self.flush().await);
        }
        Ok(StreamState::Open)
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
        let instance = store.instance();
        store.with(|mut store| {
            let file = get_file(store.get().table, &fd)?;
            if !file.perms.contains(FilePerms::READ) {
                return Ok((
                    StreamReader::new(instance, &mut store, StreamEmptyProducer),
                    FutureReader::new(
                        instance,
                        &mut store,
                        FutureReadyProducer(Err(ErrorCode::NotPermitted)),
                    ),
                ));
            }

            let file = file.clone();
            let (result_tx, result_rx) = oneshot::channel();
            Ok((
                StreamReader::new(
                    instance,
                    &mut store,
                    ReadStreamProducer {
                        file,
                        offset,
                        result: Some(result_tx),
                        buffer: Cursor::default(),
                    },
                ),
                FutureReader::new(instance, &mut store, FutureOneshotProducer(result_rx)),
            ))
        })
    }

    async fn write_via_stream<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        data: StreamReader<u8>,
        offset: Filesize,
    ) -> FilesystemResult<()> {
        let (result_tx, result_rx) = oneshot::channel();
        store.with(|mut store| {
            let file = get_file(store.get().table, &fd)?;
            if !file.perms.contains(FilePerms::WRITE) {
                return Err(ErrorCode::NotPermitted.into());
            }
            let file = file.clone();
            data.pipe(
                store,
                WriteStreamConsumer {
                    file,
                    offset,
                    result: Some(result_tx),
                    buffer: BytesMut::default(),
                },
            );
            FilesystemResult::Ok(())
        })?;
        result_rx
            .await
            .context("oneshot sender dropped")
            .map_err(FilesystemError::trap)??;
        Ok(())
    }

    async fn append_via_stream<U>(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        data: StreamReader<u8>,
    ) -> FilesystemResult<()> {
        let (result_tx, result_rx) = oneshot::channel();
        store.with(|mut store| {
            let file = get_file(store.get().table, &fd)?;
            if !file.perms.contains(FilePerms::WRITE) {
                return Err(ErrorCode::NotPermitted.into());
            }
            let file = file.clone();
            data.pipe(
                store,
                AppendStreamConsumer {
                    file,
                    result: Some(result_tx),
                    buffer: BytesMut::default(),
                },
            );
            FilesystemResult::Ok(())
        })?;
        result_rx
            .await
            .context("oneshot sender dropped")
            .map_err(FilesystemError::trap)??;
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
        let instance = store.instance();
        store.with(|mut store| {
            let dir = get_dir(store.get().table, &fd)?;
            if !dir.perms.contains(DirPerms::READ) {
                return Ok((
                    StreamReader::new(instance, &mut store, StreamEmptyProducer),
                    FutureReader::new(
                        instance,
                        &mut store,
                        FutureReadyProducer(Err(ErrorCode::NotPermitted)),
                    ),
                ));
            }

            let dir = dir.clone();
            let (result_tx, result_rx) = oneshot::channel();
            Ok((
                StreamReader::new(
                    instance,
                    &mut store,
                    DirectoryStreamProducer {
                        dir,
                        entries: None,
                        buffered: None,
                        result: Some(result_tx),
                    },
                ),
                FutureReader::new(instance, &mut store, FutureOneshotProducer(result_rx)),
            ))
        })
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

use crate::filesystem::{Descriptor, Dir, File, WasiFilesystem, WasiFilesystemCtxView};
use crate::p3::bindings::clocks::wall_clock;
use crate::p3::bindings::filesystem::types::{
    self, Advice, DescriptorFlags, DescriptorStat, DescriptorType, DirectoryEntry, ErrorCode,
    Filesize, MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
};
use crate::p3::filesystem::{FilesystemError, FilesystemResult, preopens};
use crate::p3::{
    DEFAULT_BUFFER_CAPACITY, FutureOneshotProducer, FutureReadyProducer, MAX_BUFFER_CAPACITY,
    StreamEmptyProducer,
};
use crate::{DirPerms, FilePerms};
use anyhow::{Context as _, bail};
use bytes::BytesMut;
use core::mem;
use futures::FutureExt;
use std::io::Cursor;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{self, Context, Poll};
use system_interface::fs::FileIoExt as _;
use tokio::sync::oneshot;
use wasmtime::component::{
    Accessor, Destination, FutureReader, Resource, ResourceTable, Source, StreamConsumer,
    StreamProducer, StreamReader, StreamResult,
};
use wasmtime::{AsContextMut as _, StoreContextMut};

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
    future: Option<Pin<Box<dyn Future<Output = Result<Option<BytesMut>, ErrorCode>> + Send>>>,
}

impl ReadStreamProducer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let Some(tx) = self.result.take() {
            _ = tx.send(res);
        }
    }
}

impl<D> StreamProducer<D> for ReadStreamProducer {
    type Item = u8;
    type Buffer = Cursor<BytesMut>;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        destination: &'a mut Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let me = self.get_mut();
        if me.future.is_none() {
            if finish {
                return Poll::Ready(Ok(StreamResult::Cancelled));
            }

            let capacity = destination
                .remaining(store)
                .unwrap_or(DEFAULT_BUFFER_CAPACITY)
                // In the case of small or zero-length reads, we read more than
                // was asked for; this will save the runtime from having to
                // block or call `poll_produce` on subsequent reads.  See the
                // documentation for `StreamProducer::poll_produce` for details.
                .max(DEFAULT_BUFFER_CAPACITY)
                .min(MAX_BUFFER_CAPACITY);
            let mut buffer = destination.take_buffer().into_inner();
            buffer.resize(capacity, 0);
            let offset = me.offset;
            let file = me.file.clone();
            me.future = Some(
                async move {
                    match file
                        .run_blocking(move |file| {
                            let n = file.read_at(&mut buffer, offset)?;
                            buffer.truncate(n);
                            std::io::Result::Ok(buffer)
                        })
                        .await
                    {
                        Ok(buffer) if buffer.is_empty() => Ok(None),
                        Ok(buffer) => {
                            let n_u64 = buffer.len().try_into().or(Err(ErrorCode::Overflow))?;
                            offset.checked_add(n_u64).ok_or(ErrorCode::Overflow)?;
                            Ok(Some(buffer))
                        }
                        Err(err) => Err(err.into()),
                    }
                }
                .boxed(),
            );
        }

        let result = match task::ready!(me.future.as_mut().unwrap().as_mut().poll(cx)) {
            Ok(Some(buffer)) => {
                // We've already checked for overflow inside the future above,
                // so no need to do it again here:
                me.offset += u64::try_from(buffer.len()).unwrap();
                destination.set_buffer(Cursor::new(buffer));
                StreamResult::Completed
            }
            Ok(None) => {
                me.close(Ok(()));
                StreamResult::Dropped
            }
            Err(error) => {
                me.close(Err(error));
                StreamResult::Dropped
            }
        };

        me.future = None;

        Poll::Ready(Ok(result))
    }
}

struct DirectoryStreamProducer {
    dir: Dir,
    entries: Option<cap_std::fs::ReadDir>,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
    future: Option<
        Pin<
            Box<
                dyn Future<
                        Output = Result<Option<(DirectoryEntry, cap_std::fs::ReadDir)>, ErrorCode>,
                    > + Send,
            >,
        >,
    >,
}

impl DirectoryStreamProducer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let Some(tx) = self.result.take() {
            _ = tx.send(res);
        }
    }
}

impl<D> StreamProducer<D> for DirectoryStreamProducer {
    type Item = DirectoryEntry;
    type Buffer = Option<DirectoryEntry>;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        destination: &'a mut Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let me = self.get_mut();
        if me.future.is_none() {
            if finish {
                return Poll::Ready(Ok(StreamResult::Cancelled));
            }

            let dir = me.dir.clone();
            let mut entries = me.entries.take();
            me.future = Some(
                async move {
                    loop {
                        let mut entries = if let Some(entries) = entries.take() {
                            entries
                        } else {
                            // FIXME: Handle cancellation
                            match dir.run_blocking(cap_std::fs::Dir::entries).await {
                                Ok(entries) => entries,
                                Err(err) => break Err(err.into()),
                            }
                        };
                        // FIXME: Handle cancellation
                        let Some((res, tail)) = dir
                            .run_blocking(move |_| entries.next().map(|entry| (entry, entries)))
                            .await
                        else {
                            break Ok(None);
                        };
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
                        break Ok(Some((
                            DirectoryEntry {
                                type_: meta.file_type().into(),
                                name,
                            },
                            tail,
                        )));
                    }
                }
                .boxed(),
            );
        }

        let result = match task::ready!(me.future.as_mut().unwrap().as_mut().poll(cx)) {
            Ok(Some((entry, entries))) => {
                destination.set_buffer(Some(entry));
                me.entries = Some(entries);
                StreamResult::Completed
            }
            Ok(None) => {
                me.close(Ok(()));
                StreamResult::Dropped
            }
            Err(error) => {
                me.close(Err(error));
                StreamResult::Dropped
            }
        };

        me.future = None;

        Poll::Ready(Ok(result))
    }
}

struct WriteStreamConsumer {
    file: File,
    offset: Option<u64>,
    buffer: BytesMut,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
    future: Option<Pin<Box<dyn Future<Output = Result<(BytesMut, usize), ErrorCode>> + Send>>>,
}

impl WriteStreamConsumer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let Some(tx) = self.result.take() {
            _ = tx.send(res);
        }
    }
}

impl<D> StreamConsumer<D> for WriteStreamConsumer {
    type Item = u8;

    fn poll_consume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<D>,
        source: &mut Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let me = self.get_mut();
        if me.future.is_none() {
            if finish {
                return Poll::Ready(Ok(StreamResult::Cancelled));
            }

            let offset = me.offset;
            let file = me.file.clone();
            let mut buffer = mem::take(&mut me.buffer);
            buffer.clear();
            buffer.extend_from_slice(source.as_direct_source(store.as_context_mut()).remaining());

            me.future = Some(
                async move {
                    file.spawn_blocking(move |file| {
                        let n = if let Some(offset) = offset {
                            let n = file.write_at(&buffer, offset)?;
                            let n_u64 = n.try_into().or(Err(ErrorCode::Overflow))?;
                            offset.checked_add(n_u64).ok_or(ErrorCode::Overflow)?;
                            n
                        } else {
                            file.append(&buffer)?
                        };
                        Ok((buffer, n))
                    })
                    .await
                }
                .boxed(),
            );
        }

        let result = match task::ready!(me.future.as_mut().unwrap().as_mut().poll(cx)) {
            Ok((mut buffer, count)) => {
                source.as_direct_source(store).mark_read(count);
                let result = if count < buffer.len() && finish {
                    StreamResult::Cancelled
                } else {
                    StreamResult::Completed
                };
                if let Some(offset) = me.offset.as_mut() {
                    // We've already checked for overflow inside the future
                    // above, so no need to do it again here:
                    *offset += u64::try_from(count).unwrap();
                }
                buffer.clear();
                me.buffer = buffer;
                result
            }
            Err(error) => {
                me.close(Err(error));
                StreamResult::Dropped
            }
        };

        me.future = None;

        Poll::Ready(Ok(result))
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
                    StreamReader::new(instance, &mut store, StreamEmptyProducer(PhantomData)),
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
                        future: None,
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
                    offset: Some(offset),
                    buffer: BytesMut::default(),
                    result: Some(result_tx),
                    future: None,
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
                WriteStreamConsumer {
                    file,
                    offset: None,
                    buffer: BytesMut::default(),
                    result: Some(result_tx),
                    future: None,
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
                    StreamReader::new(instance, &mut store, StreamEmptyProducer(PhantomData)),
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
                        result: Some(result_tx),
                        future: None,
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

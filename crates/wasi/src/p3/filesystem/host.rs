use crate::filesystem::sys;
use crate::filesystem::{Descriptor, Dir, File, WasiFilesystem, WasiFilesystemCtxView};
use crate::p3::bindings::clocks::system_clock;
use crate::p3::bindings::filesystem::types::{
    self, Advice, DescriptorFlags, DescriptorStat, DescriptorType, DirectoryEntry, ErrorCode,
    Filesize, MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
};
use crate::p3::filesystem::{FilesystemError, FilesystemResult, preopens};
use crate::p3::{DEFAULT_BUFFER_CAPACITY, FallibleIteratorProducer};
use bytes::BytesMut;
use core::pin::Pin;
use core::task::{Context, Poll, ready};
use core::{iter, mem};
use std::io;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{mpsc, oneshot};
use tokio::task::{JoinHandle, spawn_blocking};
use wasmtime::component::{
    Access, Accessor, Destination, FutureReader, Resource, ResourceTable, Source, StreamConsumer,
    StreamProducer, StreamReader, StreamResult,
};
use wasmtime::error::Context as _;
use wasmtime::{AsContextMut, StoreContextMut};

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

fn get_writable_file(table: &ResourceTable, fd: &Resource<Descriptor>) -> FilesystemResult<File> {
    let file = get_file(table, fd)?;
    if file.perms.write_not_permitted() {
        return Err(ErrorCode::NotPermitted.into());
    }
    Ok(file.clone())
}

fn systemtime_from(t: system_clock::Instant) -> Result<std::time::SystemTime, ErrorCode> {
    if let Ok(seconds) = t.seconds.try_into() {
        std::time::SystemTime::UNIX_EPOCH
            .checked_add(core::time::Duration::new(seconds, t.nanoseconds))
            .ok_or(ErrorCode::Overflow)
    } else {
        std::time::SystemTime::UNIX_EPOCH
            .checked_sub(core::time::Duration::new(
                t.seconds.unsigned_abs(),
                t.nanoseconds,
            ))
            .ok_or(ErrorCode::Overflow)
    }
}

fn systemtimespec_from(t: NewTimestamp) -> Result<Option<SystemTime>, ErrorCode> {
    match t {
        NewTimestamp::NoChange => Ok(None),
        NewTimestamp::Now => Ok(Some(SystemTime::now())),
        NewTimestamp::Timestamp(st) => Ok(Some(systemtime_from(st)?)),
    }
}

struct ReadStreamProducer {
    file: File,
    offset: u64,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
    task: Option<JoinHandle<std::io::Result<BytesMut>>>,
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

    /// Update the internal `offset` field after reading `amt` bytes from the file.
    fn complete_read(&mut self, amt: usize) -> StreamResult {
        let Ok(amt) = amt.try_into() else {
            self.close(Err(ErrorCode::Overflow));
            return StreamResult::Dropped;
        };
        let Some(amt) = self.offset.checked_add(amt) else {
            self.close(Err(ErrorCode::Overflow));
            return StreamResult::Dropped;
        };
        self.offset = amt;
        StreamResult::Completed
    }
}

impl<D> StreamProducer<D> for ReadStreamProducer {
    type Item = u8;
    type Buffer = BytesMut;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        if let Some(file) = self.file.as_blocking_file() {
            // Once a blocking file, always a blocking file, so assert as such.
            assert!(self.task.is_none());
            let mut dst = dst.as_direct(store, DEFAULT_BUFFER_CAPACITY);
            let buf = dst.remaining();
            if buf.is_empty() {
                return Poll::Ready(Ok(StreamResult::Completed));
            }
            return match sys::read_at_cursor_unspecified(file, buf, self.offset) {
                Ok(0) => {
                    self.close(Ok(()));
                    Poll::Ready(Ok(StreamResult::Dropped))
                }
                Ok(n) => {
                    dst.mark_written(n);
                    Poll::Ready(Ok(self.complete_read(n)))
                }
                Err(err) => {
                    self.close(Err(err.into()));
                    Poll::Ready(Ok(StreamResult::Dropped))
                }
            };
        }

        // Lazily spawn a read task if one hasn't already been spawned yet.
        let me = &mut *self;
        let task = me.task.get_or_insert_with(|| {
            let mut buf = dst.take_buffer();
            buf.resize(DEFAULT_BUFFER_CAPACITY, 0);
            let file = Arc::clone(me.file.as_file());
            let offset = me.offset;
            spawn_blocking(move || {
                sys::read_at_cursor_unspecified(&file, &mut buf, offset).map(|n| {
                    buf.truncate(n);
                    buf
                })
            })
        });

        // Await the completion of the read task. Note that this is not a
        // cancellable await point because we can't cancel the other task, so
        // the `finish` parameter is ignored.
        let result = match Pin::new(&mut *task).poll(cx) {
            // If cancellation is requested, then flag that to Tokio. Note that
            // this still waits for the actual completion of the spawned task,
            // which won't actually happen if it's already executing.
            Poll::Pending if finish => {
                task.abort();
                ready!(Pin::new(task).poll(cx))
            }
            other => ready!(other),
        };
        self.task = None;
        match result {
            Ok(Ok(buf)) if buf.is_empty() => {
                self.close(Ok(()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Ok(Ok(buf)) => {
                let n = buf.len();
                dst.set_buffer(buf);
                Poll::Ready(Ok(self.complete_read(n)))
            }
            Ok(Err(err)) => {
                self.close(Err(err.into()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Err(err) => {
                if err.is_cancelled() {
                    return Poll::Ready(Ok(StreamResult::Cancelled));
                }
                panic!("I/O task should not panic: {err}")
            }
        }
    }
}

fn map_dir_entry(
    entry: std::io::Result<crate::filesystem::primitives::DirEntry>,
) -> Result<Option<DirectoryEntry>, ErrorCode> {
    match entry {
        Ok(entry) => {
            let meta = entry.metadata()?;
            let Ok(name) = entry.file_name().into_string() else {
                return Err(ErrorCode::IllegalByteSequence);
            };
            Ok(Some(DirectoryEntry {
                type_: meta.file_type().into(),
                name,
            }))
        }
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
                    return Ok(None);
                }
            }
            Err(err.into())
        }
    }
}

struct ReadDirStream {
    rx: mpsc::Receiver<DirectoryEntry>,
    task: JoinHandle<Result<(), ErrorCode>>,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
}

impl ReadDirStream {
    fn new(
        dir: Arc<std::fs::File>,
        result: oneshot::Sender<Result<(), ErrorCode>>,
    ) -> ReadDirStream {
        let (tx, rx) = mpsc::channel(1);
        ReadDirStream {
            task: spawn_blocking(move || {
                let entries = crate::filesystem::primitives::read_base_dir(&dir)?;
                for entry in entries {
                    if let Some(entry) = map_dir_entry(entry)? {
                        if let Err(_) = tx.blocking_send(entry) {
                            break;
                        }
                    }
                }
                Ok(())
            }),
            rx,
            result: Some(result),
        }
    }

    fn close(&mut self, res: Result<(), ErrorCode>) {
        self.rx.close();
        self.task.abort();
        let _ = self.result.take().unwrap().send(res);
    }
}

impl<D> StreamProducer<D> for ReadDirStream {
    type Item = DirectoryEntry;
    type Buffer = Option<DirectoryEntry>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'a, D>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        // If this is a 0-length read then `mpsc::Receiver` does not expose an
        // API to wait for an item to be available without taking it out of the
        // channel. In lieu of that just say that we're complete and ready for a
        // read.
        if dst.remaining(&mut store) == Some(0) {
            return Poll::Ready(Ok(StreamResult::Completed));
        }

        match self.rx.poll_recv(cx) {
            // If an item is on the channel then send that along and say that
            // the read is now complete with one item being yielded.
            Poll::Ready(Some(item)) => {
                dst.set_buffer(Some(item));
                Poll::Ready(Ok(StreamResult::Completed))
            }

            // If there's nothing left on the channel then that means that an
            // error occurred or the iterator is done. In both cases an
            // un-cancellable wait for the spawned task is entered and we await
            // its completion. Upon completion there our own stream is closed
            // with the result (sending an error code on our oneshot) and then
            // the stream is reported as dropped.
            Poll::Ready(None) => {
                let result = ready!(Pin::new(&mut self.task).poll(cx))
                    .expect("spawned task should not panic");
                self.close(result);
                Poll::Ready(Ok(StreamResult::Dropped))
            }

            // If an item isn't ready yet then cancel this outstanding request
            // if `finish` is set, otherwise propagate the `Pending` status.
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for ReadDirStream {
    fn drop(&mut self) {
        if self.result.is_some() {
            self.close(Ok(()));
        }
    }
}

struct WriteStreamConsumer {
    file: File,
    location: WriteLocation,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
    buffer: BytesMut,
    task: Option<JoinHandle<std::io::Result<(BytesMut, usize)>>>,
}

#[derive(Copy, Clone)]
enum WriteLocation {
    End,
    Offset(u64),
}

impl WriteStreamConsumer {
    fn new(
        file: File,
        location: WriteLocation,
        result: oneshot::Sender<Result<(), ErrorCode>>,
    ) -> Self {
        Self {
            file,
            location,
            result: Some(result),
            buffer: BytesMut::default(),
            task: None,
        }
    }

    fn close(&mut self, res: Result<(), ErrorCode>) {
        _ = self.result.take().unwrap().send(res);
    }

    /// Update the internal `offset` field after writing `amt` bytes from the file.
    fn complete_write(&mut self, amt: usize) -> StreamResult {
        match &mut self.location {
            WriteLocation::End => StreamResult::Completed,
            WriteLocation::Offset(offset) => {
                let Ok(amt) = amt.try_into() else {
                    self.close(Err(ErrorCode::Overflow));
                    return StreamResult::Dropped;
                };
                let Some(amt) = offset.checked_add(amt) else {
                    self.close(Err(ErrorCode::Overflow));
                    return StreamResult::Dropped;
                };
                *offset = amt;
                StreamResult::Completed
            }
        }
    }
}

impl WriteLocation {
    fn write(&self, file: &std::fs::File, bytes: &[u8]) -> io::Result<usize> {
        match *self {
            WriteLocation::End => sys::append_cursor_unspecified(file, bytes),
            WriteLocation::Offset(at) => sys::write_at_cursor_unspecified(file, bytes, at),
        }
    }
}

impl<D> StreamConsumer<D> for WriteStreamConsumer {
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let mut src = src.as_direct(store);
        if let Some(file) = self.file.as_blocking_file() {
            // Once a blocking file, always a blocking file, so assert as such.
            assert!(self.task.is_none());
            return match self.location.write(file, src.remaining()) {
                Ok(n) => {
                    src.mark_read(n);
                    Poll::Ready(Ok(self.complete_write(n)))
                }
                Err(err) => {
                    self.close(Err(err.into()));
                    Poll::Ready(Ok(StreamResult::Dropped))
                }
            };
        }
        let me = &mut *self;
        let task = me.task.get_or_insert_with(|| {
            debug_assert!(me.buffer.is_empty());
            let remaining = src.remaining();
            let n = remaining.len().min(DEFAULT_BUFFER_CAPACITY);
            me.buffer.extend_from_slice(&remaining[..n]);
            let buf = mem::take(&mut me.buffer);
            let file = Arc::clone(me.file.as_file());
            let location = me.location;
            spawn_blocking(move || location.write(&file, &buf).map(|n| (buf, n)))
        });
        let result = match Pin::new(&mut *task).poll(cx) {
            // If cancellation is requested, then flag that to Tokio. Note that
            // this still waits for the actual completion of the spawned task,
            // which won't actually happen if it's already executing.
            Poll::Pending if finish => {
                task.abort();
                ready!(Pin::new(task).poll(cx))
            }
            other => ready!(other),
        };
        self.task = None;
        match result {
            Ok(Ok((buf, n))) => {
                src.mark_read(n);
                self.buffer = buf;
                self.buffer.clear();
                Poll::Ready(Ok(self.complete_write(n)))
            }
            Ok(Err(err)) => {
                self.close(Err(err.into()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Err(err) => {
                if err.is_cancelled() {
                    return Poll::Ready(Ok(StreamResult::Cancelled));
                }
                panic!("I/O task should not panic: {err}")
            }
        }
    }
}

impl Drop for WriteStreamConsumer {
    fn drop(&mut self) {
        if self.result.is_some() {
            self.close(Ok(()))
        }
    }
}

impl types::Host for WasiFilesystemCtxView<'_> {
    fn convert_error_code(&mut self, error: FilesystemError) -> wasmtime::Result<ErrorCode> {
        error.downcast()
    }
}

fn read_via_stream(
    mut store: impl AsContextMut,
    fd: Descriptor,
    offset: Filesize,
) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
    let mut store = store.as_context_mut();
    let file = match fd {
        Descriptor::File(file) => file,
        Descriptor::Dir(_) => {
            return Ok((
                StreamReader::new(&mut store, iter::empty())?,
                FutureReader::new(&mut store, async move {
                    wasmtime::error::Ok(Err(ErrorCode::IsDirectory))
                })?,
            ));
        }
    };
    let (result_tx, result_rx) = oneshot::channel();
    Ok((
        StreamReader::new(
            &mut store,
            ReadStreamProducer {
                file,
                offset,
                result: Some(result_tx),
                task: None,
            },
        )?,
        FutureReader::new(&mut store, result_rx)?,
    ))
}

fn write_via_stream(
    mut store: impl AsContextMut,
    file: FilesystemResult<File>,
    mut data: StreamReader<u8>,
    location: WriteLocation,
) -> wasmtime::Result<FutureReader<Result<(), ErrorCode>>> {
    let mut store = store.as_context_mut();
    let (result_tx, result_rx) = oneshot::channel();
    match file {
        Ok(file) => {
            data.pipe(
                &mut store,
                WriteStreamConsumer::new(file, location, result_tx),
            )?;
        }
        Err(err) => {
            data.close(&mut store)?;
            let _ = result_tx.send(Err(err.downcast().unwrap_or(ErrorCode::Io)));
        }
    }
    FutureReader::new(&mut store, result_rx)
}

fn read_directory(
    mut store: impl AsContextMut,
    dir: FilesystemResult<Dir>,
) -> wasmtime::Result<(
    StreamReader<DirectoryEntry>,
    FutureReader<Result<(), ErrorCode>>,
)> {
    let mut store = store.as_context_mut();
    let (result_tx, result_rx) = oneshot::channel();
    let stream = match dir {
        Ok(dir) => {
            let allow_blocking_current_thread = dir.allow_blocking_current_thread;
            let dir = Arc::clone(dir.as_dir());
            if allow_blocking_current_thread {
                match crate::filesystem::primitives::read_base_dir(&dir) {
                    Ok(readdir) => StreamReader::new(
                        &mut store,
                        FallibleIteratorProducer::new(
                            readdir.filter_map(|e| map_dir_entry(e).transpose()),
                            result_tx,
                        ),
                    )?,
                    Err(e) => {
                        let _ = result_tx.send(Err(e.into()));
                        StreamReader::new(&mut store, iter::empty())?
                    }
                }
            } else {
                StreamReader::new(&mut store, ReadDirStream::new(dir, result_tx))?
            }
        }
        Err(err) => {
            let _ = result_tx.send(Err(err.downcast().unwrap_or(ErrorCode::Io)));
            StreamReader::new(&mut store, iter::empty())?
        }
    };
    Ok((stream, FutureReader::new(&mut store, result_rx)?))
}

impl WasiFilesystemCtxView<'_> {
    fn advise(
        &self,
        fd: &Resource<Descriptor>,
        offset: Filesize,
        length: Filesize,
        advice: Advice,
    ) -> impl Future<Output = FilesystemResult<()>> + use<> {
        let file = get_file(self.table, fd).cloned();
        async move {
            file?.advise(offset, length, advice.into()).await?;
            Ok(())
        }
    }

    fn get_flags(
        &self,
        fd: &Resource<Descriptor>,
    ) -> impl Future<Output = FilesystemResult<DescriptorFlags>> + use<> {
        let fd = get_descriptor(self.table, fd).cloned();
        async move {
            let flags = fd?.get_flags().await?;
            Ok(flags.into())
        }
    }

    fn get_type(
        &self,
        fd: &Resource<Descriptor>,
    ) -> impl Future<Output = FilesystemResult<DescriptorType>> + use<> {
        let fd = get_descriptor(self.table, fd).cloned();
        async move {
            let ty = fd?.get_type().await?;
            Ok(ty.into())
        }
    }

    fn set_size(
        &self,
        fd: &Resource<Descriptor>,
        size: Filesize,
    ) -> impl Future<Output = FilesystemResult<()>> + use<> {
        let file = get_file(self.table, fd).cloned();
        async move {
            file?.set_size(size).await?;
            Ok(())
        }
    }

    fn set_times(
        &self,
        fd: &Resource<Descriptor>,
        data_access_timestamp: NewTimestamp,
        data_modification_timestamp: NewTimestamp,
    ) -> impl Future<Output = FilesystemResult<()>> + use<> {
        let fd = get_descriptor(self.table, &fd).cloned();
        async move {
            let atim = systemtimespec_from(data_access_timestamp)?;
            let mtim = systemtimespec_from(data_modification_timestamp)?;
            fd?.set_times(atim, mtim).await?;
            Ok(())
        }
    }

    fn sync(
        &self,
        fd: &Resource<Descriptor>,
    ) -> impl Future<Output = FilesystemResult<()>> + use<> {
        let fd = get_descriptor(self.table, &fd).cloned();
        async move {
            fd?.sync().await?;
            Ok(())
        }
    }

    fn sync_data(
        &self,
        fd: &Resource<Descriptor>,
    ) -> impl Future<Output = FilesystemResult<()>> + use<> {
        let fd = get_descriptor(self.table, &fd).cloned();
        async move {
            fd?.sync_data().await?;
            Ok(())
        }
    }

    fn create_directory_at(
        &self,
        fd: &Resource<Descriptor>,
        path: String,
    ) -> impl Future<Output = FilesystemResult<()>> + use<> {
        let dir = get_dir(self.table, &fd).cloned();
        async move {
            dir?.create_directory_at(path).await?;
            Ok(())
        }
    }

    fn stat(
        &self,
        fd: &Resource<Descriptor>,
    ) -> impl Future<Output = FilesystemResult<DescriptorStat>> + use<> {
        let fd = get_descriptor(self.table, &fd).cloned();
        async move {
            let stat = fd?.stat().await?;
            Ok(stat.into())
        }
    }

    fn stat_at(
        &self,
        fd: &Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
    ) -> impl Future<Output = FilesystemResult<DescriptorStat>> + use<> {
        let dir = get_dir(self.table, &fd).cloned();
        async move {
            let stat = dir?.stat_at(path_flags.into(), path).await?;
            Ok(stat.into())
        }
    }

    fn set_times_at(
        &self,
        fd: &Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
        data_access_timestamp: NewTimestamp,
        data_modification_timestamp: NewTimestamp,
    ) -> impl Future<Output = FilesystemResult<()>> + use<> {
        let dir = get_dir(self.table, &fd).cloned();
        async move {
            let atim = systemtimespec_from(data_access_timestamp)?;
            let mtim = systemtimespec_from(data_modification_timestamp)?;
            dir?.set_times_at(path_flags.into(), path, atim, mtim)
                .await?;
            Ok(())
        }
    }

    fn link_at(
        &self,
        old_fd: &Resource<Descriptor>,
        old_path_flags: PathFlags,
        old_path: String,
        new_fd: &Resource<Descriptor>,
        new_path: String,
    ) -> impl Future<Output = FilesystemResult<()>> + use<> {
        let old_dir = get_dir(self.table, old_fd).cloned();
        let new_dir = get_dir(self.table, new_fd).cloned();

        async move {
            old_dir?
                .link_at(old_path_flags.into(), old_path, &new_dir?, new_path)
                .await?;
            Ok(())
        }
    }

    fn open_at(
        &self,
        fd: &Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
        open_flags: OpenFlags,
        flags: DescriptorFlags,
    ) -> impl Future<Output = FilesystemResult<Descriptor>> + use<> {
        let dir = get_dir(self.table, fd).cloned();
        let allow_blocking_current_thread = self.ctx.allow_blocking_current_thread;
        async move {
            let fd = dir?
                .open_at(
                    path_flags.into(),
                    path,
                    open_flags.into(),
                    flags.into(),
                    allow_blocking_current_thread,
                )
                .await?;
            Ok(fd)
        }
    }

    fn readlink_at(
        &self,
        fd: &Resource<Descriptor>,
        path: String,
    ) -> impl Future<Output = FilesystemResult<String>> + use<> {
        let dir = get_dir(self.table, fd).cloned();
        async move { Ok(dir?.readlink_at(path).await?) }
    }

    fn remove_directory_at(
        &self,
        fd: &Resource<Descriptor>,
        path: String,
    ) -> impl Future<Output = FilesystemResult<()>> + use<> {
        let dir = get_dir(self.table, fd).cloned();
        async move {
            dir?.remove_directory_at(path).await?;
            Ok(())
        }
    }

    fn rename_at(
        &self,
        fd: &Resource<Descriptor>,
        old_path: String,
        new_fd: &Resource<Descriptor>,
        new_path: String,
    ) -> impl Future<Output = FilesystemResult<()>> + use<> {
        let old_dir = get_dir(self.table, fd).cloned();
        let new_dir = get_dir(self.table, new_fd).cloned();
        async move {
            old_dir?.rename_at(old_path, &new_dir?, new_path).await?;
            Ok(())
        }
    }

    fn symlink_at(
        &self,
        fd: &Resource<Descriptor>,
        old_path: String,
        new_path: String,
    ) -> impl Future<Output = FilesystemResult<()>> + use<> {
        let dir = get_dir(self.table, fd).cloned();
        async move {
            dir?.symlink_at(old_path, new_path).await?;
            Ok(())
        }
    }

    fn unlink_file_at(
        &self,
        fd: &Resource<Descriptor>,
        path: String,
    ) -> impl Future<Output = FilesystemResult<()>> + use<> {
        let dir = get_dir(self.table, fd).cloned();
        async move {
            dir?.unlink_file_at(path).await?;
            Ok(())
        }
    }

    fn is_same_object(
        &self,
        fd: &Resource<Descriptor>,
        other: &Resource<Descriptor>,
    ) -> impl Future<Output = wasmtime::Result<bool>> + use<> {
        let fd = get_descriptor(self.table, fd).cloned();
        let other = get_descriptor(self.table, other).cloned();
        async move { fd?.is_same_object(&other?).await }
    }

    fn metadata_hash(
        &self,
        fd: &Resource<Descriptor>,
    ) -> impl Future<Output = FilesystemResult<MetadataHashValue>> + use<> {
        let fd = get_descriptor(self.table, fd).cloned();
        async move {
            let meta = fd?.metadata_hash().await?;
            Ok(meta.into())
        }
    }

    fn metadata_hash_at(
        &self,
        fd: &Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
    ) -> impl Future<Output = FilesystemResult<MetadataHashValue>> + use<> {
        let dir = get_dir(self.table, fd).cloned();
        async move {
            let meta = dir?.metadata_hash_at(path_flags.into(), path).await?;
            Ok(meta.into())
        }
    }
}

impl<U> types::HostDescriptorWithStore<U> for WasiFilesystem {
    fn read_via_stream(
        mut store: Access<U, Self>,
        fd: Resource<Descriptor>,
        offset: Filesize,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
        let fd = get_descriptor(store.get().table, &fd)?.clone();
        read_via_stream(&mut store, fd, offset)
    }

    fn write_via_stream(
        mut store: Access<'_, U, Self>,
        fd: Resource<Descriptor>,
        data: StreamReader<u8>,
        offset: Filesize,
    ) -> wasmtime::Result<FutureReader<Result<(), ErrorCode>>> {
        let file = get_writable_file(store.get().table, &fd);
        write_via_stream(&mut store, file, data, WriteLocation::Offset(offset))
    }

    fn append_via_stream(
        mut store: Access<'_, U, Self>,
        fd: Resource<Descriptor>,
        data: StreamReader<u8>,
    ) -> wasmtime::Result<FutureReader<Result<(), ErrorCode>>> {
        let file = get_writable_file(store.get().table, &fd);
        write_via_stream(&mut store, file, data, WriteLocation::End)
    }

    async fn advise(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        offset: Filesize,
        length: Filesize,
        advice: Advice,
    ) -> FilesystemResult<()> {
        store
            .with(|mut s| s.get().advise(&fd, offset, length, advice))
            .await
    }

    async fn sync_data(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> FilesystemResult<()> {
        store.with(|mut s| s.get().sync_data(&fd)).await
    }

    async fn get_flags(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> FilesystemResult<DescriptorFlags> {
        store.with(|mut s| s.get().get_flags(&fd)).await
    }

    async fn get_type(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> FilesystemResult<DescriptorType> {
        store.with(|mut s| s.get().get_type(&fd)).await
    }

    async fn set_size(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        size: Filesize,
    ) -> FilesystemResult<()> {
        store.with(|mut s| s.get().set_size(&fd, size)).await
    }

    async fn set_times(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        data_access_timestamp: NewTimestamp,
        data_modification_timestamp: NewTimestamp,
    ) -> FilesystemResult<()> {
        store
            .with(|mut s| {
                s.get()
                    .set_times(&fd, data_access_timestamp, data_modification_timestamp)
            })
            .await
    }

    fn read_directory(
        mut store: Access<'_, U, Self>,
        fd: Resource<Descriptor>,
    ) -> wasmtime::Result<(
        StreamReader<DirectoryEntry>,
        FutureReader<Result<(), ErrorCode>>,
    )> {
        let dir = get_dir(store.get().table, &fd).cloned();
        read_directory(&mut store, dir)
    }

    async fn sync(store: &Accessor<U, Self>, fd: Resource<Descriptor>) -> FilesystemResult<()> {
        store.with(|mut s| s.get().sync(&fd)).await
    }

    async fn create_directory_at(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path: String,
    ) -> FilesystemResult<()> {
        store
            .with(|mut s| s.get().create_directory_at(&fd, path))
            .await
    }

    async fn stat(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> FilesystemResult<DescriptorStat> {
        store.with(|mut s| s.get().stat(&fd)).await
    }

    async fn stat_at(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
    ) -> FilesystemResult<DescriptorStat> {
        store
            .with(|mut s| s.get().stat_at(&fd, path_flags, path))
            .await
    }

    async fn set_times_at(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
        data_access_timestamp: NewTimestamp,
        data_modification_timestamp: NewTimestamp,
    ) -> FilesystemResult<()> {
        store
            .with(|mut s| {
                s.get().set_times_at(
                    &fd,
                    path_flags,
                    path,
                    data_access_timestamp,
                    data_modification_timestamp,
                )
            })
            .await
    }

    async fn link_at(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        old_path_flags: PathFlags,
        old_path: String,
        new_fd: Resource<Descriptor>,
        new_path: String,
    ) -> FilesystemResult<()> {
        store
            .with(|mut s| {
                s.get()
                    .link_at(&fd, old_path_flags, old_path, &new_fd, new_path)
            })
            .await
    }

    async fn open_at(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
        open_flags: OpenFlags,
        flags: DescriptorFlags,
    ) -> FilesystemResult<Resource<Descriptor>> {
        let fd = store
            .with(|mut s| s.get().open_at(&fd, path_flags, path, open_flags, flags))
            .await?;
        let fd = store.with(|mut store| store.get().table.push(fd))?;
        Ok(fd)
    }

    async fn readlink_at(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path: String,
    ) -> FilesystemResult<String> {
        store.with(|mut s| s.get().readlink_at(&fd, path)).await
    }

    async fn remove_directory_at(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path: String,
    ) -> FilesystemResult<()> {
        store
            .with(|mut s| s.get().remove_directory_at(&fd, path))
            .await
    }

    async fn rename_at(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        old_path: String,
        new_fd: Resource<Descriptor>,
        new_path: String,
    ) -> FilesystemResult<()> {
        store
            .with(|mut s| s.get().rename_at(&fd, old_path, &new_fd, new_path))
            .await
    }

    async fn symlink_at(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        old_path: String,
        new_path: String,
    ) -> FilesystemResult<()> {
        store
            .with(|mut s| s.get().symlink_at(&fd, old_path, new_path))
            .await
    }

    async fn unlink_file_at(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path: String,
    ) -> FilesystemResult<()> {
        store.with(|mut s| s.get().unlink_file_at(&fd, path)).await
    }

    async fn is_same_object(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        other: Resource<Descriptor>,
    ) -> wasmtime::Result<bool> {
        store
            .with(|mut s| s.get().is_same_object(&fd, &other))
            .await
    }

    async fn metadata_hash(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
    ) -> FilesystemResult<MetadataHashValue> {
        store.with(|mut s| s.get().metadata_hash(&fd)).await
    }

    async fn metadata_hash_at(
        store: &Accessor<U, Self>,
        fd: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
    ) -> FilesystemResult<MetadataHashValue> {
        store
            .with(|mut s| s.get().metadata_hash_at(&fd, path_flags, path))
            .await
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

mod named {
    use crate::filesystem::{Descriptor, WasiFilesystemNamed, WasiFilesystemNamedView};
    use crate::p3::bindings::filesystem::types::{
        Advice, DescriptorFlags, DescriptorStat, DescriptorType, DirectoryEntry, ErrorCode,
        Filesize, MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
    };
    use crate::p3::bindings::named_imports::wasi::filesystem::{preopens, types};
    use crate::p3::filesystem::{FilesystemError, FilesystemResult};
    use crate::{NamedId, WasiCtxNamedView};
    use wasmtime::component::{Access, Accessor, FutureReader, Resource, StreamReader};

    impl<T> types::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiFilesystemNamedView,
    {
        fn convert_error_code(&mut self, error: FilesystemError) -> wasmtime::Result<ErrorCode> {
            error.downcast()
        }
    }

    impl<T, U> types::HostDescriptorWithStore<U> for WasiFilesystemNamed<T>
    where
        T: WasiFilesystemNamedView,
    {
        fn read_via_stream(
            mut store: Access<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            offset: Filesize,
        ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
            let fd = super::get_descriptor(store.get().0.filesystem(id).table, &fd)?.clone();
            super::read_via_stream(&mut store, fd, offset)
        }

        fn write_via_stream(
            mut store: Access<'_, U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            data: StreamReader<u8>,
            offset: Filesize,
        ) -> wasmtime::Result<FutureReader<Result<(), ErrorCode>>> {
            let file = super::get_writable_file(store.get().0.filesystem(id).table, &fd);
            super::write_via_stream(&mut store, file, data, super::WriteLocation::Offset(offset))
        }

        fn append_via_stream(
            mut store: Access<'_, U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            data: StreamReader<u8>,
        ) -> wasmtime::Result<FutureReader<Result<(), ErrorCode>>> {
            let file = super::get_writable_file(store.get().0.filesystem(id).table, &fd);
            super::write_via_stream(&mut store, file, data, super::WriteLocation::End)
        }

        async fn advise(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            offset: Filesize,
            length: Filesize,
            advice: Advice,
        ) -> FilesystemResult<()> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.advise(&fd, offset, length, advice)
            });
            result.await
        }

        async fn sync_data(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
        ) -> FilesystemResult<()> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.sync_data(&fd)
            });
            result.await
        }

        async fn get_flags(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
        ) -> FilesystemResult<DescriptorFlags> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.get_flags(&fd)
            });
            result.await
        }

        async fn get_type(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
        ) -> FilesystemResult<DescriptorType> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.get_type(&fd)
            });
            result.await
        }

        async fn set_size(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            size: Filesize,
        ) -> FilesystemResult<()> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.set_size(&fd, size)
            });
            result.await
        }

        async fn set_times(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            data_access_timestamp: NewTimestamp,
            data_modification_timestamp: NewTimestamp,
        ) -> FilesystemResult<()> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.set_times(&fd, data_access_timestamp, data_modification_timestamp)
            });
            result.await
        }

        fn read_directory(
            mut store: Access<'_, U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
        ) -> wasmtime::Result<(
            StreamReader<DirectoryEntry>,
            FutureReader<Result<(), ErrorCode>>,
        )> {
            let dir = super::get_dir(store.get().0.filesystem(id).table, &fd).cloned();
            super::read_directory(&mut store, dir)
        }

        async fn sync(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
        ) -> FilesystemResult<()> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.sync(&fd)
            });
            result.await
        }

        async fn create_directory_at(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            path: String,
        ) -> FilesystemResult<()> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.create_directory_at(&fd, path)
            });
            result.await
        }

        async fn stat(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
        ) -> FilesystemResult<DescriptorStat> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.stat(&fd)
            });
            result.await
        }

        async fn stat_at(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            path_flags: PathFlags,
            path: String,
        ) -> FilesystemResult<DescriptorStat> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.stat_at(&fd, path_flags, path)
            });
            result.await
        }

        async fn set_times_at(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            path_flags: PathFlags,
            path: String,
            data_access_timestamp: NewTimestamp,
            data_modification_timestamp: NewTimestamp,
        ) -> FilesystemResult<()> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.set_times_at(
                    &fd,
                    path_flags,
                    path,
                    data_access_timestamp,
                    data_modification_timestamp,
                )
            });
            result.await
        }

        async fn link_at(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            old_path_flags: PathFlags,
            old_path: String,
            new_fd: Resource<Descriptor>,
            new_path: String,
        ) -> FilesystemResult<()> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.link_at(&fd, old_path_flags, old_path, &new_fd, new_path)
            });
            result.await
        }

        async fn open_at(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            path_flags: PathFlags,
            path: String,
            open_flags: OpenFlags,
            flags: DescriptorFlags,
        ) -> FilesystemResult<Resource<Descriptor>> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.open_at(&fd, path_flags, path, open_flags, flags)
            });
            let fd = result.await?;
            let fd = store.with(|mut store| store.get().0.filesystem(id).table.push(fd))?;
            Ok(fd)
        }

        async fn readlink_at(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            path: String,
        ) -> FilesystemResult<String> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.readlink_at(&fd, path)
            });
            result.await
        }

        async fn remove_directory_at(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            path: String,
        ) -> FilesystemResult<()> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.remove_directory_at(&fd, path)
            });
            result.await
        }

        async fn rename_at(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            old_path: String,
            new_fd: Resource<Descriptor>,
            new_path: String,
        ) -> FilesystemResult<()> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.rename_at(&fd, old_path, &new_fd, new_path)
            });
            result.await
        }

        async fn symlink_at(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            old_path: String,
            new_path: String,
        ) -> FilesystemResult<()> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.symlink_at(&fd, old_path, new_path)
            });
            result.await
        }

        async fn unlink_file_at(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            path: String,
        ) -> FilesystemResult<()> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.unlink_file_at(&fd, path)
            });
            result.await
        }

        async fn is_same_object(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            other: Resource<Descriptor>,
        ) -> wasmtime::Result<bool> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.is_same_object(&fd, &other)
            });
            result.await
        }

        async fn metadata_hash(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
        ) -> FilesystemResult<MetadataHashValue> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.metadata_hash(&fd)
            });
            result.await
        }

        async fn metadata_hash_at(
            store: &Accessor<U, Self>,
            id: NamedId,
            fd: Resource<Descriptor>,
            path_flags: PathFlags,
            path: String,
        ) -> FilesystemResult<MetadataHashValue> {
            let result = store.with(|mut s| {
                let ctx = s.get().0.filesystem(id);
                ctx.metadata_hash_at(&fd, path_flags, path)
            });
            result.await
        }
    }

    impl<T> types::HostDescriptor for WasiCtxNamedView<'_, T>
    where
        T: WasiFilesystemNamedView,
    {
        fn drop(&mut self, id: NamedId, fd: Resource<Descriptor>) -> wasmtime::Result<()> {
            super::types::HostDescriptor::drop(&mut self.0.filesystem(id), fd)
        }
    }

    impl<T> preopens::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiFilesystemNamedView,
    {
        fn get_directories(
            &mut self,
            id: NamedId,
        ) -> wasmtime::Result<Vec<(Resource<Descriptor>, String)>> {
            super::preopens::Host::get_directories(&mut self.0.filesystem(id))
        }
    }
}

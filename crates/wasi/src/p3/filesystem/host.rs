use crate::filesystem::{Descriptor, Dir, File, WasiFilesystem, WasiFilesystemCtxView};
use crate::p3::bindings::clocks::system_clock;
use crate::p3::bindings::filesystem::types::{
    self, Advice, DescriptorFlags, DescriptorStat, DescriptorType, DirectoryEntry, ErrorCode,
    Filesize, MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
};
use crate::p3::filesystem::{FilesystemError, FilesystemResult, preopens};
use crate::p3::{DEFAULT_BUFFER_CAPACITY, FallibleIteratorProducer};
use crate::{DirPerms, FilePerms};
use anyhow::Context as _;
use bytes::BytesMut;
use core::pin::Pin;
use core::task::{Context, Poll, ready};
use core::{iter, mem};
use std::io::{self, Cursor};
use std::sync::Arc;
use system_interface::fs::FileIoExt as _;
use tokio::sync::{mpsc, oneshot};
use tokio::task::{JoinHandle, spawn_blocking};
use wasmtime::StoreContextMut;
use wasmtime::component::{
    Access, Accessor, Destination, FutureReader, Resource, ResourceTable, Source, StreamConsumer,
    StreamProducer, StreamReader, StreamResult,
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
    type Buffer = Cursor<BytesMut>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        // Intentionally ignore this as in blocking mode everything is always
        // ready and otherwise spawned blocking work can't be cancelled.
        _finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        if let Some(file) = self.file.as_blocking_file() {
            // Once a blocking file, always a blocking file, so assert as such.
            assert!(self.task.is_none());
            let mut dst = dst.as_direct(store, DEFAULT_BUFFER_CAPACITY);
            let buf = dst.remaining();
            if buf.is_empty() {
                return Poll::Ready(Ok(StreamResult::Completed));
            }
            return match file.read_at(buf, self.offset) {
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
            let mut buf = dst.take_buffer().into_inner();
            buf.resize(DEFAULT_BUFFER_CAPACITY, 0);
            let file = Arc::clone(me.file.as_file());
            let offset = me.offset;
            spawn_blocking(move || {
                file.read_at(&mut buf, offset).map(|n| {
                    buf.truncate(n);
                    buf
                })
            })
        });

        // Await the completion of the read task. Note that this is not a
        // cancellable await point because we can't cancel the other task, so
        // the `finish` parameter is ignored.
        let res = ready!(Pin::new(task).poll(cx)).expect("I/O task should not panic");
        self.task = None;
        match res {
            Ok(buf) if buf.is_empty() => {
                self.close(Ok(()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Ok(buf) => {
                let n = buf.len();
                dst.set_buffer(Cursor::new(buf));
                Poll::Ready(Ok(self.complete_read(n)))
            }
            Err(err) => {
                self.close(Err(err.into()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
        }
    }
}

fn map_dir_entry(
    entry: std::io::Result<cap_std::fs::DirEntry>,
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
        dir: Arc<cap_std::fs::Dir>,
        result: oneshot::Sender<Result<(), ErrorCode>>,
    ) -> ReadDirStream {
        let (tx, rx) = mpsc::channel(1);
        ReadDirStream {
            task: spawn_blocking(move || {
                let entries = dir.entries()?;
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
    fn new_at(file: File, offset: u64, result: oneshot::Sender<Result<(), ErrorCode>>) -> Self {
        Self {
            file,
            location: WriteLocation::Offset(offset),
            result: Some(result),
            buffer: BytesMut::default(),
            task: None,
        }
    }

    fn new_append(file: File, result: oneshot::Sender<Result<(), ErrorCode>>) -> Self {
        Self {
            file,
            location: WriteLocation::End,
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
    fn write(&self, file: &cap_std::fs::File, bytes: &[u8]) -> io::Result<usize> {
        match *self {
            WriteLocation::End => file.append(bytes),
            WriteLocation::Offset(at) => file.write_at(bytes, at),
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
        // Intentionally ignore this as in blocking mode everything is always
        // ready and otherwise spawned blocking work can't be cancelled.
        _finish: bool,
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
            me.buffer.extend_from_slice(src.remaining());
            let buf = mem::take(&mut me.buffer);
            let file = Arc::clone(me.file.as_file());
            let location = me.location;
            spawn_blocking(move || location.write(&file, &buf).map(|n| (buf, n)))
        });
        let res = ready!(Pin::new(task).poll(cx)).expect("I/O task should not panic");
        self.task = None;
        match res {
            Ok((buf, n)) => {
                src.mark_read(n);
                self.buffer = buf;
                self.buffer.clear();
                Poll::Ready(Ok(self.complete_write(n)))
            }
            Err(err) => {
                self.close(Err(err.into()));
                Poll::Ready(Ok(StreamResult::Dropped))
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

impl types::HostDescriptorWithStore for WasiFilesystem {
    fn read_via_stream<U>(
        mut store: Access<U, Self>,
        fd: Resource<Descriptor>,
        offset: Filesize,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
        let file = get_file(store.get().table, &fd)?;
        if !file.perms.contains(FilePerms::READ) {
            return Ok((
                StreamReader::new(&mut store, iter::empty()),
                FutureReader::new(&mut store, async {
                    anyhow::Ok(Err(ErrorCode::NotPermitted))
                }),
            ));
        }

        let file = file.clone();
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
            ),
            FutureReader::new(&mut store, result_rx),
        ))
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
            data.pipe(store, WriteStreamConsumer::new_at(file, offset, result_tx));
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
            data.pipe(store, WriteStreamConsumer::new_append(file, result_tx));
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
        store.with(|mut store| {
            let dir = get_dir(store.get().table, &fd)?;
            if !dir.perms.contains(DirPerms::READ) {
                return Ok((
                    StreamReader::new(&mut store, iter::empty()),
                    FutureReader::new(&mut store, async {
                        anyhow::Ok(Err(ErrorCode::NotPermitted))
                    }),
                ));
            }
            let allow_blocking_current_thread = dir.allow_blocking_current_thread;
            let dir = Arc::clone(dir.as_dir());
            let (result_tx, result_rx) = oneshot::channel();
            let stream = if allow_blocking_current_thread {
                match dir.entries() {
                    Ok(readdir) => StreamReader::new(
                        &mut store,
                        FallibleIteratorProducer::new(
                            readdir.filter_map(|e| map_dir_entry(e).transpose()),
                            result_tx,
                        ),
                    ),
                    Err(e) => {
                        result_tx.send(Err(e.into())).unwrap();
                        StreamReader::new(&mut store, iter::empty())
                    }
                }
            } else {
                StreamReader::new(&mut store, ReadDirStream::new(dir, result_tx))
            };
            Ok((stream, FutureReader::new(&mut store, result_rx)))
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

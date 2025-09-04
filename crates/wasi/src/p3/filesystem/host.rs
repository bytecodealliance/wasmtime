use crate::filesystem::{Descriptor, Dir, File, WasiFilesystem, WasiFilesystemCtxView};
use crate::p3::bindings::clocks::wall_clock;
use crate::p3::bindings::filesystem::types::{
    self, Advice, DescriptorFlags, DescriptorStat, DescriptorType, DirectoryEntry, ErrorCode,
    Filesize, MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
};
use crate::p3::filesystem::{FilesystemError, FilesystemResult, preopens};
use crate::p3::{
    DEFAULT_BUFFER_CAPACITY, FutureOneshotProducer, FutureReadyProducer, StreamEmptyProducer,
};
use crate::{DirPerms, FilePerms};
use anyhow::{Context as _, anyhow};
use bytes::BytesMut;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll, ready};
use std::io::Cursor;
use std::sync::Arc;
use system_interface::fs::FileIoExt as _;
use tokio::sync::{mpsc, oneshot};
use tokio::task::{JoinHandle, spawn_blocking};
use wasmtime::StoreContextMut;
use wasmtime::component::{
    Accessor, Destination, FutureReader, Resource, ResourceTable, Source, StreamConsumer,
    StreamProducer, StreamReader, StreamResult, VecBuffer,
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
}

impl<D> StreamProducer<D> for ReadStreamProducer {
    type Item = u8;
    type Buffer = Cursor<BytesMut>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        dst: &'a mut Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        if let Some(task) = self.task.as_mut() {
            let res = ready!(Pin::new(task).poll(cx));
            self.task = None;
            match res {
                Ok(Ok(buf)) if buf.is_empty() => {
                    self.close(Ok(()));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                }
                Ok(Ok(buf)) => {
                    let n = buf.len();
                    dst.set_buffer(Cursor::new(buf));
                    let Ok(n) = n.try_into() else {
                        self.close(Err(ErrorCode::Overflow));
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    };
                    let Some(n) = self.offset.checked_add(n) else {
                        self.close(Err(ErrorCode::Overflow));
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    };
                    self.offset = n;
                    return Poll::Ready(Ok(StreamResult::Completed));
                }
                Ok(Err(err)) => {
                    self.close(Err(err.into()));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                }
                Err(err) => {
                    return Poll::Ready(Err(anyhow!(err).context("failed to join I/O task")));
                }
            }
        }
        if finish {
            return Poll::Ready(Ok(StreamResult::Cancelled));
        }
        if let Some(file) = self.file.as_blocking_file() {
            if let Some(mut dst) = dst.as_direct_destination(store) {
                let buf = dst.remaining();
                if !buf.is_empty() {
                    match file.read_at(buf, self.offset) {
                        Ok(0) => {
                            self.close(Ok(()));
                            return Poll::Ready(Ok(StreamResult::Dropped));
                        }
                        Ok(n) => {
                            dst.mark_written(n);
                            let Ok(n) = n.try_into() else {
                                self.close(Err(ErrorCode::Overflow));
                                return Poll::Ready(Ok(StreamResult::Dropped));
                            };
                            let Some(n) = self.offset.checked_add(n) else {
                                self.close(Err(ErrorCode::Overflow));
                                return Poll::Ready(Ok(StreamResult::Dropped));
                            };
                            self.offset = n;
                            return Poll::Ready(Ok(StreamResult::Completed));
                        }
                        Err(err) => {
                            self.close(Err(err.into()));
                            return Poll::Ready(Ok(StreamResult::Dropped));
                        }
                    }
                }
            }
            let mut buf = dst.take_buffer().into_inner();
            buf.resize(DEFAULT_BUFFER_CAPACITY, 0);
            match file.read_at(&mut buf, self.offset) {
                Ok(0) => {
                    self.close(Ok(()));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                }
                Ok(n) => {
                    buf.truncate(n);
                    dst.set_buffer(Cursor::new(buf));
                    let Ok(n) = n.try_into() else {
                        self.close(Err(ErrorCode::Overflow));
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    };
                    let Some(n) = self.offset.checked_add(n) else {
                        self.close(Err(ErrorCode::Overflow));
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    };
                    self.offset = n;
                    return Poll::Ready(Ok(StreamResult::Completed));
                }
                Err(err) => {
                    self.close(Err(err.into()));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                }
            }
        }
        let mut buf = dst.take_buffer().into_inner();
        buf.resize(DEFAULT_BUFFER_CAPACITY, 0);
        let file = Arc::clone(self.file.as_file());
        let offset = self.offset;
        let mut task = spawn_blocking(move || {
            file.read_at(&mut buf, offset).map(|n| {
                buf.truncate(n);
                buf
            })
        });
        let res = match Pin::new(&mut task).poll(cx) {
            Poll::Ready(res) => res,
            Poll::Pending => {
                self.task = Some(task);
                return Poll::Pending;
            }
        };
        match res {
            Ok(Ok(buf)) if buf.is_empty() => {
                self.close(Ok(()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Ok(Ok(buf)) => {
                let n = buf.len();
                dst.set_buffer(Cursor::new(buf));
                let Ok(n) = n.try_into() else {
                    self.close(Err(ErrorCode::Overflow));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                };
                let Some(n) = self.offset.checked_add(n) else {
                    self.close(Err(ErrorCode::Overflow));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                };
                self.offset = n;
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Ok(Err(err)) => {
                self.close(Err(err.into()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Err(err) => Poll::Ready(Err(anyhow!(err).context("failed to join I/O task"))),
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

struct BlockingDirectoryStreamProducer {
    dir: Arc<cap_std::fs::Dir>,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
}

impl Drop for BlockingDirectoryStreamProducer {
    fn drop(&mut self) {
        self.close(Ok(()))
    }
}

impl BlockingDirectoryStreamProducer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let Some(tx) = self.result.take() {
            _ = tx.send(res);
        }
    }
}

impl<D> StreamProducer<D> for BlockingDirectoryStreamProducer {
    type Item = DirectoryEntry;
    type Buffer = VecBuffer<DirectoryEntry>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: StoreContextMut<'a, D>,
        dst: &'a mut Destination<'a, Self::Item, Self::Buffer>,
        _finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let entries = match self.dir.entries() {
            Ok(entries) => entries,
            Err(err) => {
                self.close(Err(err.into()));
                return Poll::Ready(Ok(StreamResult::Dropped));
            }
        };
        let res = match entries
            .filter_map(|entry| map_dir_entry(entry).transpose())
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(entries) => {
                dst.set_buffer(entries.into());
                Ok(())
            }
            Err(err) => Err(err),
        };
        self.close(res);
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

struct NonblockingDirectoryStreamProducer(DirStreamState);

enum DirStreamState {
    Init {
        dir: Arc<cap_std::fs::Dir>,
        result: oneshot::Sender<Result<(), ErrorCode>>,
    },
    InProgress {
        rx: mpsc::Receiver<DirectoryEntry>,
        task: JoinHandle<Result<(), ErrorCode>>,
        result: oneshot::Sender<Result<(), ErrorCode>>,
    },
    Closed,
}

impl Drop for NonblockingDirectoryStreamProducer {
    fn drop(&mut self) {
        self.close(Ok(()))
    }
}

impl NonblockingDirectoryStreamProducer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let DirStreamState::Init { result, .. } | DirStreamState::InProgress { result, .. } =
            mem::replace(&mut self.0, DirStreamState::Closed)
        {
            _ = result.send(res);
        }
    }
}

impl<D> StreamProducer<D> for NonblockingDirectoryStreamProducer {
    type Item = DirectoryEntry;
    type Buffer = Option<DirectoryEntry>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        dst: &'a mut Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        match mem::replace(&mut self.0, DirStreamState::Closed) {
            DirStreamState::Init { .. } if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            DirStreamState::Init { dir, result } => {
                let (entry_tx, entry_rx) = mpsc::channel(1);
                let task = spawn_blocking(move || {
                    let entries = dir.entries()?;
                    for entry in entries {
                        if let Some(entry) = map_dir_entry(entry)? {
                            if let Err(_) = entry_tx.blocking_send(entry) {
                                break;
                            }
                        }
                    }
                    Ok(())
                });
                self.0 = DirStreamState::InProgress {
                    rx: entry_rx,
                    task,
                    result,
                };
                self.poll_produce(cx, store, dst, finish)
            }
            DirStreamState::InProgress {
                mut rx,
                mut task,
                result,
            } => {
                let Poll::Ready(res) = rx.poll_recv(cx) else {
                    self.0 = DirStreamState::InProgress { rx, task, result };
                    if finish {
                        return Poll::Ready(Ok(StreamResult::Cancelled));
                    }
                    return Poll::Pending;
                };
                match res {
                    Some(entry) => {
                        self.0 = DirStreamState::InProgress { rx, task, result };
                        dst.set_buffer(Some(entry));
                        Poll::Ready(Ok(StreamResult::Completed))
                    }
                    None => {
                        let res = ready!(Pin::new(&mut task).poll(cx))
                            .context("failed to join I/O task")?;
                        self.0 = DirStreamState::InProgress { rx, task, result };
                        self.close(res);
                        Poll::Ready(Ok(StreamResult::Dropped))
                    }
                }
            }
            DirStreamState::Closed => Poll::Ready(Ok(StreamResult::Dropped)),
        }
    }
}

struct WriteStreamConsumer {
    file: File,
    offset: u64,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
    buffer: BytesMut,
    task: Option<JoinHandle<std::io::Result<(BytesMut, usize)>>>,
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
}

impl<D> StreamConsumer<D> for WriteStreamConsumer {
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: &mut Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let mut src = src.as_direct_source(store);
        if let Some(task) = self.task.as_mut() {
            let res = ready!(Pin::new(task).poll(cx));
            self.task = None;
            match res {
                Ok(Ok((buf, n))) => {
                    src.mark_read(n);
                    self.buffer = buf;
                    self.buffer.clear();
                    let Ok(n) = n.try_into() else {
                        self.close(Err(ErrorCode::Overflow));
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    };
                    let Some(n) = self.offset.checked_add(n) else {
                        self.close(Err(ErrorCode::Overflow));
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    };
                    self.offset = n;
                    return Poll::Ready(Ok(StreamResult::Completed));
                }
                Ok(Err(err)) => {
                    self.close(Err(err.into()));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                }
                Err(err) => {
                    return Poll::Ready(Err(anyhow!(err).context("failed to join I/O task")));
                }
            }
        }
        if finish {
            return Poll::Ready(Ok(StreamResult::Cancelled));
        }
        if let Some(file) = self.file.as_blocking_file() {
            match file.write_at(src.remaining(), self.offset) {
                Ok(n) => {
                    src.mark_read(n);
                    let Ok(n) = n.try_into() else {
                        self.close(Err(ErrorCode::Overflow));
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    };
                    let Some(n) = self.offset.checked_add(n) else {
                        self.close(Err(ErrorCode::Overflow));
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    };
                    self.offset = n;
                    return Poll::Ready(Ok(StreamResult::Completed));
                }
                Err(err) => {
                    self.close(Err(err.into()));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                }
            }
        }
        debug_assert!(self.buffer.is_empty());
        self.buffer.extend_from_slice(src.remaining());
        let buf = mem::take(&mut self.buffer);
        let file = Arc::clone(self.file.as_file());
        let offset = self.offset;
        let mut task = spawn_blocking(move || file.write_at(&buf, offset).map(|n| (buf, n)));
        let res = match Pin::new(&mut task).poll(cx) {
            Poll::Ready(res) => res,
            Poll::Pending => {
                self.task = Some(task);
                return Poll::Pending;
            }
        };
        match res {
            Ok(Ok((buf, n))) => {
                src.mark_read(n);
                self.buffer = buf;
                self.buffer.clear();
                let Ok(n) = n.try_into() else {
                    self.close(Err(ErrorCode::Overflow));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                };
                let Some(n) = self.offset.checked_add(n) else {
                    self.close(Err(ErrorCode::Overflow));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                };
                self.offset = n;
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Ok(Err(err)) => {
                self.close(Err(err.into()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Err(err) => Poll::Ready(Err(anyhow!(err).context("failed to join I/O task"))),
        }
    }
}

struct AppendStreamConsumer {
    file: File,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
    buffer: BytesMut,
    task: Option<JoinHandle<std::io::Result<(BytesMut, usize)>>>,
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
}

impl<D> StreamConsumer<D> for AppendStreamConsumer {
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: &mut Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let mut src = src.as_direct_source(store);
        if let Some(task) = self.task.as_mut() {
            let res = ready!(Pin::new(task).poll(cx));
            self.task = None;
            match res {
                Ok(Ok((buf, n))) => {
                    src.mark_read(n);
                    self.buffer = buf;
                    self.buffer.clear();
                    return Poll::Ready(Ok(StreamResult::Completed));
                }
                Ok(Err(err)) => {
                    self.close(Err(err.into()));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                }
                Err(err) => {
                    return Poll::Ready(Err(anyhow!(err).context("failed to join I/O task")));
                }
            }
        }
        if finish {
            return Poll::Ready(Ok(StreamResult::Cancelled));
        }
        if let Some(file) = self.file.as_blocking_file() {
            match file.append(src.remaining()) {
                Ok(n) => {
                    src.mark_read(n);
                    return Poll::Ready(Ok(StreamResult::Completed));
                }
                Err(err) => {
                    self.close(Err(err.into()));
                    return Poll::Ready(Ok(StreamResult::Dropped));
                }
            }
        }
        debug_assert!(self.buffer.is_empty());
        self.buffer.extend_from_slice(src.remaining());
        let buf = mem::take(&mut self.buffer);
        let file = Arc::clone(self.file.as_file());
        let mut task = spawn_blocking(move || file.append(&buf).map(|n| (buf, n)));
        let res = match Pin::new(&mut task).poll(cx) {
            Poll::Ready(res) => res,
            Poll::Pending => {
                self.task = Some(task);
                return Poll::Pending;
            }
        };
        match res {
            Ok(Ok((buf, n))) => {
                src.mark_read(n);
                self.buffer = buf;
                self.buffer.clear();
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Ok(Err(err)) => {
                self.close(Err(err.into()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Err(err) => Poll::Ready(Err(anyhow!(err).context("failed to join I/O task"))),
        }
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
                    StreamReader::new(instance, &mut store, StreamEmptyProducer::default()),
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
                        task: None,
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
                    task: None,
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
                    task: None,
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
                    StreamReader::new(instance, &mut store, StreamEmptyProducer::default()),
                    FutureReader::new(
                        instance,
                        &mut store,
                        FutureReadyProducer(Err(ErrorCode::NotPermitted)),
                    ),
                ));
            }
            let allow_blocking_current_thread = dir.allow_blocking_current_thread;
            let dir = Arc::clone(dir.as_dir());
            let (result_tx, result_rx) = oneshot::channel();
            let stream = if allow_blocking_current_thread {
                StreamReader::new(
                    instance,
                    &mut store,
                    BlockingDirectoryStreamProducer {
                        dir,
                        result: Some(result_tx),
                    },
                )
            } else {
                StreamReader::new(
                    instance,
                    &mut store,
                    NonblockingDirectoryStreamProducer(DirStreamState::Init {
                        dir,
                        result: result_tx,
                    }),
                )
            };
            Ok((
                stream,
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

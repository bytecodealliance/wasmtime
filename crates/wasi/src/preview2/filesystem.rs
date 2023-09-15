use crate::preview2::bindings::filesystem::types::Descriptor;
use crate::preview2::{
    AbortOnDropJoinHandle, HostOutputStream, OutputStreamError, StreamRuntimeError, StreamState,
    Table, TableError,
};
use anyhow::anyhow;
use bytes::{Bytes, BytesMut};
use futures::future::{maybe_done, MaybeDone};
use std::sync::Arc;
use wasmtime::component::Resource;

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct FilePerms: usize {
        const READ = 0b1;
        const WRITE = 0b10;
    }
}

pub(crate) struct File {
    /// Wrapped in an Arc because the same underlying file is used for
    /// implementing the stream types. Also needed for [`spawn_blocking`].
    ///
    /// [`spawn_blocking`]: Self::spawn_blocking
    pub file: Arc<cap_std::fs::File>,
    pub perms: FilePerms,
}

impl File {
    pub fn new(file: cap_std::fs::File, perms: FilePerms) -> Self {
        Self {
            file: Arc::new(file),
            perms,
        }
    }

    /// Spawn a task on tokio's blocking thread for performing blocking
    /// syscalls on the underlying [`cap_std::fs::File`].
    pub(crate) async fn spawn_blocking<F, R>(&self, body: F) -> R
    where
        F: FnOnce(&cap_std::fs::File) -> R + Send + 'static,
        R: Send + 'static,
    {
        let f = self.file.clone();
        tokio::task::spawn_blocking(move || body(&f)).await.unwrap()
    }
}
pub(crate) trait TableFsExt {
    fn push_file(&mut self, file: File) -> Result<Resource<Descriptor>, TableError>;
    fn delete_file(&mut self, fd: Resource<Descriptor>) -> Result<File, TableError>;
    fn is_file(&self, fd: &Resource<Descriptor>) -> bool;
    fn get_file(&self, fd: &Resource<Descriptor>) -> Result<&File, TableError>;

    fn push_dir(&mut self, dir: Dir) -> Result<Resource<Descriptor>, TableError>;
    fn delete_dir(&mut self, fd: Resource<Descriptor>) -> Result<Dir, TableError>;
    fn is_dir(&self, fd: &Resource<Descriptor>) -> bool;
    fn get_dir(&self, fd: &Resource<Descriptor>) -> Result<&Dir, TableError>;
}

impl TableFsExt for Table {
    fn push_file(&mut self, file: File) -> Result<Resource<Descriptor>, TableError> {
        Ok(Resource::new_own(self.push(Box::new(file))?))
    }
    fn delete_file(&mut self, fd: Resource<Descriptor>) -> Result<File, TableError> {
        self.delete(fd.rep())
    }
    fn is_file(&self, fd: &Resource<Descriptor>) -> bool {
        self.is::<File>(fd.rep())
    }
    fn get_file(&self, fd: &Resource<Descriptor>) -> Result<&File, TableError> {
        self.get(fd.rep())
    }

    fn push_dir(&mut self, dir: Dir) -> Result<Resource<Descriptor>, TableError> {
        Ok(Resource::new_own(self.push(Box::new(dir))?))
    }
    fn delete_dir(&mut self, fd: Resource<Descriptor>) -> Result<Dir, TableError> {
        self.delete(fd.rep())
    }
    fn is_dir(&self, fd: &Resource<Descriptor>) -> bool {
        self.is::<Dir>(fd.rep())
    }
    fn get_dir(&self, fd: &Resource<Descriptor>) -> Result<&Dir, TableError> {
        self.get(fd.rep())
    }
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct DirPerms: usize {
        const READ = 0b1;
        const MUTATE = 0b10;
    }
}

pub(crate) struct Dir {
    pub dir: Arc<cap_std::fs::Dir>,
    pub perms: DirPerms,
    pub file_perms: FilePerms,
}

impl Dir {
    pub fn new(dir: cap_std::fs::Dir, perms: DirPerms, file_perms: FilePerms) -> Self {
        Dir {
            dir: Arc::new(dir),
            perms,
            file_perms,
        }
    }

    /// Spawn a task on tokio's blocking thread for performing blocking
    /// syscalls on the underlying [`cap_std::fs::Dir`].
    pub(crate) async fn spawn_blocking<F, R>(&self, body: F) -> R
    where
        F: FnOnce(&cap_std::fs::Dir) -> R + Send + 'static,
        R: Send + 'static,
    {
        let d = self.dir.clone();
        tokio::task::spawn_blocking(move || body(&d)).await.unwrap()
    }
}

pub(crate) struct FileInputStream {
    file: Arc<cap_std::fs::File>,
    position: u64,
}
impl FileInputStream {
    pub fn new(file: Arc<cap_std::fs::File>, position: u64) -> Self {
        Self { file, position }
    }

    pub async fn read(&mut self, size: usize) -> anyhow::Result<(Bytes, StreamState)> {
        use system_interface::fs::FileIoExt;
        let f = Arc::clone(&self.file);
        let p = self.position;
        let (r, mut buf) = tokio::task::spawn_blocking(move || {
            let mut buf = BytesMut::zeroed(size);
            let r = f.read_at(&mut buf, p);
            (r, buf)
        })
        .await
        .unwrap();
        let (n, state) = read_result(r)?;
        buf.truncate(n);
        self.position += n as u64;
        Ok((buf.freeze(), state))
    }

    pub async fn skip(&mut self, nelem: usize) -> anyhow::Result<(usize, StreamState)> {
        let mut nread = 0;
        let mut state = StreamState::Open;

        let (bs, read_state) = self.read(nelem).await?;
        // TODO: handle the case where `bs.len()` is less than `nelem`
        nread += bs.len();
        if read_state.is_closed() {
            state = read_state;
        }

        Ok((nread, state))
    }
}

fn read_result(r: Result<usize, std::io::Error>) -> Result<(usize, StreamState), anyhow::Error> {
    match r {
        Ok(0) => Ok((0, StreamState::Closed)),
        Ok(n) => Ok((n, StreamState::Open)),
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Ok((0, StreamState::Open)),
        Err(e) => Err(StreamRuntimeError::from(anyhow!(e)).into()),
    }
}

#[derive(Clone, Copy)]
pub(crate) enum FileOutputMode {
    Position(u64),
    Append,
}

pub(crate) struct FileOutputStream {
    file: Arc<cap_std::fs::File>,
    mode: FileOutputMode,
    // Allows join future to be awaited in a cancellable manner. Gone variant indicates
    // no task is currently outstanding.
    task: MaybeDone<AbortOnDropJoinHandle<Result<(), std::io::Error>>>,
    closed: bool,
}
impl FileOutputStream {
    pub fn write_at(file: Arc<cap_std::fs::File>, position: u64) -> Self {
        Self {
            file,
            mode: FileOutputMode::Position(position),
            task: MaybeDone::Gone,
            closed: false,
        }
    }
    pub fn append(file: Arc<cap_std::fs::File>) -> Self {
        Self {
            file,
            mode: FileOutputMode::Append,
            task: MaybeDone::Gone,
            closed: false,
        }
    }
}

// FIXME: configurable? determine from how much space left in file?
const FILE_WRITE_CAPACITY: usize = 1024 * 1024;

#[async_trait::async_trait]
impl HostOutputStream for FileOutputStream {
    fn write(&mut self, buf: Bytes) -> Result<(), OutputStreamError> {
        use system_interface::fs::FileIoExt;

        if self.closed {
            return Err(OutputStreamError::Closed);
        }
        if !matches!(self.task, MaybeDone::Gone) {
            // a write is pending - this call was not permitted
            return Err(OutputStreamError::Trap(anyhow!(
                "write not permitted: FileOutputStream write pending"
            )));
        }
        let f = Arc::clone(&self.file);
        let m = self.mode;
        self.task = maybe_done(AbortOnDropJoinHandle::from(tokio::task::spawn_blocking(
            move || match m {
                FileOutputMode::Position(mut p) => {
                    let mut buf = buf;
                    while !buf.is_empty() {
                        let nwritten = f.write_at(buf.as_ref(), p)?;
                        // afterwards buf contains [nwritten, len):
                        let _ = buf.split_to(nwritten);
                        p += nwritten as u64;
                    }
                    Ok(())
                }
                FileOutputMode::Append => {
                    let mut buf = buf;
                    while !buf.is_empty() {
                        let nwritten = f.append(buf.as_ref())?;
                        let _ = buf.split_to(nwritten);
                    }
                    Ok(())
                }
            },
        )));
        Ok(())
    }
    fn flush(&mut self) -> Result<(), OutputStreamError> {
        if self.closed {
            return Err(OutputStreamError::Closed);
        }
        // Only userland buffering of file writes is in the blocking task.
        Ok(())
    }
    async fn write_ready(&mut self) -> Result<usize, OutputStreamError> {
        if self.closed {
            return Err(OutputStreamError::Closed);
        }
        // If there is no outstanding task, accept more input:
        if matches!(self.task, MaybeDone::Gone) {
            return Ok(FILE_WRITE_CAPACITY);
        }
        // Wait for outstanding task:
        std::pin::Pin::new(&mut self.task).await;

        // Mark task as finished, and handle output:
        match std::pin::Pin::new(&mut self.task)
            .take_output()
            .expect("just awaited for MaybeDone completion")
        {
            Ok(()) => Ok(FILE_WRITE_CAPACITY),
            Err(e) => {
                self.closed = true;
                Err(OutputStreamError::LastOperationFailed(e.into()))
            }
        }
    }
}

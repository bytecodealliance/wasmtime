use crate::bindings::filesystem::types;
use crate::{
    spawn_blocking, AbortOnDropJoinHandle, HostOutputStream, StreamError, Subscribe, TrappableError,
};
use anyhow::anyhow;
use bytes::{Bytes, BytesMut};
use std::io;
use std::mem;
use std::sync::Arc;

pub type FsResult<T> = Result<T, FsError>;

pub type FsError = TrappableError<types::ErrorCode>;

impl From<wasmtime::component::ResourceTableError> for FsError {
    fn from(error: wasmtime::component::ResourceTableError) -> Self {
        Self::trap(error)
    }
}

impl From<io::Error> for FsError {
    fn from(error: io::Error) -> Self {
        types::ErrorCode::from(error).into()
    }
}

pub enum Descriptor {
    File(File),
    Dir(Dir),
}

impl Descriptor {
    pub fn file(&self) -> Result<&File, types::ErrorCode> {
        match self {
            Descriptor::File(f) => Ok(f),
            Descriptor::Dir(_) => Err(types::ErrorCode::BadDescriptor),
        }
    }

    pub fn dir(&self) -> Result<&Dir, types::ErrorCode> {
        match self {
            Descriptor::Dir(d) => Ok(d),
            Descriptor::File(_) => Err(types::ErrorCode::NotDirectory),
        }
    }

    pub fn is_file(&self) -> bool {
        match self {
            Descriptor::File(_) => true,
            Descriptor::Dir(_) => false,
        }
    }

    pub fn is_dir(&self) -> bool {
        match self {
            Descriptor::File(_) => false,
            Descriptor::Dir(_) => true,
        }
    }
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct FilePerms: usize {
        const READ = 0b1;
        const WRITE = 0b10;
    }
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct OpenMode: usize {
        const READ = 0b1;
        const WRITE = 0b10;
    }
}

pub struct File {
    /// The operating system File this struct is mediating access to.
    ///
    /// Wrapped in an Arc because the same underlying file is used for
    /// implementing the stream types. A copy is also needed for
    /// [`spawn_blocking`].
    ///
    /// [`spawn_blocking`]: Self::spawn_blocking
    pub file: Arc<cap_std::fs::File>,
    /// Permissions to enforce on access to the file. These permissions are
    /// specified by a user of the `crate::WasiCtxBuilder`, and are
    /// enforced prior to any enforced by the underlying operating system.
    pub perms: FilePerms,
    /// The mode the file was opened under: bits for reading, and writing.
    /// Required to correctly report the DescriptorFlags, because cap-std
    /// doesn't presently provide a cross-platform equivelant of reading the
    /// oflags back out using fcntl.
    pub open_mode: OpenMode,
}

impl File {
    pub fn new(file: cap_std::fs::File, perms: FilePerms, open_mode: OpenMode) -> Self {
        Self {
            file: Arc::new(file),
            perms,
            open_mode,
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
        spawn_blocking(move || body(&f)).await
    }
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct DirPerms: usize {
        const READ = 0b1;
        const MUTATE = 0b10;
    }
}

#[derive(Clone)]
pub struct Dir {
    /// The operating system file descriptor this struct is mediating access
    /// to.
    ///
    /// Wrapped in an Arc because a copy is needed for [`spawn_blocking`].
    ///
    /// [`spawn_blocking`]: Self::spawn_blocking
    pub dir: Arc<cap_std::fs::Dir>,
    /// Permissions to enforce on access to this directory. These permissions
    /// are specified by a user of the `crate::WasiCtxBuilder`, and
    /// are enforced prior to any enforced by the underlying operating system.
    ///
    /// These permissions are also enforced on any directories opened under
    /// this directory.
    pub perms: DirPerms,
    /// Permissions to enforce on any files opened under this directory.
    pub file_perms: FilePerms,
    /// The mode the directory was opened under: bits for reading, and writing.
    /// Required to correctly report the DescriptorFlags, because cap-std
    /// doesn't presently provide a cross-platform equivelant of reading the
    /// oflags back out using fcntl.
    pub open_mode: OpenMode,
}

impl Dir {
    pub fn new(
        dir: cap_std::fs::Dir,
        perms: DirPerms,
        file_perms: FilePerms,
        open_mode: OpenMode,
    ) -> Self {
        Dir {
            dir: Arc::new(dir),
            perms,
            file_perms,
            open_mode,
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
        spawn_blocking(move || body(&d)).await
    }
}

pub struct FileInputStream {
    file: Arc<cap_std::fs::File>,
    position: u64,
}
impl FileInputStream {
    pub fn new(file: Arc<cap_std::fs::File>, position: u64) -> Self {
        Self { file, position }
    }

    pub async fn read(&mut self, size: usize) -> Result<Bytes, StreamError> {
        use system_interface::fs::FileIoExt;
        let f = Arc::clone(&self.file);
        let p = self.position;
        let (r, mut buf) = spawn_blocking(move || {
            let mut buf = BytesMut::zeroed(size);
            let r = f.read_at(&mut buf, p);
            (r, buf)
        })
        .await;
        let n = read_result(r)?;
        buf.truncate(n);
        self.position += n as u64;
        Ok(buf.freeze())
    }

    pub async fn skip(&mut self, nelem: usize) -> Result<usize, StreamError> {
        let bs = self.read(nelem).await?;
        Ok(bs.len())
    }
}

fn read_result(r: io::Result<usize>) -> Result<usize, StreamError> {
    match r {
        Ok(0) => Err(StreamError::Closed),
        Ok(n) => Ok(n),
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Ok(0),
        Err(e) => Err(StreamError::LastOperationFailed(e.into())),
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
    state: OutputState,
}

enum OutputState {
    Ready,
    /// Allows join future to be awaited in a cancellable manner. Gone variant indicates
    /// no task is currently outstanding.
    Waiting(AbortOnDropJoinHandle<io::Result<usize>>),
    /// The last I/O operation failed with this error.
    Error(io::Error),
    Closed,
}

impl FileOutputStream {
    pub fn write_at(file: Arc<cap_std::fs::File>, position: u64) -> Self {
        Self {
            file,
            mode: FileOutputMode::Position(position),
            state: OutputState::Ready,
        }
    }
    pub fn append(file: Arc<cap_std::fs::File>) -> Self {
        Self {
            file,
            mode: FileOutputMode::Append,
            state: OutputState::Ready,
        }
    }
}

// FIXME: configurable? determine from how much space left in file?
const FILE_WRITE_CAPACITY: usize = 1024 * 1024;

impl HostOutputStream for FileOutputStream {
    fn write(&mut self, buf: Bytes) -> Result<(), StreamError> {
        use system_interface::fs::FileIoExt;
        match self.state {
            OutputState::Ready => {}
            OutputState::Closed => return Err(StreamError::Closed),
            OutputState::Waiting(_) | OutputState::Error(_) => {
                // a write is pending - this call was not permitted
                return Err(StreamError::Trap(anyhow!(
                    "write not permitted: check_write not called first"
                )));
            }
        }

        if buf.is_empty() {
            return Ok(());
        }

        let f = Arc::clone(&self.file);
        let m = self.mode;
        let task = spawn_blocking(move || match m {
            FileOutputMode::Position(mut p) => {
                let mut total = 0;
                let mut buf = buf;
                while !buf.is_empty() {
                    let nwritten = f.write_at(buf.as_ref(), p)?;
                    // afterwards buf contains [nwritten, len):
                    let _ = buf.split_to(nwritten);
                    p += nwritten as u64;
                    total += nwritten;
                }
                Ok(total)
            }
            FileOutputMode::Append => {
                let mut total = 0;
                let mut buf = buf;
                while !buf.is_empty() {
                    let nwritten = f.append(buf.as_ref())?;
                    let _ = buf.split_to(nwritten);
                    total += nwritten;
                }
                Ok(total)
            }
        });
        self.state = OutputState::Waiting(task);
        Ok(())
    }
    fn flush(&mut self) -> Result<(), StreamError> {
        match self.state {
            // Only userland buffering of file writes is in the blocking task,
            // so there's nothing extra that needs to be done to request a
            // flush.
            OutputState::Ready | OutputState::Waiting(_) => Ok(()),
            OutputState::Closed => Err(StreamError::Closed),
            OutputState::Error(_) => match mem::replace(&mut self.state, OutputState::Closed) {
                OutputState::Error(e) => Err(StreamError::LastOperationFailed(e.into())),
                _ => unreachable!(),
            },
        }
    }
    fn check_write(&mut self) -> Result<usize, StreamError> {
        match self.state {
            OutputState::Ready => Ok(FILE_WRITE_CAPACITY),
            OutputState::Closed => Err(StreamError::Closed),
            OutputState::Error(_) => match mem::replace(&mut self.state, OutputState::Closed) {
                OutputState::Error(e) => Err(StreamError::LastOperationFailed(e.into())),
                _ => unreachable!(),
            },
            OutputState::Waiting(_) => Ok(0),
        }
    }
}

#[async_trait::async_trait]
impl Subscribe for FileOutputStream {
    async fn ready(&mut self) {
        if let OutputState::Waiting(task) = &mut self.state {
            self.state = match task.await {
                Ok(nwritten) => {
                    if let FileOutputMode::Position(ref mut p) = &mut self.mode {
                        *p += nwritten as u64;
                    }
                    OutputState::Ready
                }
                Err(e) => OutputState::Error(e),
            };
        }
    }
}

pub struct ReaddirIterator(
    std::sync::Mutex<Box<dyn Iterator<Item = FsResult<types::DirectoryEntry>> + Send + 'static>>,
);

impl ReaddirIterator {
    pub(crate) fn new(
        i: impl Iterator<Item = FsResult<types::DirectoryEntry>> + Send + 'static,
    ) -> Self {
        ReaddirIterator(std::sync::Mutex::new(Box::new(i)))
    }
    pub(crate) fn next(&self) -> FsResult<Option<types::DirectoryEntry>> {
        self.0.lock().unwrap().next().transpose()
    }
}

impl IntoIterator for ReaddirIterator {
    type Item = FsResult<types::DirectoryEntry>;
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + Send>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_inner().unwrap()
    }
}

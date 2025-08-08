use crate::TrappableError;
use crate::filesystem::File;
use crate::p2::bindings::filesystem::types;
use crate::p2::{InputStream, OutputStream, Pollable, StreamError, StreamResult};
use crate::runtime::AbortOnDropJoinHandle;
use anyhow::anyhow;
use bytes::{Bytes, BytesMut};
use std::io;
use std::mem;

pub type FsResult<T> = Result<T, FsError>;

pub type FsError = TrappableError<types::ErrorCode>;

impl From<crate::filesystem::ErrorCode> for types::ErrorCode {
    fn from(error: crate::filesystem::ErrorCode) -> Self {
        match error {
            crate::filesystem::ErrorCode::Access => Self::Access,
            crate::filesystem::ErrorCode::Already => Self::Already,
            crate::filesystem::ErrorCode::BadDescriptor => Self::BadDescriptor,
            crate::filesystem::ErrorCode::Busy => Self::Busy,
            crate::filesystem::ErrorCode::Exist => Self::Exist,
            crate::filesystem::ErrorCode::FileTooLarge => Self::FileTooLarge,
            crate::filesystem::ErrorCode::IllegalByteSequence => Self::IllegalByteSequence,
            crate::filesystem::ErrorCode::InProgress => Self::InProgress,
            crate::filesystem::ErrorCode::Interrupted => Self::Interrupted,
            crate::filesystem::ErrorCode::Invalid => Self::Invalid,
            crate::filesystem::ErrorCode::Io => Self::Io,
            crate::filesystem::ErrorCode::IsDirectory => Self::IsDirectory,
            crate::filesystem::ErrorCode::Loop => Self::Loop,
            crate::filesystem::ErrorCode::TooManyLinks => Self::TooManyLinks,
            crate::filesystem::ErrorCode::NameTooLong => Self::NameTooLong,
            crate::filesystem::ErrorCode::NoEntry => Self::NoEntry,
            crate::filesystem::ErrorCode::InsufficientMemory => Self::InsufficientMemory,
            crate::filesystem::ErrorCode::InsufficientSpace => Self::InsufficientSpace,
            crate::filesystem::ErrorCode::NotDirectory => Self::NotDirectory,
            crate::filesystem::ErrorCode::NotEmpty => Self::NotEmpty,
            crate::filesystem::ErrorCode::Unsupported => Self::Unsupported,
            crate::filesystem::ErrorCode::Overflow => Self::Overflow,
            crate::filesystem::ErrorCode::NotPermitted => Self::NotPermitted,
            crate::filesystem::ErrorCode::Pipe => Self::Pipe,
            crate::filesystem::ErrorCode::InvalidSeek => Self::InvalidSeek,
        }
    }
}

impl From<crate::filesystem::ErrorCode> for FsError {
    fn from(error: crate::filesystem::ErrorCode) -> Self {
        types::ErrorCode::from(error).into()
    }
}

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

pub struct FileInputStream {
    file: File,
    position: u64,
    state: ReadState,
}
enum ReadState {
    Idle,
    Waiting(AbortOnDropJoinHandle<ReadState>),
    DataAvailable(Bytes),
    Error(io::Error),
    Closed,
}
impl FileInputStream {
    pub fn new(file: &File, position: u64) -> Self {
        Self {
            file: file.clone(),
            position,
            state: ReadState::Idle,
        }
    }

    fn blocking_read(file: &cap_std::fs::File, offset: u64, size: usize) -> ReadState {
        use system_interface::fs::FileIoExt;

        let mut buf = BytesMut::zeroed(size);
        loop {
            match file.read_at(&mut buf, offset) {
                Ok(0) => return ReadState::Closed,
                Ok(n) => {
                    buf.truncate(n);
                    return ReadState::DataAvailable(buf.freeze());
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                    // Try again, continue looping
                }
                Err(e) => return ReadState::Error(e),
            }
        }
    }

    /// Wait for existing background task to finish, without starting any new background reads.
    async fn wait_ready(&mut self) {
        match &mut self.state {
            ReadState::Waiting(task) => {
                self.state = task.await;
            }
            _ => {}
        }
    }
}
#[async_trait::async_trait]
impl InputStream for FileInputStream {
    fn read(&mut self, size: usize) -> StreamResult<Bytes> {
        match &mut self.state {
            ReadState::Idle => {
                if size == 0 {
                    return Ok(Bytes::new());
                }

                let p = self.position;
                self.state = ReadState::Waiting(
                    self.file
                        .spawn_blocking(move |f| Self::blocking_read(f, p, size)),
                );
                Ok(Bytes::new())
            }
            ReadState::DataAvailable(b) => {
                let min_len = b.len().min(size);
                let chunk = b.split_to(min_len);
                if b.len() == 0 {
                    self.state = ReadState::Idle;
                }
                self.position += min_len as u64;
                Ok(chunk)
            }
            ReadState::Waiting(_) => Ok(Bytes::new()),
            ReadState::Error(_) => match mem::replace(&mut self.state, ReadState::Closed) {
                ReadState::Error(e) => Err(StreamError::LastOperationFailed(e.into())),
                _ => unreachable!(),
            },
            ReadState::Closed => Err(StreamError::Closed),
        }
    }
    /// Specialized blocking_* variant to bypass tokio's task spawning & joining
    /// overhead on synchronous file I/O.
    async fn blocking_read(&mut self, size: usize) -> StreamResult<Bytes> {
        self.wait_ready().await;

        // Before we defer to the regular `read`, make sure it has data ready to go:
        if let ReadState::Idle = self.state {
            let p = self.position;
            self.state = self
                .file
                .run_blocking(move |f| Self::blocking_read(f, p, size))
                .await;
        }

        self.read(size)
    }
    async fn cancel(&mut self) {
        match mem::replace(&mut self.state, ReadState::Closed) {
            ReadState::Waiting(task) => {
                // The task was created using `spawn_blocking`, so unless we're
                // lucky enough that the task hasn't started yet, the abort
                // signal won't have any effect and we're forced to wait for it
                // to run to completion.
                // From the guest's point of view, `input-stream::drop` then
                // appears to block. Certainly less than ideal, but arguably still
                // better than letting the guest rack up an unbounded number of
                // background tasks. Also, the guest is only blocked if
                // the stream was dropped mid-read, which we don't expect to
                // occur frequently.
                task.cancel().await;
            }
            _ => {}
        }
    }
}
#[async_trait::async_trait]
impl Pollable for FileInputStream {
    async fn ready(&mut self) {
        if let ReadState::Idle = self.state {
            // The guest hasn't initiated any read, but is nonetheless waiting
            // for data to be available. We'll start a read for them:

            const DEFAULT_READ_SIZE: usize = 4096;
            let p = self.position;
            self.state = ReadState::Waiting(
                self.file
                    .spawn_blocking(move |f| Self::blocking_read(f, p, DEFAULT_READ_SIZE)),
            );
        }

        self.wait_ready().await
    }
}

#[derive(Clone, Copy)]
pub(crate) enum FileOutputMode {
    Position(u64),
    Append,
}

pub(crate) struct FileOutputStream {
    file: File,
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
    pub fn write_at(file: &File, position: u64) -> Self {
        Self {
            file: file.clone(),
            mode: FileOutputMode::Position(position),
            state: OutputState::Ready,
        }
    }

    pub fn append(file: &File) -> Self {
        Self {
            file: file.clone(),
            mode: FileOutputMode::Append,
            state: OutputState::Ready,
        }
    }

    fn blocking_write(
        file: &cap_std::fs::File,
        mut buf: Bytes,
        mode: FileOutputMode,
    ) -> io::Result<usize> {
        use system_interface::fs::FileIoExt;

        match mode {
            FileOutputMode::Position(mut p) => {
                let mut total = 0;
                loop {
                    let nwritten = file.write_at(buf.as_ref(), p)?;
                    // afterwards buf contains [nwritten, len):
                    let _ = buf.split_to(nwritten);
                    p += nwritten as u64;
                    total += nwritten;
                    if buf.is_empty() {
                        break;
                    }
                }
                Ok(total)
            }
            FileOutputMode::Append => {
                let mut total = 0;
                loop {
                    let nwritten = file.append(buf.as_ref())?;
                    let _ = buf.split_to(nwritten);
                    total += nwritten;
                    if buf.is_empty() {
                        break;
                    }
                }
                Ok(total)
            }
        }
    }
}

// FIXME: configurable? determine from how much space left in file?
const FILE_WRITE_CAPACITY: usize = 1024 * 1024;

#[async_trait::async_trait]
impl OutputStream for FileOutputStream {
    fn write(&mut self, buf: Bytes) -> Result<(), StreamError> {
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

        let m = self.mode;
        self.state = OutputState::Waiting(
            self.file
                .spawn_blocking(move |f| Self::blocking_write(f, buf, m)),
        );
        Ok(())
    }
    /// Specialized blocking_* variant to bypass tokio's task spawning & joining
    /// overhead on synchronous file I/O.
    async fn blocking_write_and_flush(&mut self, buf: Bytes) -> StreamResult<()> {
        self.ready().await;

        match self.state {
            OutputState::Ready => {}
            OutputState::Closed => return Err(StreamError::Closed),
            OutputState::Error(_) => match mem::replace(&mut self.state, OutputState::Closed) {
                OutputState::Error(e) => return Err(StreamError::LastOperationFailed(e.into())),
                _ => unreachable!(),
            },
            OutputState::Waiting(_) => unreachable!("we've just waited for readiness"),
        }

        let m = self.mode;
        match self
            .file
            .run_blocking(move |f| Self::blocking_write(f, buf, m))
            .await
        {
            Ok(nwritten) => {
                if let FileOutputMode::Position(p) = &mut self.mode {
                    *p += nwritten as u64;
                }
                self.state = OutputState::Ready;
                Ok(())
            }
            Err(e) => {
                self.state = OutputState::Closed;
                Err(StreamError::LastOperationFailed(e.into()))
            }
        }
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
    async fn cancel(&mut self) {
        match mem::replace(&mut self.state, OutputState::Closed) {
            OutputState::Waiting(task) => {
                // The task was created using `spawn_blocking`, so unless we're
                // lucky enough that the task hasn't started yet, the abort
                // signal won't have any effect and we're forced to wait for it
                // to run to completion.
                // From the guest's point of view, `output-stream::drop` then
                // appears to block. Certainly less than ideal, but arguably still
                // better than letting the guest rack up an unbounded number of
                // background tasks. Also, the guest is only blocked if
                // the stream was dropped mid-write, which we don't expect to
                // occur frequently.
                task.cancel().await;
            }
            _ => {}
        }
    }
}

#[async_trait::async_trait]
impl Pollable for FileOutputStream {
    async fn ready(&mut self) {
        if let OutputState::Waiting(task) = &mut self.state {
            self.state = match task.await {
                Ok(nwritten) => {
                    if let FileOutputMode::Position(p) = &mut self.mode {
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

//! Virtual pipes.
//!
//! These types provide easy implementations of `WasiFile` that mimic much of the behavior of Unix
//! pipes. These are particularly helpful for redirecting WASI stdio handles to destinations other
//! than OS files.
//!
//! Some convenience constructors are included for common backing types like `Vec<u8>` and `String`,
//! but the virtual pipes can be instantiated with any `Read` or `Write` type.
//!
use crate::file::{FdFlags, FileType, WasiFile};
use crate::Error;
use std::any::Any;
use std::io::{self, Read, Write};
use std::sync::{Arc, RwLock};

/// A virtual pipe read end.
///
/// A variety of `From` impls are provided so that common pipe types are easy to create. For example:
///
/// ```no_run
/// use wasi_common::{pipe::ReadPipe, WasiCtx, Table};
/// let stdin = ReadPipe::from("hello from stdin!");
/// // Brint these instances from elsewhere (e.g. wasi-cap-std-sync):
/// let random = todo!();
/// let clocks = todo!();
/// let sched = todo!();
/// let table = Table::new();
/// let mut ctx = WasiCtx::new(random, clocks, sched, table);
/// ctx.set_stdin(Box::new(stdin.clone()));
/// ```
#[derive(Debug)]
pub struct ReadPipe<R: Read> {
    reader: Arc<RwLock<R>>,
}

impl<R: Read> Clone for ReadPipe<R> {
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
        }
    }
}

impl<R: Read> ReadPipe<R> {
    /// Create a new pipe from a `Read` type.
    ///
    /// All `Handle` read operations delegate to reading from this underlying reader.
    pub fn new(r: R) -> Self {
        Self::from_shared(Arc::new(RwLock::new(r)))
    }

    /// Create a new pipe from a shareable `Read` type.
    ///
    /// All `Handle` read operations delegate to reading from this underlying reader.
    pub fn from_shared(reader: Arc<RwLock<R>>) -> Self {
        Self { reader }
    }

    /// Try to convert this `ReadPipe<R>` back to the underlying `R` type.
    ///
    /// This will fail with `Err(self)` if multiple references to the underlying `R` exist.
    pub fn try_into_inner(mut self) -> Result<R, Self> {
        match Arc::try_unwrap(self.reader) {
            Ok(rc) => Ok(RwLock::into_inner(rc).unwrap()),
            Err(reader) => {
                self.reader = reader;
                Err(self)
            }
        }
    }
    fn borrow(&self) -> std::sync::RwLockWriteGuard<R> {
        RwLock::write(&self.reader).unwrap()
    }
}

impl From<Vec<u8>> for ReadPipe<io::Cursor<Vec<u8>>> {
    fn from(r: Vec<u8>) -> Self {
        Self::new(io::Cursor::new(r))
    }
}

impl From<&[u8]> for ReadPipe<io::Cursor<Vec<u8>>> {
    fn from(r: &[u8]) -> Self {
        Self::from(r.to_vec())
    }
}

impl From<String> for ReadPipe<io::Cursor<String>> {
    fn from(r: String) -> Self {
        Self::new(io::Cursor::new(r))
    }
}

impl From<&str> for ReadPipe<io::Cursor<String>> {
    fn from(r: &str) -> Self {
        Self::from(r.to_string())
    }
}

#[wiggle::async_trait]
impl<R: Read + Any + Send + Sync> WasiFile for ReadPipe<R> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    async fn get_filetype(&self) -> Result<FileType, Error> {
        Ok(FileType::Pipe)
    }
    async fn read_vectored<'a>(&self, bufs: &mut [io::IoSliceMut<'a>]) -> Result<u64, Error> {
        let n = self.borrow().read_vectored(bufs)?;
        Ok(n.try_into()?)
    }
}

/// A virtual pipe write end.
///
/// ```no_run
/// use wasi_common::{pipe::WritePipe, WasiCtx, Table};
/// let stdout = WritePipe::new_in_memory();
/// // Brint these instances from elsewhere (e.g. wasi-cap-std-sync):
/// let random = todo!();
/// let clocks = todo!();
/// let sched = todo!();
/// let table = Table::new();
/// let mut ctx = WasiCtx::new(random, clocks, sched, table);
/// ctx.set_stdout(Box::new(stdout.clone()));
/// // use ctx in an instance, then make sure it is dropped:
/// drop(ctx);
/// let contents: Vec<u8> = stdout.try_into_inner().expect("sole remaining reference to WritePipe").into_inner();
/// println!("contents of stdout: {:?}", contents);
/// ```
#[derive(Debug)]
pub struct WritePipe<W: Write> {
    writer: Arc<RwLock<W>>,
}

impl<W: Write> Clone for WritePipe<W> {
    fn clone(&self) -> Self {
        Self {
            writer: self.writer.clone(),
        }
    }
}

impl<W: Write> WritePipe<W> {
    /// Create a new pipe from a `Write` type.
    ///
    /// All `Handle` write operations delegate to writing to this underlying writer.
    pub fn new(w: W) -> Self {
        Self::from_shared(Arc::new(RwLock::new(w)))
    }

    /// Create a new pipe from a shareable `Write` type.
    ///
    /// All `Handle` write operations delegate to writing to this underlying writer.
    pub fn from_shared(writer: Arc<RwLock<W>>) -> Self {
        Self { writer }
    }

    /// Try to convert this `WritePipe<W>` back to the underlying `W` type.
    ///
    /// This will fail with `Err(self)` if multiple references to the underlying `W` exist.
    pub fn try_into_inner(mut self) -> Result<W, Self> {
        match Arc::try_unwrap(self.writer) {
            Ok(rc) => Ok(RwLock::into_inner(rc).unwrap()),
            Err(writer) => {
                self.writer = writer;
                Err(self)
            }
        }
    }

    fn borrow(&self) -> std::sync::RwLockWriteGuard<W> {
        RwLock::write(&self.writer).unwrap()
    }
}

impl WritePipe<io::Cursor<Vec<u8>>> {
    /// Create a new writable virtual pipe backed by a `Vec<u8>` buffer.
    pub fn new_in_memory() -> Self {
        Self::new(io::Cursor::new(vec![]))
    }
}

#[wiggle::async_trait]
impl<W: Write + Any + Send + Sync> WasiFile for WritePipe<W> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    async fn get_filetype(&self) -> Result<FileType, Error> {
        Ok(FileType::Pipe)
    }
    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        Ok(FdFlags::APPEND)
    }
    async fn write_vectored<'a>(&self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        let n = self.borrow().write_vectored(bufs)?;
        Ok(n.try_into()?)
    }
}

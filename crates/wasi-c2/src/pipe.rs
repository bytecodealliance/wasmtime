// This is mostly stubs
#![allow(unused_variables, dead_code)]
//! Virtual pipes.
//!
//! These types provide easy implementations of `WasiFile` that mimic much of the behavior of Unix
//! pipes. These are particularly helpful for redirecting WASI stdio handles to destinations other
//! than OS files.
//!
//! Some convenience constructors are included for common backing types like `Vec<u8>` and `String`,
//! but the virtual pipes can be instantiated with any `Read` or `Write` type.
//!
use crate::{
    file::{FdFlags, FileType, Filestat, WasiFile},
    Error, SystemTimeSpec,
};
use std::any::Any;
use std::convert::TryInto;
use std::io::{self, Read, Write};
use std::sync::{Arc, RwLock};
use system_interface::fs::Advice;

/// A virtual pipe read end.
///
/// A variety of `From` impls are provided so that common pipe types are easy to create. For example:
///
/// ```
/// # use wasi_c2::WasiCtx;
/// # use wasi_c2::virt::pipe::ReadPipe;
/// let stdin = ReadPipe::from("hello from stdin!");
/// let ctx = WasiCtx::builder().stdin(Box::new(stdin)).build();
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

impl<R: Read + Any> WasiFile for ReadPipe<R> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn datasync(&self) -> Result<(), Error> {
        Ok(()) // trivial: no implementation needed
    }
    fn sync(&self) -> Result<(), Error> {
        Ok(()) // trivial
    }
    fn get_filetype(&self) -> Result<FileType, Error> {
        Ok(FileType::CharacterDevice) // XXX wrong
    }
    fn get_fdflags(&self) -> Result<FdFlags, Error> {
        Ok(FdFlags::empty())
    }
    unsafe fn reopen_with_fdflags(&self, _fdflags: FdFlags) -> Result<Box<dyn WasiFile>, Error> {
        Err(Error::Badf)
    }
    fn get_filestat(&self) -> Result<Filestat, Error> {
        Ok(Filestat {
            device_id: 0,
            inode: 0,
            filetype: self.get_filetype()?,
            nlink: 0,
            size: 0, // XXX no way to get a size out of a Read :(
            atim: None,
            mtim: None,
            ctim: None,
        })
    }
    fn set_filestat_size(&self, _size: u64) -> Result<(), Error> {
        Err(Error::Perm)
    }
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
        Err(Error::Badf)
    }
    fn allocate(&self, offset: u64, len: u64) -> Result<(), Error> {
        Err(Error::Badf)
    }
    fn read_vectored(&self, bufs: &mut [io::IoSliceMut]) -> Result<u64, Error> {
        let n = self.borrow().read_vectored(bufs)?;
        Ok(n.try_into().map_err(|_| Error::Overflow)?)
    }
    fn read_vectored_at(&self, bufs: &mut [io::IoSliceMut], offset: u64) -> Result<u64, Error> {
        Err(Error::Badf)
    }
    fn write_vectored(&self, bufs: &[io::IoSlice]) -> Result<u64, Error> {
        Err(Error::Badf)
    }
    fn write_vectored_at(&self, bufs: &[io::IoSlice], offset: u64) -> Result<u64, Error> {
        Err(Error::Badf)
    }
    fn seek(&self, pos: std::io::SeekFrom) -> Result<u64, Error> {
        Err(Error::Badf)
    }
    fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
        Err(Error::Badf)
    }
    fn set_times(
        &self,
        atime: Option<SystemTimeSpec>,
        mtime: Option<SystemTimeSpec>,
    ) -> Result<(), Error> {
        Err(Error::Badf)
    }
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(0)
    }
}

/// A virtual pipe write end.
///
/// ```
/// # use wasi_c2::WasiCtx;
/// # use wasi_c2::virt::pipe::WritePipe;
/// let stdout = WritePipe::new_in_memory();
/// let ctx = WasiCtx::builder().stdout(Box::new(stdout.clone())).build();
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

impl<W: Write + Any> WasiFile for WritePipe<W> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn datasync(&self) -> Result<(), Error> {
        Ok(())
    }
    fn sync(&self) -> Result<(), Error> {
        Ok(())
    }
    fn get_filetype(&self) -> Result<FileType, Error> {
        Ok(FileType::CharacterDevice) // XXX
    }
    fn get_fdflags(&self) -> Result<FdFlags, Error> {
        Ok(FdFlags::APPEND)
    }
    unsafe fn reopen_with_fdflags(&self, _fdflags: FdFlags) -> Result<Box<dyn WasiFile>, Error> {
        Err(Error::Badf)
    }
    fn get_filestat(&self) -> Result<Filestat, Error> {
        Ok(Filestat {
            device_id: 0,
            inode: 0,
            filetype: self.get_filetype()?,
            nlink: 0,
            size: 0, // XXX no way to get a size out of a Write :(
            atim: None,
            mtim: None,
            ctim: None,
        })
    }
    fn set_filestat_size(&self, _size: u64) -> Result<(), Error> {
        Err(Error::Perm)
    }
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
        Err(Error::Badf)
    }
    fn allocate(&self, offset: u64, len: u64) -> Result<(), Error> {
        Err(Error::Badf)
    }
    fn read_vectored(&self, bufs: &mut [io::IoSliceMut]) -> Result<u64, Error> {
        Err(Error::Badf)
    }
    fn read_vectored_at(&self, bufs: &mut [io::IoSliceMut], offset: u64) -> Result<u64, Error> {
        Err(Error::Badf)
    }
    fn write_vectored(&self, bufs: &[io::IoSlice]) -> Result<u64, Error> {
        let n = self.borrow().write_vectored(bufs)?;
        Ok(n.try_into().map_err(|_| Error::Overflow)?)
    }
    fn write_vectored_at(&self, bufs: &[io::IoSlice], offset: u64) -> Result<u64, Error> {
        Err(Error::Badf)
    }
    fn seek(&self, pos: std::io::SeekFrom) -> Result<u64, Error> {
        Err(Error::Badf)
    }
    fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
        Err(Error::Badf)
    }
    fn set_times(
        &self,
        atime: Option<SystemTimeSpec>,
        mtime: Option<SystemTimeSpec>,
    ) -> Result<(), Error> {
        Err(Error::Badf)
    }
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(0)
    }
}

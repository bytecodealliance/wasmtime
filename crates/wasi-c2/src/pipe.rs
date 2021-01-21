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
use crate::file::{FdFlags, FileType, Filestat, WasiFile};
use crate::Error;
use std::any::Any;
use std::io::{self, Read, Write};
use std::sync::{Arc, RwLock};
use system_interface::fs::{Advice, FileIoExt};
use system_interface::io::ReadReady;

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

impl<R: Read> FileIoExt for ReadPipe<R> {
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn allocate(&self, offset: u64, len: u64) -> io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.borrow().read(buf)
    }
    fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
        self.borrow().read_exact(buf)
    }
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn read_vectored(&self, bufs: &mut [io::IoSliceMut]) -> io::Result<usize> {
        self.borrow().read_vectored(bufs)
    }
    fn read_to_end(&self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.borrow().read_to_end(buf)
    }
    fn read_to_string(&self, buf: &mut String) -> io::Result<usize> {
        self.borrow().read_to_string(buf)
    }
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn write_all(&self, buf: &[u8]) -> io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn write_all_at(&self, buf: &[u8], offset: u64) -> io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn write_vectored(&self, bufs: &[io::IoSlice]) -> io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn write_fmt(&self, fmt: std::fmt::Arguments) -> io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn flush(&self) -> io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn seek(&self, _pos: std::io::SeekFrom) -> io::Result<u64> {
        Err(std::io::Error::from_raw_os_error(libc::ESPIPE))
    }
    fn stream_position(&self) -> io::Result<u64> {
        Err(std::io::Error::from_raw_os_error(libc::ESPIPE))
    }
    fn peek(&self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF)) // XXX is this right? or do we have to implement this faithfully, and add a buffer of peeked values to handle during reads?
    }
}

impl<R: Read> fs_set_times::SetTimes for ReadPipe<R> {
    fn set_times(
        &self,
        _: Option<fs_set_times::SystemTimeSpec>,
        _: Option<fs_set_times::SystemTimeSpec>,
    ) -> io::Result<()> {
        todo!()
    }
}

impl<R: Read + Any> ReadReady for ReadPipe<R> {
    fn num_ready_bytes(&self) -> Result<u64, std::io::Error> {
        todo!()
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
    fn set_fdflags(&self, _fdflags: FdFlags) -> Result<(), Error> {
        Err(Error::Perm)
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

impl<W: Write> FileIoExt for WritePipe<W> {
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn allocate(&self, offset: u64, len: u64) -> io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn read_vectored(&self, bufs: &mut [io::IoSliceMut]) -> io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn read_to_end(&self, buf: &mut Vec<u8>) -> io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn read_to_string(&self, buf: &mut String) -> io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.borrow().write(buf)
    }
    fn write_all(&self, buf: &[u8]) -> io::Result<()> {
        self.borrow().write_all(buf)
    }
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn write_all_at(&self, buf: &[u8], offset: u64) -> io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
    fn write_vectored(&self, bufs: &[io::IoSlice]) -> io::Result<usize> {
        self.borrow().write_vectored(bufs)
    }
    fn write_fmt(&self, fmt: std::fmt::Arguments) -> io::Result<()> {
        self.borrow().write_fmt(fmt)
    }
    fn flush(&self) -> io::Result<()> {
        self.borrow().flush()
    }
    fn seek(&self, pos: std::io::SeekFrom) -> io::Result<u64> {
        Err(std::io::Error::from_raw_os_error(libc::ESPIPE))
    }
    fn stream_position(&self) -> io::Result<u64> {
        Err(std::io::Error::from_raw_os_error(libc::ESPIPE))
    }
    fn peek(&self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
}

impl<W: Write> fs_set_times::SetTimes for WritePipe<W> {
    fn set_times(
        &self,
        _: Option<fs_set_times::SystemTimeSpec>,
        _: Option<fs_set_times::SystemTimeSpec>,
    ) -> io::Result<()> {
        todo!() //
    }
}

impl<W: Write + Any> ReadReady for WritePipe<W> {
    fn num_ready_bytes(&self) -> Result<u64, std::io::Error> {
        Ok(0)
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
    fn set_fdflags(&self, _fdflags: FdFlags) -> Result<(), Error> {
        Err(Error::Perm)
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
}

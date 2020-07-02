//! Virtual pipes.
//!
//! These types provide easy implementations of `Handle` that mimic much of the behavior of Unix
//! pipes. These are particularly helpful for redirecting WASI stdio handles to destinations other
//! than OS files.
//!
//! Some convenience constructors are included for common backing types like `Vec<u8>` and `String`,
//! but the virtual pipes can be instantiated with any `Read` or `Write` type.
//!
//! Note that `poll_oneoff` is not supported for these types, so they do not match the behavior of
//! real pipes exactly.
use crate::handle::{Handle, HandleRights};
use crate::wasi::{types, Errno, Result};
use std::any::Any;
use std::io::{self, Read, Write};
use std::sync::{Arc, RwLock};

/// A virtual pipe read end.
///
/// A variety of `From` impls are provided so that common pipe types are easy to create. For example:
///
/// ```
/// # use wasi_common::WasiCtxBuilder;
/// # use wasi_common::virtfs::pipe::ReadPipe;
/// let mut ctx = WasiCtxBuilder::new();
/// let stdin = ReadPipe::from("hello from stdin!");
/// ctx.stdin(stdin);
/// ```
#[derive(Debug)]
pub struct ReadPipe<R: Read + Any> {
    rights: RwLock<HandleRights>,
    reader: Arc<RwLock<R>>,
}

impl<R: Read + Any> Clone for ReadPipe<R> {
    fn clone(&self) -> Self {
        Self {
            rights: RwLock::new(*self.rights.read().unwrap()),
            reader: self.reader.clone(),
        }
    }
}

impl<R: Read + Any> ReadPipe<R> {
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
        use types::Rights;
        Self {
            rights: RwLock::new(HandleRights::from_base(
                Rights::FD_DATASYNC
                    | Rights::FD_FDSTAT_SET_FLAGS
                    | Rights::FD_READ
                    | Rights::FD_SYNC
                    | Rights::FD_FILESTAT_GET
                    | Rights::POLL_FD_READWRITE,
            )),
            reader,
        }
    }

    /// Try to convert this `ReadPipe<R>` back to the underlying `R` type.
    ///
    /// This will fail with `Err(self)` if multiple references to the underlying `R` exist.
    pub fn try_into_inner(mut self) -> std::result::Result<R, Self> {
        match Arc::try_unwrap(self.reader) {
            Ok(rc) => Ok(RwLock::into_inner(rc).unwrap()),
            Err(reader) => {
                self.reader = reader;
                Err(self)
            }
        }
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

impl<R: Read + Any> Handle for ReadPipe<R> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn try_clone(&self) -> io::Result<Box<dyn Handle>> {
        Ok(Box::new(self.clone()))
    }

    fn get_file_type(&self) -> types::Filetype {
        types::Filetype::Unknown
    }

    fn get_rights(&self) -> HandleRights {
        *self.rights.read().unwrap()
    }

    fn set_rights(&self, rights: HandleRights) {
        *self.rights.write().unwrap() = rights;
    }

    fn advise(
        &self,
        _advice: types::Advice,
        _offset: types::Filesize,
        _len: types::Filesize,
    ) -> Result<()> {
        Err(Errno::Spipe)
    }

    fn allocate(&self, _offset: types::Filesize, _len: types::Filesize) -> Result<()> {
        Err(Errno::Spipe)
    }

    fn fdstat_set_flags(&self, _fdflags: types::Fdflags) -> Result<()> {
        // do nothing for now
        Ok(())
    }

    fn filestat_get(&self) -> Result<types::Filestat> {
        let stat = types::Filestat {
            dev: 0,
            ino: 0,
            nlink: 0,
            size: 0,
            atim: 0,
            ctim: 0,
            mtim: 0,
            filetype: self.get_file_type(),
        };
        Ok(stat)
    }

    fn filestat_set_size(&self, _st_size: types::Filesize) -> Result<()> {
        Err(Errno::Spipe)
    }

    fn preadv(&self, buf: &mut [io::IoSliceMut], offset: types::Filesize) -> Result<usize> {
        if offset != 0 {
            return Err(Errno::Spipe);
        }
        Ok(self.reader.write().unwrap().read_vectored(buf)?)
    }

    fn seek(&self, _offset: io::SeekFrom) -> Result<types::Filesize> {
        Err(Errno::Spipe)
    }

    fn read_vectored(&self, iovs: &mut [io::IoSliceMut]) -> Result<usize> {
        Ok(self.reader.write().unwrap().read_vectored(iovs)?)
    }

    fn create_directory(&self, _path: &str) -> Result<()> {
        Err(Errno::Notdir)
    }

    fn openat(
        &self,
        _path: &str,
        _read: bool,
        _write: bool,
        _oflags: types::Oflags,
        _fd_flags: types::Fdflags,
    ) -> Result<Box<dyn Handle>> {
        Err(Errno::Notdir)
    }

    fn link(
        &self,
        _old_path: &str,
        _new_handle: Box<dyn Handle>,
        _new_path: &str,
        _follow: bool,
    ) -> Result<()> {
        Err(Errno::Notdir)
    }

    fn readlink(&self, _path: &str, _buf: &mut [u8]) -> Result<usize> {
        Err(Errno::Notdir)
    }

    fn readlinkat(&self, _path: &str) -> Result<String> {
        Err(Errno::Notdir)
    }

    fn rename(&self, _old_path: &str, _new_handle: Box<dyn Handle>, _new_path: &str) -> Result<()> {
        Err(Errno::Notdir)
    }

    fn remove_directory(&self, _path: &str) -> Result<()> {
        Err(Errno::Notdir)
    }

    fn symlink(&self, _old_path: &str, _new_path: &str) -> Result<()> {
        Err(Errno::Notdir)
    }

    fn unlink_file(&self, _path: &str) -> Result<()> {
        Err(Errno::Notdir)
    }
}

/// A virtual pipe write end.
#[derive(Debug)]
pub struct WritePipe<W: Write + Any> {
    rights: RwLock<HandleRights>,
    writer: Arc<RwLock<W>>,
}

impl<W: Write + Any> Clone for WritePipe<W> {
    fn clone(&self) -> Self {
        Self {
            rights: RwLock::new(*self.rights.read().unwrap()),
            writer: self.writer.clone(),
        }
    }
}

impl<W: Write + Any> WritePipe<W> {
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
        use types::Rights;
        Self {
            rights: RwLock::new(HandleRights::from_base(
                Rights::FD_DATASYNC
                    | Rights::FD_FDSTAT_SET_FLAGS
                    | Rights::FD_SYNC
                    | Rights::FD_WRITE
                    | Rights::FD_FILESTAT_GET
                    | Rights::POLL_FD_READWRITE,
            )),
            writer,
        }
    }

    /// Try to convert this `WritePipe<W>` back to the underlying `W` type.
    ///
    /// This will fail with `Err(self)` if multiple references to the underlying `W` exist.
    pub fn try_into_inner(mut self) -> std::result::Result<W, Self> {
        match Arc::try_unwrap(self.writer) {
            Ok(rc) => Ok(RwLock::into_inner(rc).unwrap()),
            Err(writer) => {
                self.writer = writer;
                Err(self)
            }
        }
    }
}

impl WritePipe<io::Cursor<Vec<u8>>> {
    /// Create a new writable virtual pipe backed by a `Vec<u8>` buffer.
    pub fn new_in_memory() -> Self {
        Self::new(io::Cursor::new(vec![]))
    }
}

impl<W: Write + Any> Handle for WritePipe<W> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn try_clone(&self) -> io::Result<Box<dyn Handle>> {
        Ok(Box::new(self.clone()))
    }

    fn get_file_type(&self) -> types::Filetype {
        types::Filetype::Unknown
    }

    fn get_rights(&self) -> HandleRights {
        *self.rights.read().unwrap()
    }

    fn set_rights(&self, rights: HandleRights) {
        *self.rights.write().unwrap() = rights;
    }

    fn advise(
        &self,
        _advice: types::Advice,
        _offset: types::Filesize,
        _len: types::Filesize,
    ) -> Result<()> {
        Err(Errno::Spipe)
    }

    fn allocate(&self, _offset: types::Filesize, _len: types::Filesize) -> Result<()> {
        Err(Errno::Spipe)
    }

    fn fdstat_set_flags(&self, _fdflags: types::Fdflags) -> Result<()> {
        // do nothing for now
        Ok(())
    }

    fn filestat_get(&self) -> Result<types::Filestat> {
        let stat = types::Filestat {
            dev: 0,
            ino: 0,
            nlink: 0,
            size: 0,
            atim: 0,
            ctim: 0,
            mtim: 0,
            filetype: self.get_file_type(),
        };
        Ok(stat)
    }

    fn filestat_set_size(&self, _st_size: types::Filesize) -> Result<()> {
        Err(Errno::Spipe)
    }

    fn pwritev(&self, buf: &[io::IoSlice], offset: types::Filesize) -> Result<usize> {
        if offset != 0 {
            return Err(Errno::Spipe);
        }
        Ok(self.writer.write().unwrap().write_vectored(buf)?)
    }

    fn seek(&self, _offset: io::SeekFrom) -> Result<types::Filesize> {
        Err(Errno::Spipe)
    }

    fn write_vectored(&self, iovs: &[io::IoSlice]) -> Result<usize> {
        Ok(self.writer.write().unwrap().write_vectored(iovs)?)
    }

    fn create_directory(&self, _path: &str) -> Result<()> {
        Err(Errno::Notdir)
    }

    fn openat(
        &self,
        _path: &str,
        _read: bool,
        _write: bool,
        _oflags: types::Oflags,
        _fd_flags: types::Fdflags,
    ) -> Result<Box<dyn Handle>> {
        Err(Errno::Notdir)
    }

    fn link(
        &self,
        _old_path: &str,
        _new_handle: Box<dyn Handle>,
        _new_path: &str,
        _follow: bool,
    ) -> Result<()> {
        Err(Errno::Notdir)
    }

    fn readlink(&self, _path: &str, _buf: &mut [u8]) -> Result<usize> {
        Err(Errno::Notdir)
    }

    fn readlinkat(&self, _path: &str) -> Result<String> {
        Err(Errno::Notdir)
    }

    fn rename(&self, _old_path: &str, _new_handle: Box<dyn Handle>, _new_path: &str) -> Result<()> {
        Err(Errno::Notdir)
    }

    fn remove_directory(&self, _path: &str) -> Result<()> {
        Err(Errno::Notdir)
    }

    fn symlink(&self, _old_path: &str, _new_path: &str) -> Result<()> {
        Err(Errno::Notdir)
    }

    fn unlink_file(&self, _path: &str) -> Result<()> {
        Err(Errno::Notdir)
    }
}

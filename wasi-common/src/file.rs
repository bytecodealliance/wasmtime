use crate::{Error, ErrorExt, InputStream, OutputStream, SystemTimeSpec};
use bitflags::bitflags;
use std::any::Any;
use std::io;

#[async_trait::async_trait]
pub trait WasiFile: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    async fn get_filetype(&self) -> Result<FileType, Error>;

    #[cfg(unix)]
    fn pollable(&self) -> Option<rustix::fd::BorrowedFd> {
        None
    }

    #[cfg(windows)]
    fn pollable(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
        None
    }

    fn isatty(&mut self) -> bool {
        false
    }

    async fn datasync(&self) -> Result<(), Error>;

    async fn sync(&self) -> Result<(), Error>;

    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        Ok(FdFlags::empty())
    }

    async fn set_fdflags(&mut self, _flags: FdFlags) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn get_filestat(&self) -> Result<Filestat, Error> {
        Ok(Filestat {
            device_id: 0,
            inode: 0,
            filetype: self.get_filetype().await?,
            nlink: 0,
            size: 0, // XXX no way to get a size out of a Read :(
            atim: None,
            mtim: None,
            ctim: None,
        })
    }

    async fn set_filestat_size(&mut self, _size: u64) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn advise(&mut self, _offset: u64, _len: u64, _advice: Advice) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn allocate(&mut self, _offset: u64, _len: u64) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn set_times(
        &mut self,
        _atime: Option<SystemTimeSpec>,
        _mtime: Option<SystemTimeSpec>,
    ) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn read_at<'a>(&mut self, _buf: &mut [u8], _offset: u64) -> Result<(u64, bool), Error> {
        Err(Error::badf())
    }

    async fn read_vectored_at<'a>(
        &mut self,
        _bufs: &mut [std::io::IoSliceMut<'a>],
        _offset: u64,
    ) -> Result<(u64, bool), Error> {
        Err(Error::badf())
    }

    fn is_read_vectored_at(&self) -> bool {
        false
    }

    async fn write_at<'a>(&mut self, _bufs: &[u8], _offset: u64) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn write_vectored_at<'a>(
        &mut self,
        _bufs: &[std::io::IoSlice<'a>],
        _offset: u64,
    ) -> Result<u64, Error> {
        Err(Error::badf())
    }

    fn is_write_vectored_at(&self) -> bool {
        false
    }

    async fn append<'a>(&mut self, _bufs: &[u8]) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn append_vectored<'a>(&mut self, _bufs: &[std::io::IoSlice<'a>]) -> Result<u64, Error> {
        Err(Error::badf())
    }

    fn is_append_vectored(&self) -> bool {
        false
    }

    async fn readable(&self) -> Result<(), Error>;

    async fn writable(&self) -> Result<(), Error>;

    fn dup(&self) -> Box<dyn WasiFile>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FileType {
    Unknown,
    BlockDevice,
    CharacterDevice,
    Directory,
    RegularFile,
    SocketDgram,
    SocketStream,
    SymbolicLink,
    Pipe,
}

bitflags! {
    pub struct FdFlags: u32 {
        const APPEND   = 0b1;
        const DSYNC    = 0b10;
        const NONBLOCK = 0b100;
        const RSYNC    = 0b1000;
        const SYNC     = 0b10000;
    }
}

bitflags! {
    pub struct OFlags: u32 {
        const CREATE    = 0b1;
        const DIRECTORY = 0b10;
        const EXCLUSIVE = 0b100;
        const TRUNCATE  = 0b1000;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filestat {
    pub device_id: u64,
    pub inode: u64,
    pub filetype: FileType,
    pub nlink: u64,
    pub size: u64, // this is a read field, the rest are file fields
    pub atim: Option<std::time::SystemTime>,
    pub mtim: Option<std::time::SystemTime>,
    pub ctim: Option<std::time::SystemTime>,
}

pub trait TableFileExt {
    fn get_file(&self, fd: u32) -> Result<&dyn WasiFile, Error>;
    fn get_file_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiFile>, Error>;
}
impl TableFileExt for crate::table::Table {
    fn get_file(&self, fd: u32) -> Result<&dyn WasiFile, Error> {
        self.get::<Box<dyn WasiFile>>(fd).map(|f| f.as_ref())
    }
    fn get_file_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiFile>, Error> {
        self.get_mut::<Box<dyn WasiFile>>(fd)
    }
}

#[derive(Debug, Clone)]
pub struct FdStat {
    pub filetype: FileType,
    pub flags: FdFlags,
}

#[derive(Debug, Clone)]
pub enum Advice {
    Normal,
    Sequential,
    Random,
    WillNeed,
    DontNeed,
    NoReuse,
}

enum FileStreamType {
    /// Reading from a file, tracking our current position.
    Read(u64),

    /// Writing to a file, tracking our current position.
    Write(u64),

    /// Appending to a file.
    Append,
}

pub struct FileStream {
    /// Which file are we streaming?
    file: Box<dyn WasiFile>,

    /// What type of streaming are we doing?
    type_: FileStreamType,
}

impl FileStream {
    pub fn new_reader(file: Box<dyn WasiFile>, position: u64) -> Self {
        Self {
            file,
            type_: FileStreamType::Read(position),
        }
    }

    pub fn new_writer(file: Box<dyn WasiFile>, position: u64) -> Self {
        Self {
            file,
            type_: FileStreamType::Write(position),
        }
    }

    pub fn new_appender(file: Box<dyn WasiFile>) -> Self {
        Self {
            file,
            type_: FileStreamType::Append,
        }
    }
}

#[async_trait::async_trait]
impl InputStream for FileStream {
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[cfg(unix)]
    fn pollable_read(&self) -> Option<rustix::fd::BorrowedFd> {
        if let FileStreamType::Read(_) = self.type_ {
            self.file.pollable()
        } else {
            None
        }
    }

    #[cfg(windows)]
    fn pollable_read(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
        if let FileStreamType::Read(_) = self.type_ {
            self.file.pollable()
        } else {
            None
        }
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<(u64, bool), Error> {
        if let FileStreamType::Read(position) = &mut self.type_ {
            let (n, end) = self.file.read_at(buf, *position).await?;
            *position = position.wrapping_add(n);
            Ok((n, end))
        } else {
            Err(Error::badf())
        }
    }

    async fn read_vectored<'a>(
        &mut self,
        bufs: &mut [io::IoSliceMut<'a>],
    ) -> Result<(u64, bool), Error> {
        if let FileStreamType::Read(position) = &mut self.type_ {
            let (n, end) = self.file.read_vectored_at(bufs, *position).await?;
            *position = position.wrapping_add(n);
            Ok((n, end))
        } else {
            Err(Error::badf())
        }
    }

    #[cfg(can_vector)]
    fn is_read_vectored_at(&self) -> bool {
        if let FileStreamType::Read(_) = self.type_ {
            self.file.is_read_vectored_at()
        } else {
            false
        }
    }

    async fn skip(&mut self, nelem: u64) -> Result<(u64, bool), Error> {
        // For a zero-length request, don't do the 1-byte check below.
        if nelem == 0 {
            return self.file.read_at(&mut [], 0).await;
        }

        if let FileStreamType::Read(position) = &mut self.type_ {
            let new_position = position.checked_add(nelem).ok_or_else(Error::overflow)?;

            let file_size = self.file.get_filestat().await?.size;

            let short_by = new_position.saturating_sub(file_size);

            *position = new_position - short_by;
            Ok((nelem - short_by, false))
        } else {
            Err(Error::badf())
        }
    }

    async fn num_ready_bytes(&self) -> Result<u64, Error> {
        if let FileStreamType::Read(_) = self.type_ {
            // Default to saying that no data is ready.
            Ok(0)
        } else {
            Err(Error::badf())
        }
    }

    async fn readable(&self) -> Result<(), Error> {
        if let FileStreamType::Read(_) = self.type_ {
            self.file.readable().await
        } else {
            Err(Error::badf())
        }
    }
}

#[async_trait::async_trait]
impl OutputStream for FileStream {
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[cfg(unix)]
    fn pollable_write(&self) -> Option<rustix::fd::BorrowedFd> {
        if let FileStreamType::Read(_) = self.type_ {
            None
        } else {
            self.file.pollable()
        }
    }

    #[cfg(windows)]
    fn pollable_write(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
        if let FileStreamType::Read(_) = self.type_ {
            None
        } else {
            self.file.pollable()
        }
    }

    async fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
        match &mut self.type_ {
            FileStreamType::Write(position) => {
                let n = self.file.write_at(buf, *position).await? as i64 as u64;
                *position = position.wrapping_add(n);
                Ok(n)
            }
            FileStreamType::Append => {
                let n = self.file.append(buf).await? as i64 as u64;
                Ok(n)
            }
            FileStreamType::Read(_) => Err(Error::badf()),
        }
    }

    async fn write_vectored<'a>(&mut self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        match &mut self.type_ {
            FileStreamType::Write(position) => {
                let n = self.file.write_vectored_at(bufs, *position).await? as i64 as u64;
                *position = position.wrapping_add(n);
                Ok(n)
            }
            FileStreamType::Append => {
                let n = self.file.append_vectored(bufs).await? as i64 as u64;
                Ok(n)
            }
            FileStreamType::Read(_) => Err(Error::badf()),
        }
    }

    #[cfg(can_vector)]
    fn is_write_vectored_at(&self) -> bool {
        if let FileStreamType::Read(_) = self.type_ {
            false
        } else {
            self.file.is_write_vectored_at()
        }
    }

    // TODO: Optimize for file streams.
    /*
    async fn splice(
        &mut self,
        src: &mut dyn InputStream,
        nelem: u64,
    ) -> Result<u64, Error> {
        todo!()
    }
    */

    // TODO: Optimize for file streams.
    /*
    async fn write_zeroes(
        &mut self,
        nelem: u64,
    ) -> Result<u64, Error> {
        todo!()
    }
    */

    async fn writable(&self) -> Result<(), Error> {
        if let FileStreamType::Read(_) = self.type_ {
            Err(Error::badf())
        } else {
            self.file.writable().await
        }
    }
}

pub struct ReadOnlyFile(pub Box<dyn WasiFile>);

#[async_trait::async_trait]
impl WasiFile for ReadOnlyFile {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn get_filetype(&self) -> Result<FileType, Error> {
        self.0.get_filetype().await
    }

    #[cfg(unix)]
    fn pollable(&self) -> Option<rustix::fd::BorrowedFd> {
        self.0.pollable()
    }

    #[cfg(windows)]
    fn pollable(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
        self.0.pollable()
    }

    fn isatty(&mut self) -> bool {
        self.0.isatty()
    }

    async fn datasync(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn sync(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        self.0.get_fdflags().await
    }

    async fn set_fdflags(&mut self, _flags: FdFlags) -> Result<(), Error> {
        Err(Error::perm())
    }

    async fn get_filestat(&self) -> Result<Filestat, Error> {
        self.0.get_filestat().await
    }

    async fn set_filestat_size(&mut self, _size: u64) -> Result<(), Error> {
        Err(Error::perm())
    }

    async fn advise(&mut self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
        self.0.advise(offset, len, advice).await
    }

    async fn allocate(&mut self, _offset: u64, _len: u64) -> Result<(), Error> {
        Err(Error::perm())
    }

    async fn set_times(
        &mut self,
        _atime: Option<SystemTimeSpec>,
        _mtime: Option<SystemTimeSpec>,
    ) -> Result<(), Error> {
        Err(Error::perm())
    }

    async fn read_at<'a>(&mut self, buf: &mut [u8], offset: u64) -> Result<(u64, bool), Error> {
        self.0.read_at(buf, offset).await
    }

    async fn read_vectored_at<'a>(
        &mut self,
        bufs: &mut [std::io::IoSliceMut<'a>],
        offset: u64,
    ) -> Result<(u64, bool), Error> {
        self.0.read_vectored_at(bufs, offset).await
    }

    fn is_read_vectored_at(&self) -> bool {
        self.0.is_read_vectored_at()
    }

    async fn write_at<'a>(&mut self, _bufs: &[u8], _offset: u64) -> Result<u64, Error> {
        Err(Error::perm())
    }

    async fn write_vectored_at<'a>(
        &mut self,
        _bufs: &[std::io::IoSlice<'a>],
        _offset: u64,
    ) -> Result<u64, Error> {
        Err(Error::perm())
    }

    fn is_write_vectored_at(&self) -> bool {
        self.0.is_write_vectored_at()
    }

    async fn append<'a>(&mut self, _bufs: &[u8]) -> Result<u64, Error> {
        Err(Error::perm())
    }

    async fn append_vectored<'a>(&mut self, _bufs: &[std::io::IoSlice<'a>]) -> Result<u64, Error> {
        Err(Error::perm())
    }

    fn is_append_vectored(&self) -> bool {
        self.0.is_append_vectored()
    }

    async fn readable(&self) -> Result<(), Error> {
        self.0.readable().await
    }

    async fn writable(&self) -> Result<(), Error> {
        Err(Error::perm())
    }

    fn dup(&self) -> Box<dyn WasiFile> {
        Box::new(ReadOnlyFile(self.0.dup()))
    }
}

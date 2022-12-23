use crate::{Error, ErrorExt, SystemTimeSpec, WasiStream};
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
    fn pollable(&self) -> Option<io_extras::os::windows::RawHandleOrSocket> {
        None
    }

    fn isatty(&mut self) -> bool {
        false
    }

    async fn try_clone(&mut self) -> Result<Box<dyn WasiFile>, Error> {
        Err(Error::badf())
    }

    async fn datasync(&mut self) -> Result<(), Error> {
        Ok(())
    }

    async fn sync(&mut self) -> Result<(), Error> {
        Ok(())
    }

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

    async fn readable(&self) -> Result<(), Error>;

    async fn writable(&self) -> Result<(), Error>;
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

pub struct FileStream {
    // Which file are we streaming?
    file: Box<dyn WasiFile>,

    // Where in the file are we?
    position: u64,

    // Reading or writing?
    reading: bool,
}

impl FileStream {
    pub fn new_reader(file: Box<dyn WasiFile>, position: u64) -> Self {
        Self {
            file,
            position,
            reading: true,
        }
    }

    pub fn new_writer(file: Box<dyn WasiFile>, position: u64) -> Self {
        Self {
            file,
            position,
            reading: false,
        }
    }

    pub fn new_appender(_file: Box<dyn WasiFile>) -> Self {
        todo!()
    }

    pub async fn seek(&mut self, pos: std::io::SeekFrom) -> Result<u64, Error> {
        match pos {
            std::io::SeekFrom::Start(pos) => self.position = pos,
            std::io::SeekFrom::Current(pos) => {
                self.position = self.position.wrapping_add(pos as i64 as u64)
            }
            std::io::SeekFrom::End(pos) => {
                self.position = self
                    .file
                    .get_filestat()
                    .await?
                    .size
                    .wrapping_add(pos as i64 as u64)
            }
        }
        Ok(self.position)
    }
}

#[async_trait::async_trait]
impl WasiStream for FileStream {
    fn as_any(&self) -> &dyn Any {
        self
    }
    #[cfg(unix)]
    fn pollable_read(&self) -> Option<rustix::fd::BorrowedFd> {
        if self.reading {
            self.file.pollable()
        } else {
            None
        }
    }
    #[cfg(unix)]
    fn pollable_write(&self) -> Option<rustix::fd::BorrowedFd> {
        if self.reading {
            None
        } else {
            self.file.pollable()
        }
    }

    #[cfg(windows)]
    fn pollable_read(&self) -> Option<io_extras::os::windows::RawHandleOrSocket> {
        if self.reading {
            self.file.pollable()
        } else {
            None
        }
    }
    #[cfg(windows)]
    fn pollable_write(&self) -> Option<io_extras::os::windows::RawHandleOrSocket> {
        if self.reading {
            None
        } else {
            self.file.pollable()
        }
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<(u64, bool), Error> {
        if !self.reading {
            return Err(Error::badf());
        }
        let (n, end) = self.file.read_at(buf, self.position).await?;
        self.position = self.position.wrapping_add(n);
        Ok((n, end))
    }
    async fn read_vectored<'a>(
        &mut self,
        bufs: &mut [io::IoSliceMut<'a>],
    ) -> Result<(u64, bool), Error> {
        if !self.reading {
            return Err(Error::badf());
        }
        let (n, end) = self.file.read_vectored_at(bufs, self.position).await?;
        self.position = self.position.wrapping_add(n);
        Ok((n, end))
    }
    #[cfg(can_vector)]
    fn is_read_vectored_at(&self) -> bool {
        if !self.reading {
            return false;
        }
        self.file.is_read_vectored_at()
    }
    async fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
        if self.reading {
            return Err(Error::badf());
        }
        let n = self.file.write_at(buf, self.position).await? as i64 as u64;
        self.position = self.position.wrapping_add(n);
        Ok(n)
    }
    async fn write_vectored<'a>(&mut self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        if self.reading {
            return Err(Error::badf());
        }
        let n = self.file.write_vectored_at(bufs, self.position).await? as i64 as u64;
        self.position = self.position.wrapping_add(n);
        Ok(n)
    }
    #[cfg(can_vector)]
    fn is_write_vectored_at(&self) -> bool {
        if self.reading {
            return false;
        }
        self.file.is_write_vectored_at()
    }

    // TODO: Optimize for file streams.
    /*
    async fn splice(
        &mut self,
        dst: &mut dyn WasiStream,
        nelem: u64,
    ) -> Result<u64, Error> {
        todo!()
    }
    */

    async fn skip(&mut self, nelem: u64) -> Result<(u64, bool), Error> {
        // For a zero-length request, don't do the 1 byte check below.
        if nelem == 0 {
            return self.file.read_at(&mut [], 0).await;
        }

        if !self.reading {
            return Err(Error::badf());
        }

        let new_position = self
            .position
            .checked_add(nelem)
            .ok_or_else(Error::overflow)?;

        let file_size = self.file.get_filestat().await?.size;

        let short_by = new_position.saturating_sub(file_size);

        self.position = new_position - short_by;
        Ok((nelem - short_by, false))
    }

    // TODO: Optimize for file streams.
    /*
    async fn write_repeated(
        &mut self,
        byte: u8,
        nelem: u64,
    ) -> Result<u64, Error> {
        todo!()
    }
    */

    async fn num_ready_bytes(&self) -> Result<u64, Error> {
        if !self.reading {
            return Err(Error::badf());
        }
        Ok(0)
    }

    async fn readable(&self) -> Result<(), Error> {
        if !self.reading {
            return Err(Error::badf());
        }
        self.file.readable().await
    }

    async fn writable(&self) -> Result<(), Error> {
        if self.reading {
            return Err(Error::badf());
        }
        self.file.writable().await
    }
}

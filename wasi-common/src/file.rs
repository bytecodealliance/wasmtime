use crate::{Error, ErrorExt, SystemTimeSpec};
use bitflags::bitflags;
use std::any::Any;

#[async_trait::async_trait]
pub trait WasiFile: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    async fn get_filetype(&mut self) -> Result<FileType, Error>;

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

    async fn sock_accept(&mut self, _fdflags: FdFlags) -> Result<Box<dyn WasiFile>, Error> {
        Err(Error::badf())
    }

    async fn sock_recv<'a>(
        &mut self,
        _ri_data: &mut [std::io::IoSliceMut<'a>],
        _ri_flags: RiFlags,
    ) -> Result<(u64, RoFlags), Error> {
        Err(Error::badf())
    }

    async fn sock_send<'a>(
        &mut self,
        _si_data: &[std::io::IoSlice<'a>],
        _si_flags: SiFlags,
    ) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn sock_shutdown(&mut self, _how: SdFlags) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn datasync(&mut self) -> Result<(), Error> {
        Ok(())
    }

    async fn sync(&mut self) -> Result<(), Error> {
        Ok(())
    }

    async fn get_fdflags(&mut self) -> Result<FdFlags, Error> {
        Ok(FdFlags::empty())
    }

    async fn set_fdflags(&mut self, _flags: FdFlags) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn get_filestat(&mut self) -> Result<Filestat, Error> {
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

    async fn read_vectored<'a>(
        &mut self,
        _bufs: &mut [std::io::IoSliceMut<'a>],
    ) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn read_vectored_at<'a>(
        &mut self,
        _bufs: &mut [std::io::IoSliceMut<'a>],
        _offset: u64,
    ) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn write_vectored<'a>(&mut self, _bufs: &[std::io::IoSlice<'a>]) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn write_vectored_at<'a>(
        &mut self,
        _bufs: &[std::io::IoSlice<'a>],
        _offset: u64,
    ) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn seek(&mut self, _pos: std::io::SeekFrom) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn peek(&mut self, _buf: &mut [u8]) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(0)
    }

    async fn readable(&self) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn writable(&self) -> Result<(), Error> {
        Err(Error::badf())
    }
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
    pub struct SdFlags: u32 {
        const RD = 0b1;
        const WR = 0b10;
    }
}

bitflags! {
    pub struct SiFlags: u32 {
    }
}

bitflags! {
    pub struct RiFlags: u32 {
        const RECV_PEEK    = 0b1;
        const RECV_WAITALL = 0b10;
    }
}

bitflags! {
    pub struct RoFlags: u32 {
        const RECV_DATA_TRUNCATED = 0b1;
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

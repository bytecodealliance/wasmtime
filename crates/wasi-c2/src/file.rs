use crate::{Error, ErrorExt, SystemTimeSpec};
use bitflags::bitflags;
use std::any::Any;
use std::cell::Ref;
use std::ops::Deref;

pub trait WasiFile {
    fn as_any(&self) -> &dyn Any;
    fn datasync(&self) -> Result<(), Error>; // write op
    fn sync(&self) -> Result<(), Error>; // file op
    fn get_filetype(&self) -> Result<FileType, Error>; // file op
    fn get_fdflags(&self) -> Result<FdFlags, Error>; // file op
    /// This method takes a `&self` so that it can be called on a `&dyn WasiFile`. However,
    /// the caller makes the additional guarantee to drop `self` after the call is successful.
    unsafe fn reopen_with_fdflags(&self, flags: FdFlags) -> Result<Box<dyn WasiFile>, Error>; // file op
    fn get_filestat(&self) -> Result<Filestat, Error>; // split out get_length as a read & write op, rest is a file op
    fn set_filestat_size(&self, _size: u64) -> Result<(), Error>; // write op
    fn advise(
        &self,
        offset: u64,
        len: u64,
        advice: system_interface::fs::Advice,
    ) -> Result<(), Error>; // file op
    fn allocate(&self, offset: u64, len: u64) -> Result<(), Error>; // write op
    fn set_times(
        &self,
        atime: Option<SystemTimeSpec>,
        mtime: Option<SystemTimeSpec>,
    ) -> Result<(), Error>;
    fn read_vectored(&self, bufs: &mut [std::io::IoSliceMut]) -> Result<u64, Error>; // read op
    fn read_vectored_at(&self, bufs: &mut [std::io::IoSliceMut], offset: u64)
        -> Result<u64, Error>; // file op
    fn write_vectored(&self, bufs: &[std::io::IoSlice]) -> Result<u64, Error>; // write op
    fn write_vectored_at(&self, bufs: &[std::io::IoSlice], offset: u64) -> Result<u64, Error>; // file op
    fn seek(&self, pos: std::io::SeekFrom) -> Result<u64, Error>; // file op that generates a new stream from a file will supercede this
    fn peek(&self, buf: &mut [u8]) -> Result<u64, Error>; // read op
    fn num_ready_bytes(&self) -> Result<u64, Error>; // read op
}

#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Clone)]
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

pub(crate) trait TableFileExt {
    fn get_file(&self, fd: u32) -> Result<Ref<FileEntry>, Error>;
    fn update_file_in_place<F>(&mut self, fd: u32, f: F) -> Result<(), Error>
    where
        F: FnOnce(&dyn WasiFile) -> Result<Box<dyn WasiFile>, Error>;
}
impl TableFileExt for crate::table::Table {
    fn get_file(&self, fd: u32) -> Result<Ref<FileEntry>, Error> {
        self.get(fd)
    }
    fn update_file_in_place<F>(&mut self, fd: u32, f: F) -> Result<(), Error>
    where
        F: FnOnce(&dyn WasiFile) -> Result<Box<dyn WasiFile>, Error>,
    {
        self.update_in_place(fd, |FileEntry { caps, file }| {
            let file = f(file.deref())?;
            Ok(FileEntry { caps, file })
        })
    }
}

pub(crate) struct FileEntry {
    caps: FileCaps,
    file: Box<dyn WasiFile>,
}

impl FileEntry {
    pub fn new(caps: FileCaps, file: Box<dyn WasiFile>) -> Self {
        FileEntry { caps, file }
    }

    pub fn capable_of(&self, caps: FileCaps) -> Result<(), Error> {
        if self.caps.contains(caps) {
            Ok(())
        } else {
            Err(Error::not_capable().context(format!("desired {:?}, has {:?}", caps, self.caps,)))
        }
    }

    pub fn drop_caps_to(&mut self, caps: FileCaps) -> Result<(), Error> {
        self.capable_of(caps)?;
        self.caps = caps;
        Ok(())
    }

    pub fn get_fdstat(&self) -> Result<FdStat, Error> {
        Ok(FdStat {
            filetype: self.file.get_filetype()?,
            caps: self.caps,
            flags: self.file.get_fdflags()?,
        })
    }
}

pub trait FileEntryExt<'a> {
    fn get_cap(self, caps: FileCaps) -> Result<Ref<'a, dyn WasiFile>, Error>;
}

impl<'a> FileEntryExt<'a> for Ref<'a, FileEntry> {
    fn get_cap(self, caps: FileCaps) -> Result<Ref<'a, dyn WasiFile>, Error> {
        self.capable_of(caps)?;
        Ok(Ref::map(self, |r| r.file.deref()))
    }
}

bitflags! {
    pub struct FileCaps : u32 {
        const DATASYNC           = 0b1;
        const READ               = 0b10;
        const SEEK               = 0b100;
        const FDSTAT_SET_FLAGS   = 0b1000;
        const SYNC               = 0b10000;
        const TELL               = 0b100000;
        const WRITE              = 0b1000000;
        const ADVISE             = 0b10000000;
        const ALLOCATE           = 0b100000000;
        const FILESTAT_GET       = 0b1000000000;
        const FILESTAT_SET_SIZE  = 0b10000000000;
        const FILESTAT_SET_TIMES = 0b100000000000;
        const POLL_READWRITE     = 0b1000000000000;
    }
}

#[derive(Debug, Clone)]
pub struct FdStat {
    pub filetype: FileType,
    pub caps: FileCaps,
    pub flags: FdFlags,
}

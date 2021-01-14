use crate::Error;
use bitflags::bitflags;
use fs_set_times::SetTimes;
use std::any::Any;
use std::cell::Ref;
use std::ops::Deref;
use system_interface::fs::FileIoExt;

pub trait WasiFile: FileIoExt + SetTimes {
    fn as_any(&self) -> &dyn Any;
    fn datasync(&self) -> Result<(), Error>;
    fn sync(&self) -> Result<(), Error>;
    fn get_filetype(&self) -> Result<FileType, Error>;
    fn get_fdflags(&self) -> Result<FdFlags, Error>;
    fn set_fdflags(&self, _flags: FdFlags) -> Result<(), Error>;
    fn get_filestat(&self) -> Result<Filestat, Error>;
    fn set_filestat_size(&self, _size: u64) -> Result<(), Error>;
}

#[derive(Debug, Copy, Clone)]
pub enum FileType {
    Directory,
    BlockDevice,
    CharacterDevice,
    RegularFile,
    SocketDgram,
    SocketStream,
    SymbolicLink,
    Unknown,
}

impl From<&cap_std::fs::FileType> for FileType {
    fn from(ft: &cap_std::fs::FileType) -> FileType {
        use cap_fs_ext::FileTypeExt;
        if ft.is_dir() {
            FileType::Directory
        } else if ft.is_symlink() {
            FileType::SymbolicLink
        } else if ft.is_socket() {
            if ft.is_block_device() {
                FileType::SocketDgram
            } else {
                FileType::SocketStream
            }
        } else if ft.is_block_device() {
            FileType::BlockDevice
        } else if ft.is_char_device() {
            FileType::CharacterDevice
        } else if ft.is_file() {
            FileType::RegularFile
        } else {
            FileType::Unknown
        }
    }
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
    pub size: u64,
    pub atim: Option<std::time::SystemTime>,
    pub mtim: Option<std::time::SystemTime>,
    pub ctim: Option<std::time::SystemTime>,
}

pub(crate) trait TableFileExt {
    fn get_file(&self, fd: u32) -> Result<Ref<FileEntry>, Error>;
}
impl TableFileExt for crate::table::Table {
    fn get_file(&self, fd: u32) -> Result<Ref<FileEntry>, Error> {
        self.get(fd)
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
            Err(Error::FileNotCapable {
                desired: caps,
                has: self.caps,
            })
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

impl WasiFile for cap_std::fs::File {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn datasync(&self) -> Result<(), Error> {
        self.sync_data()?;
        Ok(())
    }
    fn sync(&self) -> Result<(), Error> {
        self.sync_all()?;
        Ok(())
    }
    fn get_filetype(&self) -> Result<FileType, Error> {
        let meta = self.metadata()?;
        Ok(FileType::from(&meta.file_type()))
    }
    fn get_fdflags(&self) -> Result<FdFlags, Error> {
        // XXX get_fdflags is not implemented but lets lie rather than panic:
        Ok(FdFlags::empty())
    }
    fn set_fdflags(&self, _fdflags: FdFlags) -> Result<(), Error> {
        todo!("set_fdflags is not implemented")
    }
    fn get_filestat(&self) -> Result<Filestat, Error> {
        let meta = self.metadata()?;
        use cap_fs_ext::MetadataExt;
        Ok(Filestat {
            device_id: meta.dev(),
            inode: meta.ino(),
            filetype: FileType::from(&meta.file_type()),
            nlink: meta.nlink(),
            size: meta.len(),
            atim: meta.accessed().map(|t| Some(t.into_std())).unwrap_or(None),
            mtim: meta.modified().map(|t| Some(t.into_std())).unwrap_or(None),
            ctim: meta.created().map(|t| Some(t.into_std())).unwrap_or(None),
        })
    }
    fn set_filestat_size(&self, size: u64) -> Result<(), Error> {
        self.set_len(size)?;
        Ok(())
    }
}

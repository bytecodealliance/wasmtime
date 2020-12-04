use crate::Error;
use std::ops::Deref;
use system_interface::fs::FileIoExt;

pub trait WasiFile: FileIoExt {
    fn allocate(&self, _offset: u64, _len: u64) -> Result<(), Error> {
        todo!("to implement fd_allocate, FileIoExt needs methods to get and set length of a file")
    }
    fn datasync(&self) -> Result<(), Error> {
        todo!("FileIoExt has no facilities for sync");
    }
    fn filetype(&self) -> Filetype {
        todo!("FileIoExt has no facilities for filetype");
    }
    fn oflags(&self) -> OFlags {
        todo!("FileIoExt has no facilities for oflags");
    }
    fn set_oflags(&self, _flags: OFlags) -> Result<(), Error> {
        todo!("FileIoExt has no facilities for oflags");
    }
    fn filestat_get(&self) -> Result<Filestat, Error> {
        todo!()
    }
    fn filestat_set_times(
        &self,
        _atim: Option<FilestatSetTime>,
        _mtim: Option<FilestatSetTime>,
    ) -> Result<(), Error> {
        todo!()
    }
    fn filestat_set_size(&self, _size: u64) -> Result<(), Error> {
        todo!()
    }
    fn sync(&self) -> Result<(), Error> {
        todo!("FileIoExt has no facilities for sync")
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Filetype {
    BlockDevice,
    CharacterDevice,
    RegularFile,
    SocketDgram,
    SocketStream,
}

pub struct OFlags {
    flags: u32,
}

impl OFlags {
    /// Checks if `other` is a subset of those capabilties:
    pub fn contains(&self, other: &Self) -> bool {
        self.flags & other.flags == other.flags
    }

    pub const ACCMODE: OFlags = OFlags { flags: 1 };
    pub const APPEND: OFlags = OFlags { flags: 2 };
    pub const CREATE: OFlags = OFlags { flags: 4 };
    pub const SYNC: OFlags = OFlags { flags: 4 };
    pub const NOFOLLOW: OFlags = OFlags { flags: 8 };
    pub const NONBLOCK: OFlags = OFlags { flags: 16 };
    pub const RDONLY: OFlags = OFlags { flags: 32 };
    pub const WRONLY: OFlags = OFlags { flags: 64 };
    pub const RDWR: OFlags = OFlags {
        flags: Self::RDONLY.flags | Self::WRONLY.flags,
    };
    // etc
}

#[derive(Debug, Clone)]
pub struct Filestat {
    device_id: u64,
    inode: u64,
    filetype: Filetype,
    nlink: u64,
    size: usize,
    atim: std::time::SystemTime,
    mtim: std::time::SystemTime,
    ctim: std::time::SystemTime,
}

#[derive(Debug, Copy, Clone)]
pub enum FilestatSetTime {
    Now,
    Absolute(std::time::SystemTime),
}

pub(crate) struct FileEntry {
    pub(crate) base_caps: FileCaps,
    pub(crate) inheriting_caps: FileCaps,
    pub(crate) file: Box<dyn WasiFile>,
}

impl FileEntry {
    pub fn get_cap(&self, caps: FileCaps) -> Result<&dyn WasiFile, Error> {
        if self.base_caps.contains(&caps) && self.inheriting_caps.contains(&caps) {
            Ok(self.file.deref())
        } else {
            Err(Error::FileNotCapable(caps))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FileCaps {
    flags: u32,
}

impl FileCaps {
    pub fn empty() -> Self {
        FileCaps { flags: 0 }
    }

    /// Checks if `other` is a subset of those capabilties:
    pub fn contains(&self, other: &Self) -> bool {
        self.flags & other.flags == other.flags
    }

    pub const DATASYNC: Self = FileCaps { flags: 1 };
    pub const READ: Self = FileCaps { flags: 2 };
    pub const SEEK: Self = FileCaps { flags: 4 };
    pub const FDSTAT_SET_FLAGS: Self = FileCaps { flags: 8 };
    pub const SYNC: Self = FileCaps { flags: 16 };
    pub const TELL: Self = FileCaps { flags: 32 };
    pub const WRITE: Self = FileCaps { flags: 64 };
    pub const ADVISE: Self = FileCaps { flags: 128 };
    pub const ALLOCATE: Self = FileCaps { flags: 256 };
    pub const FILESTAT_GET: Self = FileCaps { flags: 512 };
    pub const FILESTAT_SET_SIZE: Self = FileCaps { flags: 1024 };
    pub const FILESTAT_SET_TIMES: Self = FileCaps { flags: 2048 };
}

impl std::ops::BitOr for FileCaps {
    type Output = FileCaps;
    fn bitor(self, rhs: FileCaps) -> FileCaps {
        FileCaps {
            flags: self.flags | rhs.flags,
        }
    }
}

impl std::fmt::Display for FileCaps {
    fn fmt(&self, _f: &mut std::fmt::Formatter) -> std::fmt::Result {
        todo!()
    }
}

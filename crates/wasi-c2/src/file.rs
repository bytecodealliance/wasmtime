use crate::Error;
use cfg_if::cfg_if;
use fs_set_times::SetTimes;
use std::ops::Deref;
use system_interface::fs::FileIoExt;

pub trait WasiFile: FileIoExt + SetTimes {
    fn datasync(&self) -> Result<(), Error>;
    fn sync(&self) -> Result<(), Error>;
    fn get_filetype(&self) -> Result<Filetype, Error>;
    fn get_fdflags(&self) -> Result<FdFlags, Error>;
    fn get_oflags(&self) -> Result<OFlags, Error>;
    fn set_oflags(&self, _flags: OFlags) -> Result<(), Error>;
    fn get_filestat(&self) -> Result<Filestat, Error>;
    fn set_filestat_size(&self, _size: u64) -> Result<(), Error>;
}

// XXX missing:
// Unknown
// Directory
// SymbolicLink
#[derive(Debug, Copy, Clone)]
pub enum Filetype {
    BlockDevice,
    CharacterDevice,
    RegularFile,
    SocketDgram,
    SocketStream,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FdFlags {
    flags: u32,
}

impl FdFlags {
    /// Checks if `other` is a subset of those capabilties:
    pub fn contains(&self, other: &Self) -> bool {
        self.flags & other.flags == other.flags
    }

    pub const APPEND: FdFlags = FdFlags { flags: 1 };
    pub const DSYNC: FdFlags = FdFlags { flags: 2 };
    pub const NONBLOCK: FdFlags = FdFlags { flags: 4 };
    pub const RSYNC: FdFlags = FdFlags { flags: 8 };
    pub const SYNC: FdFlags = FdFlags { flags: 16 };
    // etc
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    size: u64,
    atim: std::time::SystemTime,
    mtim: std::time::SystemTime,
    ctim: std::time::SystemTime,
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

    pub fn get_fdstat(&self) -> Result<FdStat, Error> {
        Ok(FdStat {
            filetype: self.file.get_filetype()?,
            base_caps: self.base_caps,
            inheriting_caps: self.inheriting_caps,
            flags: self.file.get_fdflags()?,
        })
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

pub struct FdStat {
    pub filetype: Filetype,
    pub base_caps: FileCaps,
    pub inheriting_caps: FileCaps,
    pub flags: FdFlags,
}

impl WasiFile for cap_std::fs::File {
    fn datasync(&self) -> Result<(), Error> {
        self.sync_data()?;
        Ok(())
    }
    fn sync(&self) -> Result<(), Error> {
        self.sync_all()?;
        Ok(())
    }
    fn get_filetype(&self) -> Result<Filetype, Error> {
        let meta = self.metadata()?;
        // cap-std's Metadata/FileType only offers booleans indicating whether a file is a directory,
        // symlink, or regular file.
        // Directories should be excluded by the type system.
        if meta.is_file() {
            Ok(Filetype::RegularFile)
        } else {
            Err(Error::Badf) // XXX idk what to do here
        }
    }
    fn get_fdflags(&self) -> Result<FdFlags, Error> {
        // XXX cap-std doesnt expose append, dsync, nonblock, rsync, sync
        todo!()
    }
    fn get_oflags(&self) -> Result<OFlags, Error> {
        // XXX what if it was opened append, async, nonblock...
        let perms = self.metadata()?.permissions();
        if perms.readonly() {
            Ok(OFlags::RDONLY)
        } else {
            Ok(OFlags::RDWR)
        }
    }
    fn set_oflags(&self, flags: OFlags) -> Result<(), Error> {
        #![allow(unreachable_code, unused_variables)]
        cfg_if! {
            if #[cfg(unix)] {
                use std::os::unix::fs::PermissionsExt;
                use cap_std::fs::Permissions;
                use std::fs::Permissions as StdPermissions;
                let flags = todo!("normalize to unix flags {:?}", flags);
                self.set_permissions(Permissions::from_std(StdPermissions::from_mode(flags)))?;
            } else {
                Err(Error::Unsupported("set oflags on non-unix host system".to_owned()))
            }
        }
        Ok(())
    }
    fn get_filestat(&self) -> Result<Filestat, Error> {
        let meta = self.metadata()?;
        let (device_id, inode, nlink) = {
            cfg_if! {
                if #[cfg(unix)] {
                    use std::os::unix::fs::MetadataExt;
                    (meta.dev(), meta.ino(), meta.nlink())
                } else if #[cfg(all(windows, feature = "nightly"))] {
                    use std::os::windows::fs::MetadataExt;
                    ( meta.volume_serial_number().unwrap_or(-1),
                      meta.file_index().unwrap_or(-1),
                      meta.number_of_links().unwrap_or(0),
                    )
                } else {
                    (-1, -1, 0)
                }
            }
        };
        Ok(Filestat {
            device_id,
            inode,
            filetype: self.get_filetype()?,
            nlink,
            size: meta.len(),
            atim: meta.accessed()?.into_std(),
            mtim: meta.modified()?.into_std(),
            ctim: meta.created()?.into_std(),
        })
    }
    fn set_filestat_size(&self, size: u64) -> Result<(), Error> {
        self.set_len(size)?;
        Ok(())
    }
}

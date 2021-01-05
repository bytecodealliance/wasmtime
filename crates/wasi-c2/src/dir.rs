use crate::error::Error;
use crate::file::{FdFlags, FileCaps, FileType, Filestat, OFlags, WasiFile};
use std::any::Any;
use std::convert::TryInto;
use std::ops::Deref;
use std::path::{Path, PathBuf};

pub trait WasiDir {
    fn as_any(&self) -> &dyn Any;
    fn open_file(
        &self,
        symlink_follow: bool,
        path: &str,
        oflags: OFlags,
        caps: FileCaps,
        fdflags: FdFlags,
    ) -> Result<Box<dyn WasiFile>, Error>;
    fn open_dir(&self, symlink_follow: bool, path: &str) -> Result<Box<dyn WasiDir>, Error>;
    fn create_dir(&self, path: &str) -> Result<(), Error>;
    fn readdir(
        &self,
        cursor: ReaddirCursor,
    ) -> Result<Box<dyn Iterator<Item = Result<(ReaddirEntity, String), Error>>>, Error>;
    fn symlink(&self, old_path: &str, new_path: &str) -> Result<(), Error>;
    fn remove_dir(&self, path: &str) -> Result<(), Error>;
    fn unlink_file(&self, path: &str) -> Result<(), Error>;
    fn read_link(&self, path: &str) -> Result<PathBuf, Error>;
    fn get_filestat(&self) -> Result<Filestat, Error>;
    fn rename(&self, path: &str, dest_dir: &dyn WasiDir, dest_path: &str) -> Result<(), Error>;
    fn hard_link(
        &self,
        path: &str,
        symlink_follow: bool,
        target_dir: &dyn WasiDir,
        target_path: &str,
    ) -> Result<(), Error>;
}

pub(crate) struct DirEntry {
    caps: DirCaps,
    file_caps: FileCaps,
    preopen_path: Option<PathBuf>, // precondition: PathBuf is valid unicode
    dir: Box<dyn WasiDir>,
}

impl DirEntry {
    pub fn new(
        caps: DirCaps,
        file_caps: FileCaps,
        preopen_path: Option<PathBuf>,
        dir: Box<dyn WasiDir>,
    ) -> Self {
        DirEntry {
            caps,
            file_caps,
            preopen_path,
            dir,
        }
    }
    pub fn get_cap(&self, caps: DirCaps) -> Result<&dyn WasiDir, Error> {
        if self.caps.contains(&caps) {
            Ok(self.dir.deref())
        } else {
            Err(Error::DirNotCapable {
                desired: caps,
                has: self.caps,
            })
        }
    }
    pub fn drop_caps_to(&mut self, caps: DirCaps, file_caps: FileCaps) -> Result<(), Error> {
        if self.caps.contains(&caps) && self.file_caps.contains(&file_caps) {
            self.caps = caps;
            self.file_caps = file_caps;
            Ok(())
        } else {
            Err(Error::NotCapable)
        }
    }
    pub fn child_dir_caps(&self, desired_caps: DirCaps) -> DirCaps {
        self.caps.intersection(&desired_caps)
    }
    pub fn child_file_caps(&self, desired_caps: FileCaps) -> FileCaps {
        self.file_caps.intersection(&desired_caps)
    }
    pub fn get_dir_fdstat(&self) -> DirFdStat {
        DirFdStat {
            dir_caps: self.caps,
            file_caps: self.file_caps,
        }
    }
    pub fn preopen_path(&self) -> &Option<PathBuf> {
        &self.preopen_path
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DirCaps {
    flags: u32,
}

impl DirCaps {
    pub fn empty() -> Self {
        DirCaps { flags: 0 }
    }

    /// Checks if `other` is a subset of those capabilties:
    pub fn contains(&self, other: &Self) -> bool {
        self.flags & other.flags == other.flags
    }

    /// Intersection of two sets of flags (bitwise and)
    pub fn intersection(&self, rhs: &Self) -> Self {
        DirCaps {
            flags: self.flags & rhs.flags,
        }
    }
    pub const CREATE_DIRECTORY: Self = DirCaps { flags: 1 };
    pub const CREATE_FILE: Self = DirCaps { flags: 2 };
    pub const LINK_SOURCE: Self = DirCaps { flags: 4 };
    pub const LINK_TARGET: Self = DirCaps { flags: 8 };
    pub const OPEN: Self = DirCaps { flags: 16 };
    pub const READDIR: Self = DirCaps { flags: 32 };
    pub const READLINK: Self = DirCaps { flags: 64 };
    pub const RENAME_SOURCE: Self = DirCaps { flags: 128 };
    pub const RENAME_TARGET: Self = DirCaps { flags: 256 };
    pub const SYMLINK: Self = DirCaps { flags: 512 };
    pub const REMOVE_DIRECTORY: Self = DirCaps { flags: 1024 };
    pub const UNLINK_FILE: Self = DirCaps { flags: 2048 };
    pub const PATH_FILESTAT_GET: Self = DirCaps { flags: 4096 };
    pub const PATH_FILESTAT_SET_TIMES: Self = DirCaps { flags: 8192 };
    pub const FILESTAT_GET: Self = DirCaps { flags: 16384 };
    pub const FILESTAT_SET_TIMES: Self = DirCaps { flags: 32768 };

    // Missing that are in wasi-common directory_base:
    // FD_FDSTAT_SET_FLAGS
    // FD_SYNC
    // FD_ADVISE

    pub fn all() -> DirCaps {
        Self::CREATE_DIRECTORY
            | Self::CREATE_FILE
            | Self::LINK_SOURCE
            | Self::LINK_TARGET
            | Self::OPEN
            | Self::READDIR
            | Self::READLINK
            | Self::RENAME_SOURCE
            | Self::RENAME_TARGET
            | Self::SYMLINK
            | Self::REMOVE_DIRECTORY
            | Self::UNLINK_FILE
            | Self::PATH_FILESTAT_GET
            | Self::PATH_FILESTAT_SET_TIMES
            | Self::FILESTAT_GET
            | Self::FILESTAT_SET_TIMES
    }
}

impl std::ops::BitOr for DirCaps {
    type Output = DirCaps;
    fn bitor(self, rhs: DirCaps) -> DirCaps {
        DirCaps {
            flags: self.flags | rhs.flags,
        }
    }
}

pub struct DirFdStat {
    pub file_caps: FileCaps,
    pub dir_caps: DirCaps,
}

pub trait TableDirExt {
    fn is_preopen(&self, fd: u32) -> bool;
}

impl TableDirExt for crate::table::Table {
    fn is_preopen(&self, fd: u32) -> bool {
        if self.is::<DirEntry>(fd) {
            let dir_entry: std::cell::Ref<DirEntry> = self.get(fd).unwrap();
            dir_entry.preopen_path.is_some()
        } else {
            false
        }
    }
}

pub struct ReaddirEntity {
    pub next: ReaddirCursor,
    pub inode: u64,
    pub namelen: u32,
    pub filetype: FileType,
}

#[derive(Debug, Copy, Clone)]
pub struct ReaddirCursor(u64);
impl From<u64> for ReaddirCursor {
    fn from(c: u64) -> ReaddirCursor {
        ReaddirCursor(c)
    }
}
impl From<ReaddirCursor> for u64 {
    fn from(c: ReaddirCursor) -> u64 {
        c.0
    }
}

impl WasiDir for cap_std::fs::Dir {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn open_file(
        &self,
        symlink_follow: bool,
        path: &str,
        oflags: OFlags,
        caps: FileCaps,
        fdflags: FdFlags,
    ) -> Result<Box<dyn WasiFile>, Error> {
        use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};

        let mut opts = cap_std::fs::OpenOptions::new();

        if oflags.contains(&(OFlags::CREATE | OFlags::EXCLUSIVE)) {
            opts.create_new(true);
            opts.write(true);
        } else if oflags.contains(&OFlags::CREATE) {
            opts.create(true);
            opts.write(true);
        }
        if oflags.contains(&OFlags::TRUNCATE) {
            opts.truncate(true);
        }
        if caps.contains(&FileCaps::WRITE)
            || caps.contains(&FileCaps::DATASYNC)
            || caps.contains(&FileCaps::ALLOCATE)
            || caps.contains(&FileCaps::FILESTAT_SET_SIZE)
        {
            opts.write(true);
        } else {
            // If not opened write, open read. This way the OS lets us open the file.
            // If FileCaps::READ is not set, read calls will be rejected at the
            // get_cap check.
            opts.read(true);
        }
        if caps.contains(&FileCaps::READ) {
            opts.read(true);
        }
        if fdflags.contains(&FdFlags::APPEND) {
            opts.append(true);
        }
        // XXX what about rest of fdflags - dsync, sync become oflags.
        // what do we do with nonblock?
        // what do we do with rsync?

        if symlink_follow {
            opts.follow(FollowSymlinks::Yes);
        }

        let f = self.open_with(Path::new(path), &opts)?;
        Ok(Box::new(f))
    }

    fn open_dir(&self, symlink_follow: bool, path: &str) -> Result<Box<dyn WasiDir>, Error> {
        // XXX obey symlink_follow
        let d = self.open_dir(Path::new(path))?;
        Ok(Box::new(d))
    }

    fn create_dir(&self, path: &str) -> Result<(), Error> {
        self.create_dir(Path::new(path))?;
        Ok(())
    }
    fn readdir(
        &self,
        cursor: ReaddirCursor,
    ) -> Result<Box<dyn Iterator<Item = Result<(ReaddirEntity, String), Error>>>, Error> {
        use cap_fs_ext::MetadataExt;

        // cap_std's read_dir does not include . and .., we should prepend these.
        // Why closures? failure of any individual entry doesn't mean the whole method should
        // fail.
        // Why is the Ok case a tuple? We can't construct a cap_std::fs::DirEntry, and we don't
        // have enough info to make a ReaddirEntity yet.
        let rd = vec![
            (|| {
                let meta = self.dir_metadata()?;
                let name = ".".to_owned();
                let namelen = name.as_bytes().len().try_into()?;
                Ok((FileType::Directory, meta.ino(), namelen, name))
            })(),
            (|| {
                // XXX if parent dir is mounted it *might* be possible to give its inode, but we
                // don't know that in this context.
                let name = "..".to_owned();
                let namelen = name.as_bytes().len().try_into()?;
                Ok((FileType::Directory, 0, namelen, name))
            })(),
        ]
        .into_iter()
        .chain(
            // Now process the `DirEntry`s:
            self.read_dir(Path::new("."))
                .expect("always possible to readdir an open Dir") // XXX is this true?
                .map(|entry| {
                    let entry = entry?;
                    let meta = entry.metadata()?;
                    let inode = meta.ino();
                    let filetype = FileType::from(&meta.file_type());
                    let name = entry.file_name().into_string().map_err(|_| Error::Ilseq)?;
                    let namelen = name.as_bytes().len().try_into()?;
                    Ok((filetype, inode, namelen, name))
                }),
        )
        // Enumeration of the iterator makes it possible to define the ReaddirCursor
        .enumerate()
        .map(|(ix, r)| match r {
            Ok((filetype, inode, namelen, name)) => Ok((
                ReaddirEntity {
                    next: ReaddirCursor::from(ix as u64 + 1),
                    filetype,
                    inode,
                    namelen,
                },
                name,
            )),
            Err(e) => Err(e),
        })
        .skip(u64::from(cursor) as usize);

        Ok(Box::new(rd))
    }

    fn symlink(&self, src_path: &str, dest_path: &str) -> Result<(), Error> {
        self.symlink(Path::new(src_path), Path::new(dest_path))?;
        Ok(())
    }
    fn remove_dir(&self, path: &str) -> Result<(), Error> {
        self.remove_dir(Path::new(path))?;
        Ok(())
    }

    fn unlink_file(&self, path: &str) -> Result<(), Error> {
        self.remove_file(Path::new(path))?;
        Ok(())
    }
    fn read_link(&self, path: &str) -> Result<PathBuf, Error> {
        let link = self.read_link(Path::new(path))?;
        Ok(link)
    }
    fn get_filestat(&self) -> Result<Filestat, Error> {
        let meta = self.metadata(".")?;
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
    fn rename(&self, src_path: &str, dest_dir: &dyn WasiDir, dest_path: &str) -> Result<(), Error> {
        let dest_dir = dest_dir
            .as_any()
            .downcast_ref::<Self>()
            .ok_or(Error::NotCapable)?;
        self.rename(Path::new(src_path), dest_dir, Path::new(dest_path))?;
        Ok(())
    }
    fn hard_link(
        &self,
        src_path: &str,
        symlink_follow: bool,
        target_dir: &dyn WasiDir,
        target_path: &str,
    ) -> Result<(), Error> {
        let target_dir = target_dir
            .as_any()
            .downcast_ref::<Self>()
            .ok_or(Error::NotCapable)?;
        self.hard_link(Path::new(src_path), target_dir, Path::new(target_path))?;
        Ok(())
    }
}

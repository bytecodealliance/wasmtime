use crate::file::{FdFlags, FileCaps, FileType, Filestat, OFlags, WasiFile};
use crate::{Error, ErrorExt, SystemTimeSpec};
use bitflags::bitflags;
use std::any::Any;
use std::cell::Ref;
use std::ops::Deref;
use std::path::PathBuf;

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
    fn get_path_filestat(&self, path: &str, follow_symlinks: bool) -> Result<Filestat, Error>;
    fn rename(&self, path: &str, dest_dir: &dyn WasiDir, dest_path: &str) -> Result<(), Error>;
    fn hard_link(
        &self,
        path: &str,
        target_dir: &dyn WasiDir,
        target_path: &str,
    ) -> Result<(), Error>;
    fn set_times(
        &self,
        path: &str,
        atime: Option<SystemTimeSpec>,
        mtime: Option<SystemTimeSpec>,
        follow_symlinks: bool,
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
    pub fn capable_of_dir(&self, caps: DirCaps) -> Result<(), Error> {
        if self.caps.contains(caps) {
            Ok(())
        } else {
            Err(Error::not_capable().context(format!("desired {:?}, has {:?}", caps, self.caps,)))
        }
    }
    pub fn capable_of_file(&self, caps: FileCaps) -> Result<(), Error> {
        if self.file_caps.contains(caps) {
            Ok(())
        } else {
            Err(Error::not_capable()
                .context(format!("desired {:?}, has {:?}", caps, self.file_caps)))
        }
    }
    pub fn drop_caps_to(&mut self, caps: DirCaps, file_caps: FileCaps) -> Result<(), Error> {
        self.capable_of_dir(caps)?;
        self.capable_of_file(file_caps)?;
        self.caps = caps;
        self.file_caps = file_caps;
        Ok(())
    }
    pub fn child_dir_caps(&self, desired_caps: DirCaps) -> DirCaps {
        self.caps & desired_caps
    }
    pub fn child_file_caps(&self, desired_caps: FileCaps) -> FileCaps {
        self.file_caps & desired_caps
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

pub trait DirEntryExt<'a> {
    fn get_cap(self, caps: DirCaps) -> Result<Ref<'a, dyn WasiDir>, Error>;
}

impl<'a> DirEntryExt<'a> for Ref<'a, DirEntry> {
    fn get_cap(self, caps: DirCaps) -> Result<Ref<'a, dyn WasiDir>, Error> {
        self.capable_of_dir(caps)?;
        Ok(Ref::map(self, |r| r.dir.deref()))
    }
}

bitflags! {
    pub struct DirCaps: u32 {
        const CREATE_DIRECTORY        = 0b1;
        const CREATE_FILE             = 0b10;
        const LINK_SOURCE             = 0b100;
        const LINK_TARGET             = 0b1000;
        const OPEN                    = 0b10000;
        const READDIR                 = 0b100000;
        const READLINK                = 0b1000000;
        const RENAME_SOURCE           = 0b10000000;
        const RENAME_TARGET           = 0b100000000;
        const SYMLINK                 = 0b1000000000;
        const REMOVE_DIRECTORY        = 0b10000000000;
        const UNLINK_FILE             = 0b100000000000;
        const PATH_FILESTAT_GET       = 0b1000000000000;
        const PATH_FILESTAT_SET_TIMES = 0b10000000000000;
        const FILESTAT_GET            = 0b100000000000000;
        const FILESTAT_SET_TIMES      = 0b1000000000000000;
    }
}

#[derive(Debug, Clone)]
pub struct DirFdStat {
    pub file_caps: FileCaps,
    pub dir_caps: DirCaps,
}

pub(crate) trait TableDirExt {
    fn get_dir(&self, fd: u32) -> Result<Ref<DirEntry>, Error>;
    fn is_preopen(&self, fd: u32) -> bool;
}

impl TableDirExt for crate::table::Table {
    fn get_dir(&self, fd: u32) -> Result<Ref<DirEntry>, Error> {
        self.get(fd)
    }
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

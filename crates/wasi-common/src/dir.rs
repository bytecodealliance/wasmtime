use crate::file::{FdFlags, FileType, Filestat, OFlags, WasiFile};
use crate::{Error, ErrorExt, SystemTimeSpec};
use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;

pub enum OpenResult {
    File(Box<dyn WasiFile>),
    Dir(Box<dyn WasiDir>),
}

#[wiggle::async_trait]
pub trait WasiDir: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    async fn open_file(
        &self,
        _symlink_follow: bool,
        _path: &str,
        _oflags: OFlags,
        _read: bool,
        _write: bool,
        _fdflags: FdFlags,
    ) -> Result<OpenResult, Error> {
        Err(Error::not_supported())
    }

    async fn create_dir(&self, _path: &str) -> Result<(), Error> {
        Err(Error::not_supported())
    }

    // XXX the iterator here needs to be asyncified as well!
    async fn readdir(
        &self,
        _cursor: ReaddirCursor,
    ) -> Result<Box<dyn Iterator<Item = Result<ReaddirEntity, Error>> + Send>, Error> {
        Err(Error::not_supported())
    }

    async fn symlink(&self, _old_path: &str, _new_path: &str) -> Result<(), Error> {
        Err(Error::not_supported())
    }

    async fn remove_dir(&self, _path: &str) -> Result<(), Error> {
        Err(Error::not_supported())
    }

    async fn unlink_file(&self, _path: &str) -> Result<(), Error> {
        Err(Error::not_supported())
    }

    async fn read_link(&self, _path: &str) -> Result<PathBuf, Error> {
        Err(Error::not_supported())
    }

    async fn get_filestat(&self) -> Result<Filestat, Error> {
        Err(Error::not_supported())
    }

    async fn get_path_filestat(
        &self,
        _path: &str,
        _follow_symlinks: bool,
    ) -> Result<Filestat, Error> {
        Err(Error::not_supported())
    }

    async fn rename(
        &self,
        _path: &str,
        _dest_dir: &dyn WasiDir,
        _dest_path: &str,
    ) -> Result<(), Error> {
        Err(Error::not_supported())
    }

    async fn hard_link(
        &self,
        _path: &str,
        _target_dir: &dyn WasiDir,
        _target_path: &str,
    ) -> Result<(), Error> {
        Err(Error::not_supported())
    }

    async fn set_times(
        &self,
        _path: &str,
        _atime: Option<SystemTimeSpec>,
        _mtime: Option<SystemTimeSpec>,
        _follow_symlinks: bool,
    ) -> Result<(), Error> {
        Err(Error::not_supported())
    }
}

pub(crate) struct DirEntry {
    preopen_path: Option<PathBuf>, // precondition: PathBuf is valid unicode
    pub dir: Box<dyn WasiDir>,
}

impl DirEntry {
    pub fn new(preopen_path: Option<PathBuf>, dir: Box<dyn WasiDir>) -> Self {
        DirEntry { preopen_path, dir }
    }
    pub fn preopen_path(&self) -> &Option<PathBuf> {
        &self.preopen_path
    }
}

pub(crate) trait TableDirExt {
    fn get_dir(&self, fd: u32) -> Result<Arc<DirEntry>, Error>;
}

impl TableDirExt for crate::table::Table {
    fn get_dir(&self, fd: u32) -> Result<Arc<DirEntry>, Error> {
        self.get(fd)
    }
}

#[derive(Debug, Clone)]
pub struct ReaddirEntity {
    pub next: ReaddirCursor,
    pub inode: u64,
    pub name: String,
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

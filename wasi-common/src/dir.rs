use crate::file::{FdFlags, FileType, Filestat, OFlags, ReadOnlyFile, WasiFile};
use crate::{Error, ErrorExt, SystemTimeSpec};
use std::any::Any;
use std::path::PathBuf;

#[async_trait::async_trait]
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
    ) -> Result<Box<dyn WasiFile>, Error> {
        Err(Error::not_supported())
    }

    async fn open_dir(
        &self,
        _symlink_follow: bool,
        _path: &str,
    ) -> Result<Box<dyn WasiDir>, Error> {
        Err(Error::not_supported())
    }

    async fn datasync(&self) -> Result<(), Error>;

    async fn sync(&self) -> Result<(), Error>;

    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        Ok(FdFlags::empty())
    }

    async fn create_dir(&self, _path: &str) -> Result<(), Error> {
        Err(Error::not_supported())
    }

    // XXX the iterator here needs to be asyncified as well!
    async fn readdir(&self, _cursor: ReaddirCursor) -> Result<ReaddirIterator, Error> {
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

    fn dup(&self) -> Box<dyn WasiDir>;
}

pub trait TableDirExt {
    fn get_dir(&self, fd: u32) -> Result<&Box<dyn WasiDir>, Error>;
}

impl TableDirExt for crate::table::Table {
    fn get_dir(&self, fd: u32) -> Result<&Box<dyn WasiDir>, Error> {
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

pub type ReaddirIterator = Box<dyn Iterator<Item = Result<ReaddirEntity, Error>> + Send>;

pub struct ReadOnlyDir(pub Box<dyn WasiDir>);

#[async_trait::async_trait]
impl WasiDir for ReadOnlyDir {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn open_file(
        &self,
        symlink_follow: bool,
        path: &str,
        oflags: OFlags,
        read: bool,
        write: bool,
        fdflags: FdFlags,
    ) -> Result<Box<dyn WasiFile>, Error> {
        if write {
            Err(Error::perm())
        } else {
            self.0
                .open_file(symlink_follow, path, oflags, read, write, fdflags)
                .await
                .map(|f| Box::new(ReadOnlyFile(f)) as _)
        }
    }

    async fn datasync(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn sync(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn open_dir(&self, symlink_follow: bool, path: &str) -> Result<Box<dyn WasiDir>, Error> {
        self.0
            .open_dir(symlink_follow, path)
            .await
            .map(|d| Box::new(Self(d)) as _)
    }

    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        self.0.get_fdflags().await
    }

    async fn create_dir(&self, _path: &str) -> Result<(), Error> {
        Err(Error::perm())
    }

    async fn readdir(&self, cursor: ReaddirCursor) -> Result<ReaddirIterator, Error> {
        self.0.readdir(cursor).await
    }

    async fn symlink(&self, _old_path: &str, _new_path: &str) -> Result<(), Error> {
        Err(Error::perm())
    }

    async fn remove_dir(&self, _path: &str) -> Result<(), Error> {
        Err(Error::perm())
    }

    async fn unlink_file(&self, _path: &str) -> Result<(), Error> {
        Err(Error::perm())
    }

    async fn read_link(&self, path: &str) -> Result<PathBuf, Error> {
        self.0.read_link(path).await
    }

    async fn get_filestat(&self) -> Result<Filestat, Error> {
        self.0.get_filestat().await
    }

    async fn get_path_filestat(
        &self,
        path: &str,
        follow_symlinks: bool,
    ) -> Result<Filestat, Error> {
        self.0.get_path_filestat(path, follow_symlinks).await
    }

    async fn rename(
        &self,
        _path: &str,
        _dest_dir: &dyn WasiDir,
        _dest_path: &str,
    ) -> Result<(), Error> {
        Err(Error::perm())
    }

    async fn hard_link(
        &self,
        _path: &str,
        _target_dir: &dyn WasiDir,
        _target_path: &str,
    ) -> Result<(), Error> {
        Err(Error::perm())
    }

    async fn set_times(
        &self,
        _path: &str,
        _atime: Option<SystemTimeSpec>,
        _mtime: Option<SystemTimeSpec>,
        _follow_symlinks: bool,
    ) -> Result<(), Error> {
        Err(Error::perm())
    }

    fn dup(&self) -> Box<dyn WasiDir> {
        Box::new(ReadOnlyDir(self.0.dup()))
    }
}

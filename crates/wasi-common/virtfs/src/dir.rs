use crate::file::File;
use std::any::Any;
use std::path::{Path, PathBuf};
use wasi_common::{
    dir::{ReaddirCursor, ReaddirEntity, WasiDir},
    file::{FdFlags, FileCaps, FileType, Filestat, OFlags, WasiFile},
    Error, ErrorExt,
};

pub struct Dir;

impl Dir {}

impl WasiDir for Dir {
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
        todo!()
    }

    fn open_dir(&self, symlink_follow: bool, path: &str) -> Result<Box<dyn WasiDir>, Error> {
        todo!()
    }

    fn create_dir(&self, path: &str) -> Result<(), Error> {
        todo!()
    }
    fn readdir(
        &self,
        cursor: ReaddirCursor,
    ) -> Result<Box<dyn Iterator<Item = Result<(ReaddirEntity, String), Error>>>, Error> {
        todo!()
    }

    fn symlink(&self, src_path: &str, dest_path: &str) -> Result<(), Error> {
        todo!()
    }
    fn remove_dir(&self, path: &str) -> Result<(), Error> {
        todo!()
    }

    fn unlink_file(&self, path: &str) -> Result<(), Error> {
        todo!()
    }
    fn read_link(&self, path: &str) -> Result<PathBuf, Error> {
        todo!()
    }
    fn get_filestat(&self) -> Result<Filestat, Error> {
        todo!()
    }
    fn get_path_filestat(&self, path: &str, follow_symlinks: bool) -> Result<Filestat, Error> {
        todo!()
    }
    fn rename(&self, src_path: &str, dest_dir: &dyn WasiDir, dest_path: &str) -> Result<(), Error> {
        todo!()
    }
    fn hard_link(
        &self,
        src_path: &str,
        target_dir: &dyn WasiDir,
        target_path: &str,
    ) -> Result<(), Error> {
        todo!()
    }
    fn set_times(
        &self,
        path: &str,
        atime: Option<wasi_common::SystemTimeSpec>,
        mtime: Option<wasi_common::SystemTimeSpec>,
        follow_symlinks: bool,
    ) -> Result<(), Error> {
        todo!()
    }
}

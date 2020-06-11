use super::sys_impl::oshandle::RawOsHandle;
use super::{fd, path, AsFile};
use crate::handle::{Handle, HandleRights};
use crate::wasi::{types, Errno, Result};
use log::{debug, error};
use std::any::Any;
use std::io;
use std::ops::Deref;

// TODO could this be cleaned up?
// The actual `OsDir` struct is OS-dependent, therefore we delegate
// its definition to OS-specific modules.
pub use super::sys_impl::osdir::OsDir;

impl Deref for OsDir {
    type Target = RawOsHandle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl Handle for OsDir {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn try_clone(&self) -> io::Result<Box<dyn Handle>> {
        let handle = self.handle.try_clone()?;
        let new = Self::new(self.rights.get(), handle)?;
        Ok(Box::new(new))
    }
    fn get_file_type(&self) -> types::Filetype {
        types::Filetype::Directory
    }
    fn get_rights(&self) -> HandleRights {
        self.rights.get()
    }
    fn set_rights(&self, rights: HandleRights) {
        self.rights.set(rights)
    }
    // FdOps
    fn fdstat_get(&self) -> Result<types::Fdflags> {
        fd::fdstat_get(&*self.as_file()?)
    }
    fn fdstat_set_flags(&self, fdflags: types::Fdflags) -> Result<()> {
        if let Some(new_file) = fd::fdstat_set_flags(&*self.as_file()?, fdflags)? {
            self.handle.update_from(new_file);
        }
        Ok(())
    }
    fn filestat_get(&self) -> Result<types::Filestat> {
        fd::filestat_get(&*self.as_file()?)
    }
    fn filestat_set_times(
        &self,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<()> {
        fd::filestat_set_times(&*self.as_file()?, atim, mtim, fst_flags)
    }
    fn readdir<'a>(
        &'a self,
        cookie: types::Dircookie,
    ) -> Result<Box<dyn Iterator<Item = Result<(types::Dirent, String)>> + 'a>> {
        fd::readdir(self, cookie)
    }
    // PathOps
    fn create_directory(&self, path: &str) -> Result<()> {
        path::create_directory(self, path)
    }
    fn filestat_get_at(&self, path: &str, follow: bool) -> Result<types::Filestat> {
        path::filestat_get_at(self, path, follow)
    }
    fn filestat_set_times_at(
        &self,
        path: &str,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
        follow: bool,
    ) -> Result<()> {
        path::filestat_set_times_at(self, path, atim, mtim, fst_flags, follow)
    }
    fn openat(
        &self,
        path: &str,
        read: bool,
        write: bool,
        oflags: types::Oflags,
        fd_flags: types::Fdflags,
    ) -> Result<Box<dyn Handle>> {
        path::open(self, path, read, write, oflags, fd_flags)
    }
    fn link(
        &self,
        old_path: &str,
        new_handle: Box<dyn Handle>,
        new_path: &str,
        follow: bool,
    ) -> Result<()> {
        let new_handle = match new_handle.as_any().downcast_ref::<Self>() {
            None => {
                error!("Tried to link with handle that's not an OsDir");
                return Err(Errno::Badf);
            }
            Some(handle) => handle,
        };
        path::link(self, old_path, new_handle, new_path, follow)
    }
    fn symlink(&self, old_path: &str, new_path: &str) -> Result<()> {
        path::symlink(old_path, self, new_path)
    }
    fn readlink(&self, path: &str, buf: &mut [u8]) -> Result<usize> {
        path::readlink(self, path, buf)
    }
    fn readlinkat(&self, path: &str) -> Result<String> {
        path::readlinkat(self, path)
    }
    fn rename(&self, old_path: &str, new_handle: Box<dyn Handle>, new_path: &str) -> Result<()> {
        let new_handle = match new_handle.as_any().downcast_ref::<Self>() {
            None => {
                error!("Tried to rename with handle that's not an OsDir");
                return Err(Errno::Badf);
            }
            Some(handle) => handle,
        };
        debug!("rename (old_dirfd, old_path)=({:?}, {:?})", self, old_path);
        debug!(
            "rename (new_dirfd, new_path)=({:?}, {:?})",
            new_handle, new_path
        );
        path::rename(self, old_path, new_handle, new_path)
    }
    fn remove_directory(&self, path: &str) -> Result<()> {
        debug!("remove_directory (dirfd, path)=({:?}, {:?})", self, path);
        path::remove_directory(self, path)
    }
    fn unlink_file(&self, path: &str) -> Result<()> {
        path::unlink_file(self, path)
    }
}

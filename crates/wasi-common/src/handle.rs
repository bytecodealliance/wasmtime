use crate::entry::EntryRights;
use crate::wasi::{types, Errno, Result};
use std::any::Any;
use std::io::{self, SeekFrom};

pub(crate) trait Handle {
    fn as_any(&self) -> &dyn Any;
    fn try_clone(&self) -> io::Result<Box<dyn Handle>>;
    fn get_file_type(&self) -> io::Result<types::Filetype>;
    fn get_rights(&self) -> io::Result<EntryRights> {
        Ok(EntryRights::empty())
    }
    fn is_directory(&self) -> bool {
        if let Ok(ft) = self.get_file_type() {
            return ft == types::Filetype::Directory;
        }
        false
    }
    // TODO perhaps should be a separate trait?
    // FdOps
    fn advise(
        &self,
        _advice: types::Advice,
        _offset: types::Filesize,
        _len: types::Filesize,
    ) -> Result<()> {
        Err(Errno::Badf)
    }
    fn allocate(&self, _offset: types::Filesize, _len: types::Filesize) -> Result<()> {
        Err(Errno::Badf)
    }
    fn datasync(&self) -> Result<()> {
        Err(Errno::Inval)
    }
    fn fdstat_get(&self) -> Result<types::Fdflags> {
        Ok(types::Fdflags::empty())
    }
    fn fdstat_set_flags(&self, _fdflags: types::Fdflags) -> Result<()> {
        Err(Errno::Badf)
    }
    fn filestat_get(&self) -> Result<types::Filestat> {
        Err(Errno::Badf)
    }
    fn filestat_set_size(&self, _st_size: types::Filesize) -> Result<()> {
        Err(Errno::Badf)
    }
    fn filestat_set_times(
        &self,
        _atim: types::Timestamp,
        _mtim: types::Timestamp,
        _fst_flags: types::Fstflags,
    ) -> Result<()> {
        Err(Errno::Badf)
    }
    fn preadv(&self, _buf: &mut [io::IoSliceMut], _offset: u64) -> Result<usize> {
        Err(Errno::Badf)
    }
    fn pwritev(&self, _buf: &[io::IoSlice], _offset: u64) -> Result<usize> {
        Err(Errno::Badf)
    }
    fn read_vectored(&self, _iovs: &mut [io::IoSliceMut]) -> Result<usize> {
        Err(Errno::Badf)
    }
    fn readdir<'a>(
        &'a self,
        _cookie: types::Dircookie,
    ) -> Result<Box<dyn Iterator<Item = Result<(types::Dirent, String)>> + 'a>> {
        Err(Errno::Badf)
    }
    fn seek(&self, _offset: SeekFrom) -> Result<u64> {
        Err(Errno::Badf)
    }
    fn sync(&self) -> Result<()> {
        Ok(())
    }
    fn write_vectored(&self, _iovs: &[io::IoSlice], _isatty: bool) -> Result<usize> {
        Err(Errno::Badf)
    }
    // TODO perhaps should be a separate trait?
    // PathOps
    fn create_directory(&self, _path: &str) -> Result<()> {
        Err(Errno::Acces)
    }
    fn openat(
        &self,
        _path: &str,
        _read: bool,
        _write: bool,
        _oflags: types::Oflags,
        _fd_flags: types::Fdflags,
    ) -> Result<Box<dyn Handle>> {
        Err(Errno::Acces)
    }
    fn link(
        &self,
        _old_path: &str,
        _new_handle: Box<dyn Handle>,
        _new_path: &str,
        _follow: bool,
    ) -> Result<()> {
        Err(Errno::Acces)
    }
    fn readlink(&self, _path: &str, _buf: &mut [u8]) -> Result<usize> {
        Err(Errno::Acces)
    }
    fn readlinkat(&self, _path: &str) -> Result<String> {
        Err(Errno::Acces)
    }
    fn remove_directory(&self, _path: &str) -> Result<()> {
        Err(Errno::Acces)
    }
    fn rename(&self, _old_path: &str, _new_handle: Box<dyn Handle>, _new_path: &str) -> Result<()> {
        Err(Errno::Acces)
    }
    fn symlink(&self, _old_path: &str, _new_path: &str) -> Result<()> {
        Err(Errno::Acces)
    }
    fn unlink_file(&self, _path: &str) -> Result<()> {
        Err(Errno::Acces)
    }
}

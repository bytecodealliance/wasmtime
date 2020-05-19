use crate::wasi::types::{self, Rights};
use crate::wasi::{Errno, Result};
use std::any::Any;
use std::fmt;
use std::io::{self, SeekFrom};

/// Represents rights of a `Handle`, either already held or required.
#[derive(Debug, Copy, Clone)]
pub(crate) struct HandleRights {
    pub(crate) base: Rights,
    pub(crate) inheriting: Rights,
}

impl HandleRights {
    pub(crate) fn new(base: Rights, inheriting: Rights) -> Self {
        Self { base, inheriting }
    }

    /// Create new `HandleRights` instance from `base` rights only, keeping
    /// `inheriting` set to none.
    pub(crate) fn from_base(base: Rights) -> Self {
        Self {
            base,
            inheriting: Rights::empty(),
        }
    }

    /// Create new `HandleRights` instance with both `base` and `inheriting`
    /// rights set to none.
    pub(crate) fn empty() -> Self {
        Self {
            base: Rights::empty(),
            inheriting: Rights::empty(),
        }
    }

    /// Check if `other` is a subset of those rights.
    pub(crate) fn contains(&self, other: &Self) -> bool {
        self.base.contains(&other.base) && self.inheriting.contains(&other.inheriting)
    }
}

impl fmt::Display for HandleRights {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "HandleRights {{ base: {}, inheriting: {} }}",
            self.base, self.inheriting
        )
    }
}

pub(crate) trait Handle {
    fn as_any(&self) -> &dyn Any;
    fn try_clone(&self) -> io::Result<Box<dyn Handle>>;
    fn get_file_type(&self) -> types::Filetype;
    fn get_rights(&self) -> HandleRights {
        HandleRights::empty()
    }
    fn set_rights(&self, rights: HandleRights);
    fn is_directory(&self) -> bool {
        self.get_file_type() == types::Filetype::Directory
    }
    /// Test whether this descriptor is considered a tty within WASI.
    /// Note that since WASI itself lacks an `isatty` syscall and relies
    /// on a conservative approximation, we use the same approximation here.
    fn is_tty(&self) -> bool {
        let file_type = self.get_file_type();
        let rights = self.get_rights();
        let required_rights = HandleRights::from_base(Rights::FD_SEEK | Rights::FD_TELL);
        file_type == types::Filetype::CharacterDevice && rights.contains(&required_rights)
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
    fn write_vectored(&self, _iovs: &[io::IoSlice]) -> Result<usize> {
        Err(Errno::Badf)
    }
    // TODO perhaps should be a separate trait?
    // PathOps
    fn create_directory(&self, _path: &str) -> Result<()> {
        Err(Errno::Acces)
    }
    fn filestat_get_at(&self, _path: &str, _follow: bool) -> Result<types::Filestat> {
        Err(Errno::Acces)
    }
    fn filestat_set_times_at(
        &self,
        _path: &str,
        _atim: types::Timestamp,
        _mtim: types::Timestamp,
        _fst_flags: types::Fstflags,
        _follow: bool,
    ) -> Result<()> {
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

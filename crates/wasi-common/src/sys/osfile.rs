use super::sys_impl::oshandle::RawOsHandle;
use super::{fd, AsFile};
use crate::handle::{
    Advice, Fdflags, Filesize, Filestat, Filetype, Fstflags, Handle, HandleRights,
};
use crate::sched::Timestamp;
use crate::{Error, Result};
use std::any::Any;
use std::cell::Cell;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::Deref;

#[derive(Debug)]
/// A file backed by the operating system's file system. Dereferences to a
/// `RawOsHandle`.  Its impl of `Handle` uses Rust's `std` to implement all
/// file descriptor operations.
///
/// # Constructing `OsFile`
///
/// `OsFile` can currently only be constructed from `std::fs::File` using
/// the `std::convert::TryFrom` trait:
///
/// ```rust,no_run
/// use std::fs::OpenOptions;
/// use std::convert::TryFrom;
/// use wasi_common::OsFile;
///
/// let file = OpenOptions::new().read(true).open("some_file").unwrap();
/// let os_file = OsFile::try_from(file).unwrap();
/// ```
pub struct OsFile {
    rights: Cell<HandleRights>,
    handle: RawOsHandle,
}

impl OsFile {
    pub(super) fn new(rights: HandleRights, handle: RawOsHandle) -> Self {
        let rights = Cell::new(rights);
        Self { rights, handle }
    }
}

impl Deref for OsFile {
    type Target = RawOsHandle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl Handle for OsFile {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn try_clone(&self) -> io::Result<Box<dyn Handle>> {
        let handle = self.handle.try_clone()?;
        let rights = self.rights.clone();
        Ok(Box::new(Self { rights, handle }))
    }
    fn get_file_type(&self) -> Filetype {
        Filetype::RegularFile
    }
    fn get_rights(&self) -> HandleRights {
        self.rights.get()
    }
    fn set_rights(&self, rights: HandleRights) {
        self.rights.set(rights)
    }
    // FdOps
    fn advise(&self, advice: Advice, offset: Filesize, len: Filesize) -> Result<()> {
        fd::advise(self, advice, offset, len)
    }
    fn allocate(&self, offset: Filesize, len: Filesize) -> Result<()> {
        let fd = self.as_file()?;
        let metadata = fd.metadata()?;
        let current_size = metadata.len();
        let wanted_size = offset.checked_add(len).ok_or(Error::TooBig)?;
        // This check will be unnecessary when rust-lang/rust#63326 is fixed
        if wanted_size > i64::max_value() as u64 {
            return Err(Error::TooBig);
        }
        if wanted_size > current_size {
            fd.set_len(wanted_size)?;
        }
        Ok(())
    }
    fn datasync(&self) -> Result<()> {
        self.as_file()?.sync_data()?;
        Ok(())
    }
    fn fdstat_get(&self) -> Result<Fdflags> {
        fd::fdstat_get(&*self.as_file()?)
    }
    fn fdstat_set_flags(&self, fdflags: Fdflags) -> Result<()> {
        if let Some(new_handle) = fd::fdstat_set_flags(&*self.as_file()?, fdflags)? {
            self.handle.update_from(new_handle);
        }
        Ok(())
    }
    fn filestat_get(&self) -> Result<Filestat> {
        fd::filestat_get(&*self.as_file()?)
    }
    fn filestat_set_size(&self, size: Filesize) -> Result<()> {
        self.as_file()?.set_len(size)?;
        Ok(())
    }
    fn filestat_set_times(
        &self,
        atim: Timestamp,
        mtim: Timestamp,
        fst_flags: Fstflags,
    ) -> Result<()> {
        fd::filestat_set_times(&*self.as_file()?, atim, mtim, fst_flags)
    }
    fn preadv(&self, buf: &mut [io::IoSliceMut], offset: u64) -> Result<usize> {
        let mut fd: &File = &*self.as_file()?;
        let cur_pos = fd.seek(SeekFrom::Current(0))?;
        fd.seek(SeekFrom::Start(offset))?;
        let nread = self.read_vectored(buf)?;
        fd.seek(SeekFrom::Start(cur_pos))?;
        Ok(nread)
    }
    fn pwritev(&self, buf: &[io::IoSlice], offset: u64) -> Result<usize> {
        let mut fd: &File = &*self.as_file()?;
        let cur_pos = fd.seek(SeekFrom::Current(0))?;
        fd.seek(SeekFrom::Start(offset))?;
        let nwritten = self.write_vectored(&buf)?;
        fd.seek(SeekFrom::Start(cur_pos))?;
        Ok(nwritten)
    }
    fn read_vectored(&self, iovs: &mut [io::IoSliceMut]) -> Result<usize> {
        let nread = self.as_file()?.read_vectored(iovs)?;
        Ok(nread)
    }
    fn seek(&self, offset: SeekFrom) -> Result<u64> {
        let pos = self.as_file()?.seek(offset)?;
        Ok(pos)
    }
    fn sync(&self) -> Result<()> {
        self.as_file()?.sync_all()?;
        Ok(())
    }
    fn write_vectored(&self, iovs: &[io::IoSlice]) -> Result<usize> {
        let nwritten = self.as_file()?.write_vectored(&iovs)?;
        Ok(nwritten)
    }
}

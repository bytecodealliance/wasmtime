use crate::host::Dirent;
use crate::{wasi, Error, Result};
use filetime::FileTime;
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub trait VirtualFile {
    // methods that virtual files need to have to uh, work right?
    fn fdstat_get(&self) -> wasi::__wasi_fdflags_t {
        0
    }

    fn try_clone(&self) -> io::Result<Box<dyn VirtualFile>>;

    fn readlinkat(&self, path: &Path) -> Result<String>;

    fn openat(
        &self,
        path: &Path,
        read: bool,
        write: bool,
        oflags: u16,
        fs_flags: u16,
    ) -> Result<Box<dyn VirtualFile>>;

    fn datasync(&self) -> Result<()> {
        Err(Error::EINVAL)
    }

    fn sync(&self) -> Result<()> {
        Ok(())
    }

    fn create_directory(&self, _path: &Path) -> Result<()> {
        Err(Error::EACCES)
    }

    fn readdir(
        &self,
        _cookie: wasi::__wasi_dircookie_t,
    ) -> Result<Box<dyn Iterator<Item = Result<Dirent>>>> {
        Err(Error::EBADF)
    }

    fn write_vectored(&mut self, _iovs: &[io::IoSlice]) -> Result<usize> {
        Err(Error::EBADF)
    }

    fn pread(&self, _buf: &mut [u8], _offset: u64) -> Result<usize> {
        Err(Error::EBADF)
    }

    fn pwrite(&self, _buf: &mut [u8], _offset: u64) -> Result<usize> {
        Err(Error::EBADF)
    }

    fn seek(&mut self, _offset: SeekFrom) -> Result<u64> {
        Err(Error::EBADF)
    }

    fn advise(
        &self,
        _advice: wasi::__wasi_advice_t,
        _offset: wasi::__wasi_filesize_t,
        _len: wasi::__wasi_filesize_t,
    ) -> Result<()> {
        Err(Error::EBADF)
    }

    fn allocate(
        &self,
        _offset: wasi::__wasi_filesize_t,
        _len: wasi::__wasi_filesize_t,
    ) -> Result<()> {
        Err(Error::EBADF)
    }

    fn filestat_get(&self) -> Result<wasi::__wasi_filestat_t> {
        Err(Error::EBADF)
    }

    fn filestat_set_times(&self, _atim: Option<FileTime>, _mtim: Option<FileTime>) -> Result<()> {
        Err(Error::EBADF)
    }

    fn filestat_set_size(&self, _st_size: wasi::__wasi_filesize_t) -> Result<()> {
        Err(Error::EBADF)
    }

    fn fdstat_set_flags(&self, _fdflags: wasi::__wasi_fdflags_t) -> Result<()> {
        Err(Error::EBADF)
    }

    fn read_vectored(&mut self, _iovs: &mut [io::IoSliceMut]) -> Result<usize> {
        Err(Error::EBADF)
    }

    fn get_file_type(&self) -> wasi::__wasi_filetype_t;

    fn get_rights_base(&self) -> wasi::__wasi_rights_t;

    fn get_rights_inheriting(&self) -> wasi::__wasi_rights_t;
}

pub struct InMemoryFile {
    cursor: usize,
    data: Arc<RefCell<Vec<u8>>>,
}

impl InMemoryFile {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            data: Arc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn append(&self, data: &[u8]) {
        self.data.borrow_mut().extend_from_slice(data);
    }
}

impl VirtualFile for InMemoryFile {
    fn try_clone(&self) -> io::Result<Box<dyn VirtualFile>> {
        Ok(Box::new(InMemoryFile {
            cursor: 0,
            data: Arc::clone(&self.data),
        }))
    }

    fn readlinkat(&self, _path: &Path) -> Result<String> {
        // no symlink support, so always say it's invalid.
        Err(Error::EINVAL)
    }

    fn openat(
        &self,
        _path: &Path,
        _read: bool,
        _write: bool,
        _oflags: u16,
        _fs_flags: u16,
    ) -> Result<Box<dyn VirtualFile>> {
        Err(Error::EACCES)
    }

    fn write_vectored(&mut self, iovs: &[io::IoSlice]) -> Result<usize> {
        let mut data = self.data.borrow_mut();
        let mut cursor = self.cursor;
        for iov in iovs {
            for el in iov.iter() {
                if cursor == data.len() {
                    data.push(*el);
                } else {
                    data[cursor] = *el;
                }
                cursor += 1;
            }
        }
        let len = cursor - self.cursor;
        self.cursor = cursor;
        Ok(len)
    }

    fn read_vectored(&mut self, iovs: &mut [io::IoSliceMut]) -> Result<usize> {
        let data = self.data.borrow();
        let mut cursor = self.cursor;
        for iov in iovs {
            for i in 0..iov.len() {
                if cursor >= data.len() {
                    let count = cursor - self.cursor;
                    self.cursor = cursor;
                    return Ok(count);
                }
                iov[i] = data[cursor];
                cursor += 1;
            }
        }

        let count = cursor - self.cursor;
        self.cursor = cursor;
        Ok(count)
    }

    fn advise(
        &self,
        advice: wasi::__wasi_advice_t,
        _offset: wasi::__wasi_filesize_t,
        _len: wasi::__wasi_filesize_t,
    ) -> Result<()> {
        // we'll just ignore advice for now, unless it's totally invalid
        match advice {
            wasi::__WASI_ADVICE_DONTNEED
            | wasi::__WASI_ADVICE_SEQUENTIAL
            | wasi::__WASI_ADVICE_WILLNEED
            | wasi::__WASI_ADVICE_NOREUSE
            | wasi::__WASI_ADVICE_RANDOM
            | wasi::__WASI_ADVICE_NORMAL => Ok(()),
            _ => Err(Error::EINVAL),
        }
    }

    fn get_file_type(&self) -> wasi::__wasi_filetype_t {
        wasi::__WASI_FILETYPE_REGULAR_FILE
    }

    fn get_rights_base(&self) -> wasi::__wasi_rights_t {
        wasi::RIGHTS_REGULAR_FILE_BASE
    }

    fn get_rights_inheriting(&self) -> wasi::__wasi_rights_t {
        wasi::RIGHTS_REGULAR_FILE_INHERITING
    }
}

/// A clonable read/write directory.
pub struct VirtualDir {
    writable: bool,
    entries: Arc<RefCell<HashMap<PathBuf, Box<dyn VirtualFile>>>>,
}

impl VirtualDir {
    pub fn new(writable: bool) -> Self {
        VirtualDir {
            writable,
            entries: Arc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn with_file<P: AsRef<Path>>(self, file: Box<dyn VirtualFile>, path: P) -> Self {
        self.entries
            .borrow_mut()
            .insert(path.as_ref().to_owned(), file);
        self
    }
}

impl VirtualFile for VirtualDir {
    fn try_clone(&self) -> io::Result<Box<dyn VirtualFile>> {
        Ok(Box::new(VirtualDir {
            writable: self.writable,
            entries: Arc::clone(&self.entries),
        }))
    }

    fn readlinkat(&self, _path: &Path) -> Result<String> {
        // no symlink support, so always say it's invalid.
        Err(Error::EINVAL)
    }

    fn openat(
        &self,
        path: &Path,
        _read: bool,
        _write: bool,
        _oflags: u16,
        _fs_flags: u16,
    ) -> Result<Box<dyn VirtualFile>> {
        let mut entries = self.entries.borrow_mut();
        match entries.entry(path.to_owned()) {
            Entry::Occupied(e) => e.get().try_clone().map_err(Into::into),
            Entry::Vacant(v) => {
                if self.writable {
                    println!("created new file: {}", path.display());
                    v.insert(Box::new(InMemoryFile::new()))
                        .try_clone()
                        .map_err(Into::into)
                } else {
                    Err(Error::EACCES)
                }
            }
        }
    }

    fn create_directory(&self, path: &Path) -> Result<()> {
        let mut entries = self.entries.borrow_mut();
        match entries.entry(path.to_owned()) {
            Entry::Occupied(_) => Err(Error::EEXIST),
            Entry::Vacant(v) => {
                if self.writable {
                    println!("created new virtual directory at: {}", path.display());
                    v.insert(Box::new(VirtualDir::new(false)));
                    Ok(())
                } else {
                    Err(Error::EACCES)
                }
            }
        }
    }

    fn write_vectored(&mut self, _iovs: &[io::IoSlice]) -> Result<usize> {
        Err(Error::EBADF)
    }

    fn get_file_type(&self) -> wasi::__wasi_filetype_t {
        wasi::__WASI_FILETYPE_DIRECTORY
    }

    fn get_rights_base(&self) -> wasi::__wasi_rights_t {
        wasi::RIGHTS_DIRECTORY_BASE
    }

    fn get_rights_inheriting(&self) -> wasi::__wasi_rights_t {
        wasi::RIGHTS_DIRECTORY_INHERITING
    }
}

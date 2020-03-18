use crate::host::Dirent;
use crate::host::FileType;
use crate::wasi::{self, WasiError, WasiResult};
use crate::wasi32;
use filetime::FileTime;
use log::trace;
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub enum VirtualDirEntry {
    Directory(HashMap<String, VirtualDirEntry>),
    File(Box<dyn FileContents>),
}

impl VirtualDirEntry {
    pub fn empty_directory() -> Self {
        Self::Directory(HashMap::new())
    }
}

/// Files and directories may be moved, and for implementation reasons retain a reference to their
/// parent VirtualFile, so files that can be moved must provide an interface to update their parent
/// reference.
pub(crate) trait MovableFile {
    fn set_parent(&self, new_parent: Option<Box<dyn VirtualFile>>);
}

/// `VirtualFile` encompasses the whole interface of a `File`, `Directory`, or `Stream`-like
/// object, suitable for forwarding from `wasi-common` public interfaces. `File` and
/// `Directory`-style objects can be moved, so implemetors of this trait must also implement
/// `MovableFile`.
///
/// Default implementations of functions here fail in ways that are intended to mimic a file-like
/// object with no permissions, no content, and that cannot be used in any way.
pub(crate) trait VirtualFile: MovableFile {
    fn fdstat_get(&self) -> wasi::__wasi_fdflags_t {
        0
    }

    fn try_clone(&self) -> io::Result<Box<dyn VirtualFile>>;

    fn readlinkat(&self, _path: &Path) -> WasiResult<String> {
        Err(WasiError::EACCES)
    }

    fn openat(
        &self,
        _path: &Path,
        _read: bool,
        _write: bool,
        _oflags: wasi::__wasi_oflags_t,
        _fd_flags: wasi::__wasi_fdflags_t,
    ) -> WasiResult<Box<dyn VirtualFile>> {
        Err(WasiError::EACCES)
    }

    fn remove_directory(&self, _path: &str) -> WasiResult<()> {
        Err(WasiError::EACCES)
    }

    fn unlink_file(&self, _path: &str) -> WasiResult<()> {
        Err(WasiError::EACCES)
    }

    fn datasync(&self) -> WasiResult<()> {
        Err(WasiError::EINVAL)
    }

    fn sync(&self) -> WasiResult<()> {
        Ok(())
    }

    fn create_directory(&self, _path: &Path) -> WasiResult<()> {
        Err(WasiError::EACCES)
    }

    fn readdir(
        &self,
        _cookie: wasi::__wasi_dircookie_t,
    ) -> WasiResult<Box<dyn Iterator<Item = WasiResult<Dirent>>>> {
        Err(WasiError::EBADF)
    }

    fn write_vectored(&mut self, _iovs: &[io::IoSlice]) -> WasiResult<usize> {
        Err(WasiError::EBADF)
    }

    fn pread(&self, _buf: &mut [u8], _offset: u64) -> WasiResult<usize> {
        Err(WasiError::EBADF)
    }

    fn pwrite(&self, _buf: &mut [u8], _offset: u64) -> WasiResult<usize> {
        Err(WasiError::EBADF)
    }

    fn seek(&mut self, _offset: SeekFrom) -> WasiResult<u64> {
        Err(WasiError::EBADF)
    }

    fn advise(
        &self,
        _advice: wasi::__wasi_advice_t,
        _offset: wasi::__wasi_filesize_t,
        _len: wasi::__wasi_filesize_t,
    ) -> WasiResult<()> {
        Err(WasiError::EBADF)
    }

    fn allocate(
        &self,
        _offset: wasi::__wasi_filesize_t,
        _len: wasi::__wasi_filesize_t,
    ) -> WasiResult<()> {
        Err(WasiError::EBADF)
    }

    fn filestat_get(&self) -> WasiResult<wasi::__wasi_filestat_t> {
        Err(WasiError::EBADF)
    }

    fn filestat_set_times(
        &self,
        _atim: Option<FileTime>,
        _mtim: Option<FileTime>,
    ) -> WasiResult<()> {
        Err(WasiError::EBADF)
    }

    fn filestat_set_size(&self, _st_size: wasi::__wasi_filesize_t) -> WasiResult<()> {
        Err(WasiError::EBADF)
    }

    fn fdstat_set_flags(&mut self, _fdflags: wasi::__wasi_fdflags_t) -> WasiResult<()> {
        Err(WasiError::EBADF)
    }

    fn read_vectored(&mut self, _iovs: &mut [io::IoSliceMut]) -> WasiResult<usize> {
        Err(WasiError::EBADF)
    }

    fn get_file_type(&self) -> wasi::__wasi_filetype_t;

    fn get_rights_base(&self) -> wasi::__wasi_rights_t {
        0
    }

    fn get_rights_inheriting(&self) -> wasi::__wasi_rights_t {
        0
    }
}

pub trait FileContents {
    /// The implementation-defined maximum size of the store corresponding to a `FileContents`
    /// implementation.
    fn max_size(&self) -> wasi::__wasi_filesize_t;
    /// The current number of bytes this `FileContents` describes.
    fn size(&self) -> wasi::__wasi_filesize_t;
    /// Resize to hold `new_size` number of bytes, or error if this is not possible.
    fn resize(&mut self, new_size: wasi::__wasi_filesize_t) -> WasiResult<()>;
    /// Write a list of `IoSlice` starting at `offset`. `offset` plus the total size of all `iovs`
    /// is guaranteed to not exceed `max_size`. Implementations must not indicate more bytes have
    /// been written than can be held by `iovs`.
    fn pwritev(
        &mut self,
        iovs: &[io::IoSlice],
        offset: wasi::__wasi_filesize_t,
    ) -> WasiResult<usize>;
    /// Read from the file from `offset`, filling a list of `IoSlice`. The returend size must not
    /// be more than the capactiy of `iovs`, and must not exceed the limit reported by
    /// `self.max_size()`.
    fn preadv(
        &self,
        iovs: &mut [io::IoSliceMut],
        offset: wasi::__wasi_filesize_t,
    ) -> WasiResult<usize>;
    /// Write contents from `buf` to this file starting at `offset`. `offset` plus the length of
    /// `buf` is guaranteed to not exceed `max_size`. Implementations must not indicate more bytes
    /// have been written than the size of `buf`.
    fn pwrite(&mut self, buf: &[u8], offset: wasi::__wasi_filesize_t) -> WasiResult<usize>;
    /// Read from the file at `offset`, filling `buf`. The returned size must not be more than the
    /// capacity of `buf`, and `offset` plus the returned size must not exceed `self.max_size()`.
    fn pread(&self, buf: &mut [u8], offset: wasi::__wasi_filesize_t) -> WasiResult<usize>;
}

impl FileContents for VecFileContents {
    fn max_size(&self) -> wasi::__wasi_filesize_t {
        std::usize::MAX as wasi::__wasi_filesize_t
    }

    fn size(&self) -> wasi::__wasi_filesize_t {
        self.content.len() as wasi::__wasi_filesize_t
    }

    fn resize(&mut self, new_size: wasi::__wasi_filesize_t) -> WasiResult<()> {
        let new_size: usize = new_size.try_into().map_err(|_| WasiError::EINVAL)?;
        self.content.resize(new_size, 0);
        Ok(())
    }

    fn preadv(
        &self,
        iovs: &mut [io::IoSliceMut],
        offset: wasi::__wasi_filesize_t,
    ) -> WasiResult<usize> {
        let mut read_total = 0usize;
        for iov in iovs.iter_mut() {
            let read = self.pread(iov, offset)?;
            read_total = read_total.checked_add(read).expect("FileContents::preadv must not be called when reads could total to more bytes than the return value can hold");
        }
        Ok(read_total)
    }

    fn pwritev(
        &mut self,
        iovs: &[io::IoSlice],
        offset: wasi::__wasi_filesize_t,
    ) -> WasiResult<usize> {
        let mut write_total = 0usize;
        for iov in iovs.iter() {
            let written = self.pwrite(iov, offset)?;
            write_total = write_total.checked_add(written).expect("FileContents::pwritev must not be called when writes could total to more bytes than the return value can hold");
        }
        Ok(write_total)
    }

    fn pread(&self, buf: &mut [u8], offset: wasi::__wasi_filesize_t) -> WasiResult<usize> {
        trace!("     | pread(buf.len={}, offset={})", buf.len(), offset);
        let offset: usize = offset.try_into().map_err(|_| WasiError::EINVAL)?;

        let data_remaining = self.content.len().saturating_sub(offset);

        let read_count = std::cmp::min(buf.len(), data_remaining);

        (&mut buf[..read_count]).copy_from_slice(&self.content[offset..][..read_count]);

        let res = Ok(read_count);
        trace!("     | pread={:?}", res);
        res
    }

    fn pwrite(&mut self, buf: &[u8], offset: wasi::__wasi_filesize_t) -> WasiResult<usize> {
        let offset: usize = offset.try_into().map_err(|_| WasiError::EINVAL)?;

        let write_end = offset.checked_add(buf.len()).ok_or(WasiError::EFBIG)?;

        if write_end > self.content.len() {
            self.content.resize(write_end, 0);
        }

        (&mut self.content[offset..][..buf.len()]).copy_from_slice(buf);

        Ok(buf.len())
    }
}

struct VecFileContents {
    content: Vec<u8>,
}

impl VecFileContents {
    fn new() -> Self {
        Self {
            content: Vec::new(),
        }
    }
}

/// An `InMemoryFile` is a shared handle to some underlying data. The relationship is analagous to
/// a filesystem wherein a file descriptor is one view into a possibly-shared underlying collection
/// of data and permissions on a filesystem.
pub struct InMemoryFile {
    cursor: wasi::__wasi_filesize_t,
    parent: Rc<RefCell<Option<Box<dyn VirtualFile>>>>,
    fd_flags: wasi::__wasi_fdflags_t,
    data: Rc<RefCell<Box<dyn FileContents>>>,
}

impl InMemoryFile {
    pub fn memory_backed() -> Self {
        Self {
            cursor: 0,
            parent: Rc::new(RefCell::new(None)),
            fd_flags: 0,
            data: Rc::new(RefCell::new(Box::new(VecFileContents::new()))),
        }
    }

    pub fn new(contents: Box<dyn FileContents>) -> Self {
        Self {
            cursor: 0,
            fd_flags: 0,
            parent: Rc::new(RefCell::new(None)),
            data: Rc::new(RefCell::new(contents)),
        }
    }
}

impl MovableFile for InMemoryFile {
    fn set_parent(&self, new_parent: Option<Box<dyn VirtualFile>>) {
        *self.parent.borrow_mut() = new_parent;
    }
}

impl VirtualFile for InMemoryFile {
    fn fdstat_get(&self) -> wasi::__wasi_fdflags_t {
        self.fd_flags
    }

    fn try_clone(&self) -> io::Result<Box<dyn VirtualFile>> {
        Ok(Box::new(Self {
            cursor: 0,
            fd_flags: self.fd_flags,
            parent: Rc::clone(&self.parent),
            data: Rc::clone(&self.data),
        }))
    }

    fn readlinkat(&self, _path: &Path) -> WasiResult<String> {
        // no symlink support, so always say it's invalid.
        Err(WasiError::ENOTDIR)
    }

    fn openat(
        &self,
        path: &Path,
        read: bool,
        write: bool,
        oflags: wasi::__wasi_oflags_t,
        fd_flags: wasi::__wasi_fdflags_t,
    ) -> WasiResult<Box<dyn VirtualFile>> {
        log::trace!(
            "InMemoryFile::openat(path={:?}, read={:?}, write={:?}, oflags={:?}, fd_flags={:?}",
            path,
            read,
            write,
            oflags,
            fd_flags
        );

        if oflags & wasi::__WASI_OFLAGS_DIRECTORY != 0 {
            log::trace!(
                "InMemoryFile::openat was passed oflags DIRECTORY, but {:?} is a file.",
                path
            );
            log::trace!("  return ENOTDIR");
            return Err(WasiError::ENOTDIR);
        }

        if path == Path::new(".") {
            return self.try_clone().map_err(Into::into);
        } else if path == Path::new("..") {
            match &*self.parent.borrow() {
                Some(file) => file.try_clone().map_err(Into::into),
                None => self.try_clone().map_err(Into::into),
            }
        } else {
            Err(WasiError::EACCES)
        }
    }

    fn remove_directory(&self, _path: &str) -> WasiResult<()> {
        Err(WasiError::ENOTDIR)
    }

    fn unlink_file(&self, _path: &str) -> WasiResult<()> {
        Err(WasiError::ENOTDIR)
    }

    fn fdstat_set_flags(&mut self, fdflags: wasi::__wasi_fdflags_t) -> WasiResult<()> {
        self.fd_flags = fdflags;
        Ok(())
    }

    fn write_vectored(&mut self, iovs: &[io::IoSlice]) -> WasiResult<usize> {
        trace!("write_vectored(iovs={:?})", iovs);
        let mut data = self.data.borrow_mut();

        let append_mode = self.fd_flags & wasi::__WASI_FDFLAGS_APPEND != 0;
        trace!("     | fd_flags={:o}", self.fd_flags);

        // If this file is in append mode, we write to the end.
        let write_start = if append_mode {
            data.size()
        } else {
            self.cursor
        };

        let max_size = iovs
            .iter()
            .map(|iov| {
                let cast_iovlen: wasi32::size_t = iov
                    .len()
                    .try_into()
                    .expect("iovec are bounded by wasi max sizes");
                cast_iovlen
            })
            .fold(Some(0u32), |len, iov| len.and_then(|x| x.checked_add(iov)))
            .expect("write_vectored will not be called with invalid iovs");

        if let Some(end) = write_start.checked_add(max_size as wasi::__wasi_filesize_t) {
            if end > data.max_size() {
                return Err(WasiError::EFBIG);
            }
        } else {
            return Err(WasiError::EFBIG);
        }

        trace!("     | *write_start={:?}", write_start);
        let written = data.pwritev(iovs, write_start)?;

        // If we are not appending, adjust the cursor appropriately for the write, too. This can't
        // overflow, as we checked against that before writing any data.
        if !append_mode {
            self.cursor += written as u64;
        }

        Ok(written)
    }

    fn read_vectored(&mut self, iovs: &mut [io::IoSliceMut]) -> WasiResult<usize> {
        trace!("read_vectored(iovs={:?})", iovs);
        trace!("     | *read_start={:?}", self.cursor);
        self.data.borrow_mut().preadv(iovs, self.cursor)
    }

    fn pread(&self, buf: &mut [u8], offset: wasi::__wasi_filesize_t) -> WasiResult<usize> {
        self.data.borrow_mut().pread(buf, offset)
    }

    fn pwrite(&self, buf: &mut [u8], offset: wasi::__wasi_filesize_t) -> WasiResult<usize> {
        self.data.borrow_mut().pwrite(buf, offset)
    }

    fn seek(&mut self, offset: SeekFrom) -> WasiResult<wasi::__wasi_filesize_t> {
        let content_len = self.data.borrow().size();
        match offset {
            SeekFrom::Current(offset) => {
                let new_cursor = if offset < 0 {
                    self.cursor
                        .checked_sub(offset.wrapping_neg() as u64)
                        .ok_or(WasiError::EINVAL)?
                } else {
                    self.cursor
                        .checked_add(offset as u64)
                        .ok_or(WasiError::EINVAL)?
                };
                self.cursor = std::cmp::min(content_len, new_cursor);
            }
            SeekFrom::End(offset) => {
                // A negative offset from the end would be past the end of the file,
                let offset: u64 = offset.try_into().map_err(|_| WasiError::EINVAL)?;
                self.cursor = content_len.saturating_sub(offset);
            }
            SeekFrom::Start(offset) => {
                // A negative offset from the end would be before the start of the file.
                let offset: u64 = offset.try_into().map_err(|_| WasiError::EINVAL)?;
                self.cursor = std::cmp::min(content_len, offset);
            }
        }

        Ok(self.cursor)
    }

    fn advise(
        &self,
        advice: wasi::__wasi_advice_t,
        _offset: wasi::__wasi_filesize_t,
        _len: wasi::__wasi_filesize_t,
    ) -> WasiResult<()> {
        // we'll just ignore advice for now, unless it's totally invalid
        match advice {
            wasi::__WASI_ADVICE_DONTNEED
            | wasi::__WASI_ADVICE_SEQUENTIAL
            | wasi::__WASI_ADVICE_WILLNEED
            | wasi::__WASI_ADVICE_NOREUSE
            | wasi::__WASI_ADVICE_RANDOM
            | wasi::__WASI_ADVICE_NORMAL => Ok(()),
            _ => Err(WasiError::EINVAL),
        }
    }

    fn allocate(
        &self,
        offset: wasi::__wasi_filesize_t,
        len: wasi::__wasi_filesize_t,
    ) -> WasiResult<()> {
        let new_limit = offset.checked_add(len).ok_or(WasiError::EFBIG)?;
        let mut data = self.data.borrow_mut();

        if new_limit > data.max_size() {
            return Err(WasiError::EFBIG);
        }

        if new_limit > data.size() {
            data.resize(new_limit)?;
        }

        Ok(())
    }

    fn filestat_set_size(&self, st_size: wasi::__wasi_filesize_t) -> WasiResult<()> {
        let mut data = self.data.borrow_mut();
        if st_size > data.max_size() {
            return Err(WasiError::EFBIG);
        }
        data.resize(st_size)
    }

    fn filestat_get(&self) -> WasiResult<wasi::__wasi_filestat_t> {
        let stat = wasi::__wasi_filestat_t {
            dev: 0,
            ino: 0,
            nlink: 0,
            size: self.data.borrow().size(),
            atim: 0,
            ctim: 0,
            mtim: 0,
            filetype: self.get_file_type(),
        };
        Ok(stat)
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
    // All copies of this `VirtualDir` must share `parent`, and changes in one copy's `parent`
    // must be reflected in all handles, so they share `Rc` of an underlying `parent`.
    parent: Rc<RefCell<Option<Box<dyn VirtualFile>>>>,
    entries: Rc<RefCell<HashMap<PathBuf, Box<dyn VirtualFile>>>>,
}

impl VirtualDir {
    pub fn new(writable: bool) -> Self {
        Self {
            writable,
            parent: Rc::new(RefCell::new(None)),
            entries: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    #[allow(dead_code)]
    pub fn with_dir<P: AsRef<Path>>(mut self, dir: Self, path: P) -> Self {
        self.add_dir(dir, path);
        self
    }

    #[allow(dead_code)]
    pub fn add_dir<P: AsRef<Path>>(&mut self, dir: Self, path: P) {
        let entry = Box::new(dir);
        entry.set_parent(Some(self.try_clone().expect("can clone self")));
        self.entries
            .borrow_mut()
            .insert(path.as_ref().to_owned(), entry);
    }

    #[allow(dead_code)]
    pub fn with_file<P: AsRef<Path>>(mut self, content: Box<dyn FileContents>, path: P) -> Self {
        self.add_file(content, path);
        self
    }

    #[allow(dead_code)]
    pub fn add_file<P: AsRef<Path>>(&mut self, content: Box<dyn FileContents>, path: P) {
        let entry = Box::new(InMemoryFile::new(content));
        entry.set_parent(Some(self.try_clone().expect("can clone self")));
        self.entries
            .borrow_mut()
            .insert(path.as_ref().to_owned(), entry);
    }
}

impl MovableFile for VirtualDir {
    fn set_parent(&self, new_parent: Option<Box<dyn VirtualFile>>) {
        *self.parent.borrow_mut() = new_parent;
    }
}

const SELF_DIR_COOKIE: u32 = 0;
const PARENT_DIR_COOKIE: u32 = 1;

// This MUST be the number of constants above. This limit is used to prevent allocation of files
// that would wrap and be mapped to the same dir cookies as `self` or `parent`.
const RESERVED_ENTRY_COUNT: u32 = 2;

impl VirtualFile for VirtualDir {
    fn try_clone(&self) -> io::Result<Box<dyn VirtualFile>> {
        Ok(Box::new(Self {
            writable: self.writable,
            parent: Rc::clone(&self.parent),
            entries: Rc::clone(&self.entries),
        }))
    }

    fn readlinkat(&self, _path: &Path) -> WasiResult<String> {
        // Files are not symbolic links or directories, faithfully report ENOTDIR.
        Err(WasiError::ENOTDIR)
    }

    fn openat(
        &self,
        path: &Path,
        read: bool,
        write: bool,
        oflags: wasi::__wasi_oflags_t,
        fd_flags: wasi::__wasi_fdflags_t,
    ) -> WasiResult<Box<dyn VirtualFile>> {
        log::trace!(
            "VirtualDir::openat(path={:?}, read={:?}, write={:?}, oflags={:?}, fd_flags={:?}",
            path,
            read,
            write,
            oflags,
            fd_flags
        );

        if path == Path::new(".") {
            return self.try_clone().map_err(Into::into);
        } else if path == Path::new("..") {
            match &*self.parent.borrow() {
                Some(file) => {
                    return file.try_clone().map_err(Into::into);
                }
                None => {
                    return self.try_clone().map_err(Into::into);
                }
            }
        }

        // openat may have been passed a path with a trailing slash, but files are mapped to paths
        // with trailing slashes normalized out.
        let file_name = path.file_name().ok_or(WasiError::EINVAL)?;
        let mut entries = self.entries.borrow_mut();
        let entry_count = entries.len();
        match entries.entry(Path::new(file_name).to_path_buf()) {
            Entry::Occupied(e) => {
                let creat_excl_mask = wasi::__WASI_OFLAGS_CREAT | wasi::__WASI_OFLAGS_EXCL;
                if (oflags & creat_excl_mask) == creat_excl_mask {
                    log::trace!("VirtualDir::openat was passed oflags CREAT|EXCL, but the file {:?} exists.", file_name);
                    log::trace!("  return EEXIST");
                    return Err(WasiError::EEXIST);
                }

                if (oflags & wasi::__WASI_OFLAGS_DIRECTORY) != 0
                    && e.get().get_file_type() != wasi::__WASI_FILETYPE_DIRECTORY
                {
                    log::trace!(
                        "VirtualDir::openat was passed oflags DIRECTORY, but {:?} is a file.",
                        file_name
                    );
                    log::trace!("  return ENOTDIR");
                    return Err(WasiError::ENOTDIR);
                }

                e.get().try_clone().map_err(Into::into)
            }
            Entry::Vacant(v) => {
                if self.writable {
                    // Enforce a hard limit at `u32::MAX - 2` files.
                    // This is to have a constant limit (rather than target-dependent limit we
                    // would have with `usize`. The limit is the full `u32` range minus two so we
                    // can reserve "self" and "parent" cookie values.
                    if entry_count >= (std::u32::MAX - RESERVED_ENTRY_COUNT) as usize {
                        return Err(WasiError::ENOSPC);
                    }

                    log::trace!(
                        "VirtualDir::openat creating an InMemoryFile named {}",
                        path.display()
                    );

                    let mut file = Box::new(InMemoryFile::memory_backed());
                    file.fd_flags = fd_flags;
                    file.set_parent(Some(self.try_clone().expect("can clone self")));
                    v.insert(file).try_clone().map_err(Into::into)
                } else {
                    Err(WasiError::EACCES)
                }
            }
        }
    }

    fn remove_directory(&self, path: &str) -> WasiResult<()> {
        let trimmed_path = path.trim_end_matches('/');
        let mut entries = self.entries.borrow_mut();
        match entries.entry(Path::new(trimmed_path).to_path_buf()) {
            Entry::Occupied(e) => {
                // first, does this name a directory?
                if e.get().get_file_type() != wasi::__WASI_FILETYPE_DIRECTORY {
                    return Err(WasiError::ENOTDIR);
                }

                // Okay, but is the directory empty?
                let iter = e.get().readdir(wasi::__WASI_DIRCOOKIE_START)?;
                if iter.skip(RESERVED_ENTRY_COUNT as usize).next().is_some() {
                    return Err(WasiError::ENOTEMPTY);
                }

                // Alright, it's an empty directory. We can remove it.
                let removed = e.remove_entry();

                // And sever the file's parent ref to avoid Rc cycles.
                removed.1.set_parent(None);

                Ok(())
            }
            Entry::Vacant(_) => {
                log::trace!(
                    "VirtualDir::remove_directory failed to remove {}, no such entry",
                    trimmed_path
                );
                Err(WasiError::ENOENT)
            }
        }
    }

    fn unlink_file(&self, path: &str) -> WasiResult<()> {
        let trimmed_path = path.trim_end_matches('/');

        // Special case: we may be unlinking this directory itself if path is `"."`. In that case,
        // fail with EISDIR, since this is a directory. Alternatively, we may be unlinking `".."`,
        // which is bound the same way, as this is by definition contained in a directory.
        if trimmed_path == "." || trimmed_path == ".." {
            return Err(WasiError::EISDIR);
        }

        let mut entries = self.entries.borrow_mut();
        match entries.entry(Path::new(trimmed_path).to_path_buf()) {
            Entry::Occupied(e) => {
                // Directories must be removed through `remove_directory`, not `unlink_file`.
                if e.get().get_file_type() == wasi::__WASI_FILETYPE_DIRECTORY {
                    return Err(WasiError::EISDIR);
                }

                let removed = e.remove_entry();

                // Sever the file's parent ref to avoid Rc cycles.
                removed.1.set_parent(None);

                Ok(())
            }
            Entry::Vacant(_) => {
                log::trace!(
                    "VirtualDir::unlink_file failed to remove {}, no such entry",
                    trimmed_path
                );
                Err(WasiError::ENOENT)
            }
        }
    }

    fn create_directory(&self, path: &Path) -> WasiResult<()> {
        let mut entries = self.entries.borrow_mut();
        match entries.entry(path.to_owned()) {
            Entry::Occupied(_) => Err(WasiError::EEXIST),
            Entry::Vacant(v) => {
                if self.writable {
                    let new_dir = Box::new(Self::new(true));
                    new_dir.set_parent(Some(self.try_clone()?));
                    v.insert(new_dir);
                    Ok(())
                } else {
                    Err(WasiError::EACCES)
                }
            }
        }
    }

    fn write_vectored(&mut self, _iovs: &[io::IoSlice]) -> WasiResult<usize> {
        Err(WasiError::EBADF)
    }

    fn readdir(
        &self,
        cookie: wasi::__wasi_dircookie_t,
    ) -> WasiResult<Box<dyn Iterator<Item = WasiResult<Dirent>>>> {
        struct VirtualDirIter {
            start: u32,
            entries: Rc<RefCell<HashMap<PathBuf, Box<dyn VirtualFile>>>>,
        }
        impl Iterator for VirtualDirIter {
            type Item = WasiResult<Dirent>;

            fn next(&mut self) -> Option<Self::Item> {
                log::trace!("VirtualDirIter::next continuing from {}", self.start);
                if self.start == SELF_DIR_COOKIE {
                    self.start += 1;
                    return Some(Ok(Dirent {
                        name: ".".to_owned(),
                        ftype: FileType::from_wasi(wasi::__WASI_FILETYPE_DIRECTORY)
                            .expect("directories are valid file types"),
                        ino: 0,
                        cookie: self.start as u64,
                    }));
                }
                if self.start == PARENT_DIR_COOKIE {
                    self.start += 1;
                    return Some(Ok(Dirent {
                        name: "..".to_owned(),
                        ftype: FileType::from_wasi(wasi::__WASI_FILETYPE_DIRECTORY)
                            .expect("directories are valid file types"),
                        ino: 0,
                        cookie: self.start as u64,
                    }));
                }

                let entries = self.entries.borrow();

                // Adjust `start` to be an appropriate number of HashMap entries.
                let start = self.start - RESERVED_ENTRY_COUNT;
                if start as usize >= entries.len() {
                    return None;
                }

                self.start += 1;

                let (path, file) = entries
                    .iter()
                    .skip(start as usize)
                    .next()
                    .expect("seeked less than the length of entries");

                let entry = Dirent {
                    name: path
                        .to_str()
                        .expect("wasi paths are valid utf8 strings")
                        .to_owned(),
                    ftype: FileType::from_wasi(file.get_file_type())
                        .expect("virtfs reports valid wasi file types"),
                    ino: 0,
                    cookie: self.start as u64,
                };

                Some(Ok(entry))
            }
        }
        let cookie = match cookie.try_into() {
            Ok(cookie) => cookie,
            Err(_) => {
                // Cookie is larger than u32. it doesn't seem like there's an explicit error
                // condition in POSIX or WASI, so just start from the start?
                0
            }
        };
        Ok(Box::new(VirtualDirIter {
            start: cookie,
            entries: Rc::clone(&self.entries),
        }))
    }

    fn filestat_get(&self) -> WasiResult<wasi::__wasi_filestat_t> {
        let stat = wasi::__wasi_filestat_t {
            dev: 0,
            ino: 0,
            nlink: 0,
            size: 0,
            atim: 0,
            ctim: 0,
            mtim: 0,
            filetype: self.get_file_type(),
        };
        Ok(stat)
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

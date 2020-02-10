use crate::host::Dirent;
use crate::host::FileType;
use crate::{wasi, Error, Result};
use filetime::FileTime;
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub trait MovableFile {
    fn set_parent(&self, new_parent: Option<Box<dyn VirtualFile>>);
}

pub trait VirtualFile: MovableFile {
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
        oflags: wasi::__wasi_oflags_t,
        fd_flags: wasi::__wasi_fdflags_t,
    ) -> Result<Box<dyn VirtualFile>>;

    fn remove_directory(&self, path: &str) -> Result<()>;

    fn unlink_file(&self, path: &str) -> Result<()>;

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

    fn fdstat_set_flags(&mut self, _fdflags: wasi::__wasi_fdflags_t) -> Result<()> {
        Err(Error::EBADF)
    }

    fn read_vectored(&mut self, _iovs: &mut [io::IoSliceMut]) -> Result<usize> {
        Err(Error::EBADF)
    }

    fn get_file_type(&self) -> wasi::__wasi_filetype_t;

    fn get_rights_base(&self) -> wasi::__wasi_rights_t;

    fn get_rights_inheriting(&self) -> wasi::__wasi_rights_t;
}

struct FileContents {
    content: Vec<u8>,
    flags: wasi::__wasi_fdflags_t,
}

impl FileContents {
    fn new(fd_flags: wasi::__wasi_fdflags_t) -> Self {
        Self {
            content: Vec::new(),
            flags: fd_flags,
        }
    }

    fn fd_flags(&self) -> &wasi::__wasi_fdflags_t {
        &self.flags
    }

    fn fd_flags_mut(&mut self) -> &mut wasi::__wasi_fdflags_t {
        &mut self.flags
    }

    fn content_mut(&mut self) -> &mut Vec<u8> {
        &mut self.content
    }
}

/// An `InMemoryFile` is a shared handle to some underlying data. The relationship is analagous to
/// a filesystem wherein a file descriptor is one view into a possibly-shared underlying collection
/// of data and permissions on a filesystem.
pub struct InMemoryFile {
    cursor: usize,
    parent: Rc<RefCell<Option<Box<dyn VirtualFile>>>>,
    data: Rc<RefCell<FileContents>>,
}

impl InMemoryFile {
    pub fn new(fd_flags: wasi::__wasi_fdflags_t) -> Self {
        Self {
            cursor: 0,
            parent: Rc::new(RefCell::new(None)),
            data: Rc::new(RefCell::new(FileContents::new(fd_flags))),
        }
    }

    pub fn append(&self, data: &[u8]) {
        self.data.borrow_mut().content_mut().extend_from_slice(data);
    }
}

impl MovableFile for InMemoryFile {
    fn set_parent(&self, new_parent: Option<Box<dyn VirtualFile>>) {
        *self.parent.borrow_mut() = new_parent;
    }
}

impl VirtualFile for InMemoryFile {
    fn fdstat_get(&self) -> wasi::__wasi_fdflags_t {
        *self.data.borrow().fd_flags()
    }

    fn try_clone(&self) -> io::Result<Box<dyn VirtualFile>> {
        Ok(Box::new(InMemoryFile {
            cursor: 0,
            parent: Rc::clone(&self.parent),
            data: Rc::clone(&self.data),
        }))
    }

    fn readlinkat(&self, _path: &Path) -> Result<String> {
        // no symlink support, so always say it's invalid.
        Err(Error::ENOTDIR)
    }

    fn openat(
        &self,
        path: &Path,
        read: bool,
        write: bool,
        oflags: wasi::__wasi_oflags_t,
        fd_flags: wasi::__wasi_fdflags_t,
    ) -> Result<Box<dyn VirtualFile>> {
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
            return Err(Error::ENOTDIR);
        }

        if path == Path::new(".") {
            return self.try_clone().map_err(Into::into);
        } else if path == Path::new("..") {
            match &*self.parent.borrow() {
                Some(file) => file.try_clone().map_err(Into::into),
                None => self.try_clone().map_err(Into::into),
            }
        } else {
            Err(Error::EACCES)
        }
    }

    fn remove_directory(&self, _path: &str) -> Result<()> {
        Err(Error::ENOTDIR)
    }

    fn unlink_file(&self, _path: &str) -> Result<()> {
        Err(Error::ENOTDIR)
    }

    fn write_vectored(&mut self, iovs: &[io::IoSlice]) -> Result<usize> {
        let mut data = self.data.borrow_mut();

        let append_mode = data.fd_flags() & wasi::__WASI_FDFLAGS_APPEND != 0;

        let content = data.content_mut();

        // If this file is in append mode, we write to the end.
        let write_start = if append_mode {
            content.len()
        } else {
            self.cursor
        };

        let mut cursor = write_start;
        for iov in iovs {
            for el in iov.iter() {
                if cursor == content.len() {
                    content.push(*el);
                } else {
                    content[cursor] = *el;
                }
                cursor += 1;
            }
        }

        let len = cursor - write_start;

        // If we are not appending, adjust the cursor appropriately for the write, too.
        if !append_mode {
            self.cursor = cursor;
        }

        Ok(len)
    }

    fn fdstat_set_flags(&mut self, fdflags: wasi::__wasi_fdflags_t) -> Result<()> {
        *self.data.borrow_mut().fd_flags_mut() = fdflags;
        Ok(())
    }

    fn read_vectored(&mut self, iovs: &mut [io::IoSliceMut]) -> Result<usize> {
        let data = self.data.borrow();
        let mut cursor = self.cursor;
        for iov in iovs {
            for i in 0..iov.len() {
                if cursor >= data.content.len() {
                    let count = cursor - self.cursor;
                    self.cursor = cursor;
                    return Ok(count);
                }
                iov[i] = data.content[cursor];
                cursor += 1;
            }
        }

        let count = cursor - self.cursor;
        self.cursor = cursor;
        Ok(count)
    }

    fn pread(&self, buf: &mut [u8], offset: u64) -> Result<usize> {
        let data = self.data.borrow();
        let mut cursor = offset;
        for i in 0..buf.len() {
            if cursor >= data.content.len() as u64 {
                let count = cursor - offset;
                return Ok(count as usize);
            }
            buf[i] = data.content[cursor as usize];
            cursor += 1;
        }

        let count = cursor - offset;
        Ok(count as usize)
    }

    fn pwrite(&self, buf: &mut [u8], offset: u64) -> Result<usize> {
        let mut data = self.data.borrow_mut();
        let mut cursor = offset;
        for el in buf.iter() {
            if cursor == data.content.len() as u64 {
                data.content.push(*el);
            } else {
                data.content[cursor as usize] = *el;
            }
            cursor += 1;
        }
        Ok(buf.len())
    }

    fn seek(&mut self, offset: SeekFrom) -> Result<u64> {
        let content_len = self.data.borrow().content.len();
        match offset {
            SeekFrom::Current(offset) => {
                let new_cursor = if offset < 0 {
                    self.cursor
                        .checked_sub(-offset as usize)
                        .ok_or(Error::EINVAL)?
                } else {
                    self.cursor
                        .checked_add(offset as usize)
                        .ok_or(Error::EINVAL)?
                };
                self.cursor = std::cmp::min(content_len, new_cursor);
            }
            SeekFrom::End(offset) => {
                self.cursor = content_len.saturating_sub(offset as usize);
            }
            SeekFrom::Start(offset) => {
                self.cursor = std::cmp::min(content_len, offset as usize);
            }
        }

        Ok(self.cursor as u64)
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

    fn allocate(
        &self,
        offset: wasi::__wasi_filesize_t,
        len: wasi::__wasi_filesize_t,
    ) -> Result<()> {
        let new_limit = offset + len;
        let mut data = self.data.borrow_mut();

        if new_limit > data.content.len() as u64 {
            data.content.resize(new_limit as usize, 0);
        }

        Ok(())
    }

    fn filestat_set_size(&self, st_size: wasi::__wasi_filesize_t) -> Result<()> {
        if st_size > std::usize::MAX as u64 {
            return Err(Error::EFBIG);
        }
        self.data.borrow_mut().content.resize(st_size as usize, 0);
        Ok(())
    }

    fn filestat_get(&self) -> Result<wasi::__wasi_filestat_t> {
        let stat = wasi::__wasi_filestat_t {
            dev: 0,
            ino: 0,
            nlink: 0,
            size: self.data.borrow().content.len() as u64,
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
        VirtualDir {
            writable,
            parent: Rc::new(RefCell::new(None)),
            entries: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn with_file<P: AsRef<Path>>(self, file: Box<dyn VirtualFile>, path: P) -> Self {
        file.set_parent(Some(self.try_clone().expect("can clone self")));
        self.entries
            .borrow_mut()
            .insert(path.as_ref().to_owned(), file);
        self
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
        Ok(Box::new(VirtualDir {
            writable: self.writable,
            parent: Rc::clone(&self.parent),
            entries: Rc::clone(&self.entries),
        }))
    }

    fn readlinkat(&self, _path: &Path) -> Result<String> {
        // Files are not symbolic links or directories, faithfully report ENOTDIR.
        Err(Error::ENOTDIR)
    }

    fn openat(
        &self,
        path: &Path,
        read: bool,
        write: bool,
        oflags: wasi::__wasi_oflags_t,
        fd_flags: wasi::__wasi_fdflags_t,
    ) -> Result<Box<dyn VirtualFile>> {
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
        let file_name = path.file_name().ok_or(Error::EINVAL)?;
        let mut entries = self.entries.borrow_mut();
        let entry_count = entries.len();
        match entries.entry(Path::new(file_name).to_path_buf()) {
            Entry::Occupied(e) => {
                let creat_excl_mask = wasi::__WASI_OFLAGS_CREAT | wasi::__WASI_OFLAGS_EXCL;
                if (oflags & creat_excl_mask) == creat_excl_mask {
                    log::trace!("VirtualDir::openat was passed oflags CREAT|EXCL, but the file {:?} exists.", file_name);
                    log::trace!("  return EEXIST");
                    return Err(Error::EEXIST);
                }

                if (oflags & wasi::__WASI_OFLAGS_DIRECTORY) != 0
                    && e.get().get_file_type() != wasi::__WASI_FILETYPE_DIRECTORY
                {
                    log::trace!(
                        "VirtualDir::openat was passed oflags DIRECTORY, but {:?} is a file.",
                        file_name
                    );
                    log::trace!("  return ENOTDIR");
                    return Err(Error::ENOTDIR);
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
                        return Err(Error::ENOSPC);
                    }

                    log::trace!(
                        "VirtualDir::openat creating an InMemoryFile named {}",
                        path.display()
                    );

                    let file = Box::new(InMemoryFile::new(fd_flags));
                    file.set_parent(Some(self.try_clone().expect("can clone self")));
                    v.insert(file).try_clone().map_err(Into::into)
                } else {
                    Err(Error::EACCES)
                }
            }
        }
    }

    fn remove_directory(&self, path: &str) -> Result<()> {
        let trimmed_path = path.trim_end_matches('/');
        let mut entries = self.entries.borrow_mut();
        match entries.entry(Path::new(trimmed_path).to_path_buf()) {
            Entry::Occupied(e) => {
                // first, does this name a directory?
                if e.get().get_file_type() != wasi::__WASI_FILETYPE_DIRECTORY {
                    return Err(Error::ENOTDIR);
                }

                // Okay, but is the directory empty?
                let iter = e.get().readdir(wasi::__WASI_DIRCOOKIE_START)?;
                if iter.skip(RESERVED_ENTRY_COUNT as usize).next().is_some() {
                    return Err(Error::ENOTEMPTY);
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
                Err(Error::ENOENT)
            }
        }
    }

    fn unlink_file(&self, path: &str) -> Result<()> {
        let trimmed_path = path.trim_end_matches('/');

        // Special case: we may be unlinking this directory itself if path is `"."`. In that case,
        // fail with EISDIR, since this is a directory. Alternatively, we may be unlinking `".."`,
        // which is bound the same way, as this is by definition contained in a directory.
        if trimmed_path == "." || trimmed_path == ".." {
            return Err(Error::EISDIR);
        }

        let mut entries = self.entries.borrow_mut();
        match entries.entry(Path::new(trimmed_path).to_path_buf()) {
            Entry::Occupied(e) => {
                // Directories must be removed through `remove_directory`, not `unlink_file`.
                if e.get().get_file_type() == wasi::__WASI_FILETYPE_DIRECTORY {
                    return Err(Error::EISDIR);
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
                Err(Error::ENOENT)
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
                    let new_dir = Box::new(VirtualDir::new(true));
                    new_dir.set_parent(Some(self.try_clone()?));
                    v.insert(new_dir);
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

    fn readdir(
        &self,
        cookie: wasi::__wasi_dircookie_t,
    ) -> Result<Box<dyn Iterator<Item = Result<Dirent>>>> {
        struct VirtualDirIter {
            start: u32,
            entries: Rc<RefCell<HashMap<PathBuf, Box<dyn VirtualFile>>>>,
        }
        impl Iterator for VirtualDirIter {
            type Item = Result<Dirent>;

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

    fn filestat_get(&self) -> Result<wasi::__wasi_filestat_t> {
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

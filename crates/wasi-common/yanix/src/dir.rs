use crate::{
    sys::dir::{iter_impl, EntryImpl},
    Errno, Result,
};
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};
use std::{ffi::CStr, ops::Deref, ptr};

pub use crate::sys::EntryExt;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Dir(pub(crate) ptr::NonNull<libc::DIR>);

impl Dir {
    /// Takes the ownership of the passed-in descriptor-based object,
    /// and creates a new instance of `Dir`.
    #[inline]
    pub fn from<F: IntoRawFd>(fd: F) -> Result<Self> {
        let fd = fd.into_raw_fd();
        unsafe { Self::from_fd(fd) }
    }

    unsafe fn from_fd(fd: RawFd) -> Result<Self> {
        let d = libc::fdopendir(fd);
        if let Some(d) = ptr::NonNull::new(d) {
            Ok(Self(d))
        } else {
            let e = Errno::last();
            libc::close(fd);
            Err(e.into())
        }
    }

    /// Set the position of the directory stream, see `seekdir(3)`.
    #[cfg(not(target_os = "android"))]
    pub fn seek(&mut self, loc: SeekLoc) {
        unsafe { libc::seekdir(self.0.as_ptr(), loc.0) }
    }

    /// Reset directory stream, see `rewinddir(3)`.
    pub fn rewind(&mut self) {
        unsafe { libc::rewinddir(self.0.as_ptr()) }
    }

    /// Get the current position in the directory stream.
    ///
    /// If this location is given to `Dir::seek`, the entries up to the previously returned
    /// will be omitted and the iteration will start from the currently pending directory entry.
    #[cfg(not(target_os = "android"))]
    #[allow(dead_code)]
    pub fn tell(&self) -> SeekLoc {
        let loc = unsafe { libc::telldir(self.0.as_ptr()) };
        SeekLoc(loc)
    }
}

unsafe impl Send for Dir {}

impl AsRawFd for Dir {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { libc::dirfd(self.0.as_ptr()) }
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.0.as_ptr()) };
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Entry(pub(crate) EntryImpl);

impl Entry {
    /// Returns the file name of this directory entry.
    pub fn file_name(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.0.d_name.as_ptr()) }
    }

    /// Returns the type of this directory entry.
    pub fn file_type(&self) -> FileType {
        unsafe { FileType::from_raw(self.0.d_type) }
    }
}

#[cfg(not(target_os = "android"))]
#[derive(Clone, Copy, Debug)]
pub struct SeekLoc(libc::c_long);

#[cfg(not(target_os = "android"))]
impl SeekLoc {
    pub unsafe fn from_raw(loc: i64) -> Self {
        Self(loc.into())
    }

    pub fn to_raw(&self) -> i64 {
        self.0.into()
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum FileType {
    CharacterDevice = libc::DT_CHR,
    Directory = libc::DT_DIR,
    BlockDevice = libc::DT_BLK,
    RegularFile = libc::DT_REG,
    Symlink = libc::DT_LNK,
    Fifo = libc::DT_FIFO,
    Socket = libc::DT_SOCK,
    Unknown = libc::DT_UNKNOWN,
}

impl FileType {
    pub unsafe fn from_raw(file_type: u8) -> Self {
        match file_type {
            libc::DT_CHR => Self::CharacterDevice,
            libc::DT_DIR => Self::Directory,
            libc::DT_BLK => Self::BlockDevice,
            libc::DT_REG => Self::RegularFile,
            libc::DT_LNK => Self::Symlink,
            libc::DT_SOCK => Self::Socket,
            libc::DT_FIFO => Self::Fifo,
            /* libc::DT_UNKNOWN */ _ => Self::Unknown,
        }
    }

    pub fn to_raw(&self) -> u8 {
        match self {
            Self::CharacterDevice => libc::DT_CHR,
            Self::Directory => libc::DT_DIR,
            Self::BlockDevice => libc::DT_BLK,
            Self::RegularFile => libc::DT_REG,
            Self::Symlink => libc::DT_LNK,
            Self::Socket => libc::DT_SOCK,
            Self::Fifo => libc::DT_FIFO,
            Self::Unknown => libc::DT_UNKNOWN,
        }
    }
}

#[derive(Debug)]
pub struct DirIter<T: Deref<Target = Dir>>(T);

impl<T> DirIter<T>
where
    T: Deref<Target = Dir>,
{
    pub fn new(dir: T) -> Self {
        Self(dir)
    }
}

impl<T> Iterator for DirIter<T>
where
    T: Deref<Target = Dir>,
{
    type Item = Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe { iter_impl(&self.0).map(|x| x.map(Entry)) }
    }
}

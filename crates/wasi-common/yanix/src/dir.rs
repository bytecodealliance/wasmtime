use crate::{
    file::FileType,
    sys::dir::{iter_impl, EntryImpl},
};
use std::convert::TryInto;
use std::io::{Error, Result};
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};
use std::{ffi::CStr, io, ops::Deref, ptr};

pub use crate::sys::EntryExt;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Dir(ptr::NonNull<libc::DIR>);

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
            let e = io::Error::last_os_error();
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

    /// For use by platform-specific implementation code. Returns the raw
    /// underlying state.
    pub(crate) fn as_raw(&self) -> ptr::NonNull<libc::DIR> {
        self.0
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
        FileType::from_dirent_d_type(self.0.d_type)
    }
}

#[cfg(not(target_os = "android"))]
#[derive(Clone, Copy, Debug)]
pub struct SeekLoc(pub(crate) libc::c_long);

#[cfg(not(target_os = "android"))]
impl SeekLoc {
    pub fn to_raw(&self) -> i64 {
        self.0.into()
    }

    pub unsafe fn from_raw(loc: i64) -> Result<Self> {
        // The cookie (or `loc`) is an opaque value, and applications aren't supposed to do
        // arithmetic on them or pick their own values or have any awareness of the numeric
        // range of the values. They're just supposed to pass back in the values that we
        // give them. And any value we give them will be convertable back to `long`,
        // because that's the type the OS gives them to us in. So return an `EINVAL`.
        let loc = loc
            .try_into()
            .map_err(|_| Error::from_raw_os_error(libc::EINVAL))?;
        Ok(Self(loc))
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
        iter_impl(&self.0).map(|x| x.map(Entry))
    }
}

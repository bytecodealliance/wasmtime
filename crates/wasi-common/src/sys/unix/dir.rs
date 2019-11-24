// Based on src/dir.rs from nix
use crate::hostcalls_impl::FileType;
use libc;
use nix::{Error, Result};
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};
use std::{ffi, ptr};

#[cfg(target_os = "linux")]
use libc::dirent64 as dirent;

#[cfg(not(target_os = "linux",))]
use libc::dirent;

/// An open directory.
///
/// This is a lower-level interface than `std::fs::ReadDir`. Notable differences:
///    * can be opened from a file descriptor (as returned by `openat`, perhaps before knowing
///      if the path represents a file or directory).
///    * implements `AsRawFd`, so it can be passed to `fstat`, `openat`, etc.
///      The file descriptor continues to be owned by the `Dir`, so callers must not keep a `RawFd`
///      after the `Dir` is dropped.
///    * can be iterated through multiple times without closing and reopening the file
///      descriptor. Each iteration rewinds when finished.
///    * returns entries for `.` (current directory) and `..` (parent directory).
///    * returns entries' names as a `CStr` (no allocation or conversion beyond whatever libc
///      does).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Dir(pub(crate) ptr::NonNull<libc::DIR>);

impl Dir {
    /// Converts from a descriptor-based object, closing the descriptor on success or failure.
    #[inline]
    pub(crate) fn from<F: IntoRawFd>(fd: F) -> Result<Self> {
        unsafe { Self::from_fd(fd.into_raw_fd()) }
    }

    /// Converts from a file descriptor, closing it on success or failure.
    unsafe fn from_fd(fd: RawFd) -> Result<Self> {
        let d = libc::fdopendir(fd);
        if d.is_null() {
            let e = Error::last();
            libc::close(fd);
            return Err(e);
        };
        // Always guaranteed to be non-null by the previous check
        Ok(Self(ptr::NonNull::new(d).unwrap()))
    }

    /// Set the position of the directory stream, see `seekdir(3)`.
    #[cfg(not(target_os = "android"))]
    pub(crate) fn seek(&mut self, loc: SeekLoc) {
        unsafe { libc::seekdir(self.0.as_ptr(), loc.0) }
    }

    /// Reset directory stream, see `rewinddir(3)`.
    pub(crate) fn rewind(&mut self) {
        unsafe { libc::rewinddir(self.0.as_ptr()) }
    }

    /// Get the current position in the directory stream.
    ///
    /// If this location is given to `Dir::seek`, the entries up to the previously returned
    /// will be omitted and the iteration will start from the currently pending directory entry.
    #[cfg(not(target_os = "android"))]
    #[allow(dead_code)]
    pub(crate) fn tell(&self) -> SeekLoc {
        let loc = unsafe { libc::telldir(self.0.as_ptr()) };
        SeekLoc(loc)
    }
}

// `Dir` is not `Sync`. With the current implementation, it could be, but according to
// https://www.gnu.org/software/libc/manual/html_node/Reading_002fClosing-Directory.html,
// future versions of POSIX are likely to obsolete `readdir_r` and specify that it's unsafe to
// call `readdir` simultaneously from multiple threads.
//
// `Dir` is safe to pass from one thread to another, as it's not reference-counted.
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

/// A directory entry, similar to `std::fs::DirEntry`.
///
/// Note that unlike the std version, this may represent the `.` or `..` entries.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub(crate) struct Entry(pub(crate) dirent);

pub(crate) type Type = FileType;

impl Entry {
    /// Returns the inode number (`d_ino`) of the underlying `dirent`.
    #[cfg(any(
        target_os = "android",
        target_os = "emscripten",
        target_os = "fuchsia",
        target_os = "haiku",
        target_os = "ios",
        target_os = "l4re",
        target_os = "linux",
        target_os = "macos",
        target_os = "solaris"
    ))]
    pub(crate) fn ino(&self) -> u64 {
        self.0.d_ino.into()
    }

    /// Returns the inode number (`d_fileno`) of the underlying `dirent`.
    #[cfg(not(any(
        target_os = "android",
        target_os = "emscripten",
        target_os = "fuchsia",
        target_os = "haiku",
        target_os = "ios",
        target_os = "l4re",
        target_os = "linux",
        target_os = "macos",
        target_os = "solaris"
    )))]
    pub(crate) fn ino(&self) -> u64 {
        u64::from(self.0.d_fileno)
    }

    /// Returns the bare file name of this directory entry without any other leading path component.
    pub(crate) fn file_name(&self) -> &ffi::CStr {
        unsafe { ::std::ffi::CStr::from_ptr(self.0.d_name.as_ptr()) }
    }

    /// Returns the type of this directory entry, if known.
    ///
    /// See platform `readdir(3)` or `dirent(5)` manpage for when the file type is known;
    /// notably, some Linux filesystems don't implement this. The caller should use `stat` or
    /// `fstat` if this returns `None`.
    pub(crate) fn file_type(&self) -> FileType {
        match self.0.d_type {
            libc::DT_CHR => Type::CharacterDevice,
            libc::DT_DIR => Type::Directory,
            libc::DT_BLK => Type::BlockDevice,
            libc::DT_REG => Type::RegularFile,
            libc::DT_LNK => Type::Symlink,
            /* libc::DT_UNKNOWN | libc::DT_SOCK | libc::DT_FIFO */ _ => Type::Unknown,
        }
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn seek_loc(&self) -> SeekLoc {
        unsafe { SeekLoc::from_raw(self.0.d_off) }
    }
}

#[cfg(not(target_os = "android"))]
#[derive(Clone, Copy, Debug)]
pub(crate) struct SeekLoc(libc::c_long);

#[cfg(not(target_os = "android"))]
impl SeekLoc {
    pub(crate) unsafe fn from_raw(loc: i64) -> Self {
        Self(loc.into())
    }

    pub(crate) fn to_raw(&self) -> i64 {
        self.0.into()
    }
}

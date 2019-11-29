// Based on src/dir.rs from nix
use super::{errno::Errno, Error, Result};
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};
use std::{ffi, ops::Deref, ptr};

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "linux",
                 target_os = "android",
                 target_os = "emscripten"))] {
        use libc::dirent64 as dirent;
    } else {
        use libc::dirent;
    }
}

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
pub struct Dir(ptr::NonNull<libc::DIR>);

impl Dir {
    /// Converts from a descriptor-based object, closing the descriptor on success or failure.
    #[inline]
    pub fn from<F: IntoRawFd>(fd: F) -> Result<Self> {
        unsafe { Self::from_fd(fd.into_raw_fd()) }
    }

    /// Converts from a file descriptor, closing it on success or failure.
    unsafe fn from_fd(fd: RawFd) -> Result<Self> {
        let d = libc::fdopendir(fd);
        if d.is_null() {
            let e = Errno::last();
            libc::close(fd);
            return Err(Error::Errno(e));
        };
        // Always guaranteed to be non-null by the previous check
        Ok(Self(ptr::NonNull::new(d).unwrap()))
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
#[derive(Copy, Clone, Debug)]
pub struct Entry {
    dirent: dirent,
    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    loc: SeekLoc,
}

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
    pub fn ino(&self) -> u64 {
        self.dirent.d_ino.into()
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
    pub fn ino(&self) -> u64 {
        u64::from(self.dirent.d_fileno)
    }

    /// Returns the bare file name of this directory entry without any other leading path component.
    pub fn file_name(&self) -> &ffi::CStr {
        unsafe { ::std::ffi::CStr::from_ptr(self.dirent.d_name.as_ptr()) }
    }

    /// Returns the type of this directory entry, if known.
    ///
    /// See platform `readdir(3)` or `dirent(5)` manpage for when the file type is known;
    /// notably, some Linux filesystems don't implement this. The caller should use `stat` or
    /// `fstat` if this returns `None`.
    pub fn file_type(&self) -> FileType {
        unsafe { FileType::from_raw(self.dirent.d_type) }
    }

    #[cfg(any(target_os = "linux", target_os = "android", target_os = "emscripten"))]
    pub fn seek_loc(&self) -> SeekLoc {
        unsafe { SeekLoc::from_raw(self.dirent.d_off) }
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    pub fn seek_loc(&self) -> SeekLoc {
        self.loc
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
        unsafe { iter_impl::get_next_entry(&self.0) }
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
mod iter_impl {
    use super::super::{errno::Errno, Error, Result};
    use super::{Dir, Entry};
    use std::ops::Deref;

    pub(super) unsafe fn get_next_entry<T: Deref<Target = Dir>>(dir: &T) -> Option<Result<Entry>> {
        let errno = Errno::last();
        let dirent = libc::readdir(dir.0.as_ptr());
        if dirent.is_null() {
            if errno != Errno::last() {
                // TODO This should be verified on different BSD-flavours.
                //
                // According to 4.3BSD/POSIX.1-2001 man pages, there was an error
                // if the errno value has changed at some point during the sequence
                // of readdir calls.
                Some(Err(Error::Errno(Errno::last())))
            } else {
                // Not an error. We've simply reached the end of the stream.
                None
            }
        } else {
            let loc = dir.tell();
            Some(Ok(Entry {
                dirent: *dirent,
                loc,
            }))
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "emscripten"))]
mod iter_impl {
    use super::super::{errno::Errno, Result};
    use super::{Dir, Entry};
    use std::ops::Deref;

    pub(super) unsafe fn get_next_entry<T: Deref<Target = Dir>>(dir: &T) -> Option<Result<Entry>> {
        use libc::{dirent64, readdir64_r};
        // Note: POSIX specifies that portable applications should dynamically allocate a
        // buffer with room for a `d_name` field of size `pathconf(..., _PC_NAME_MAX)` plus 1
        // for the NUL byte. It doesn't look like the std library does this; it just uses
        // fixed-sized buffers (and libc's dirent seems to be sized so this is appropriate).
        // Probably fine here too then.
        //
        // See `impl Iterator for ReadDir` [1] for more details.
        // [1] https://github.com/rust-lang/rust/blob/master/src/libstd/sys/unix/fs.rs
        let mut dirent = std::mem::MaybeUninit::<dirent64>::uninit();
        let mut result = std::ptr::null_mut();
        if let Err(e) = Errno::from_success_code(readdir64_r(
            dir.0.as_ptr(),
            dirent.as_mut_ptr(),
            &mut result,
        )) {
            return Some(Err(e));
        }
        if result.is_null() {
            None
        } else {
            assert_eq!(
                result,
                dirent.as_mut_ptr(),
                "readdir_r specification violated"
            );
            Some(Ok(Entry {
                dirent: dirent.assume_init(),
            }))
        }
    }
}

use super::{errno::Errno, Result};
use bitflags::bitflags;
use std::convert::TryInto;
use std::ffi::{CString, OsStr, OsString};
use std::os::unix::prelude::*;

bitflags! {
    pub struct AtFlag: libc::c_int {
        const REMOVEDIR = libc::AT_REMOVEDIR;
        const SYMLINK_FOLLOW = libc::AT_SYMLINK_FOLLOW;
        const SYMLINK_NOFOLLOW = libc::AT_SYMLINK_NOFOLLOW;
        #[cfg(any(target_os = "android", target_os = "linux"))]
        const NO_AUTOMOUNT = libc::AT_NO_AUTOMOUNT;
        #[cfg(any(target_os = "android", target_os = "linux"))]
        const EMPTY_PATH = libc::AT_EMPTY_PATH;
    }
}

bitflags! {
    pub struct Mode: libc::mode_t {
        /// Read, write, and execute for the file owner.
        const IRWXU = libc::S_IRWXU;
        /// Read permission for the file owner.
        const IRUSR = libc::S_IRUSR;
        /// Write permission for the file owner.
        const IWUSR = libc::S_IWUSR;
        /// Execute permission for the file owner.
        const IXUSR = libc::S_IXUSR;
        /// Read, write, and execute for the file's group.
        const IRWXG = libc::S_IRWXG;
        /// Read permission for the file's group.
        const IRGRP = libc::S_IRGRP;
        /// Write permission for the file's group.
        const IWGRP = libc::S_IWGRP;
        /// Execute permission for the file's group.
        const IXGRP = libc::S_IXGRP;
        /// General read, write, and execute permission.
        const IRWXO = libc::S_IRWXO;
        /// General read permission.
        const IROTH = libc::S_IROTH;
        /// General write permission.
        const IWOTH = libc::S_IWOTH;
        /// General execute permission.
        const IXOTH = libc::S_IXOTH;
        /// Set effective user ID at execution time.
        /// This bit is ignored if the object specified by path is a directory.
        const ISUID = libc::S_ISUID as libc::mode_t;
        /// Set effective group ID at execution time.
        const ISGID = libc::S_ISGID as libc::mode_t;
        /// Restricted renames and unlinks for objects within a directory.
        const ISVTX = libc::S_ISVTX as libc::mode_t;
    }
}

bitflags! {
    /// Configuration options for opened files.
    pub struct OFlag: libc::c_int {
        /// Mask for the access mode of the file.
        const ACCMODE = libc::O_ACCMODE;
        /// Use alternate I/O semantics.
        #[cfg(target_os = "netbsd")]
        const ALT_IO = libc::O_ALT_IO;
        /// Open the file in append-only mode.
        const APPEND = libc::O_APPEND;
        /// Generate a signal when input or output becomes possible.
        const ASYNC = libc::O_ASYNC;
        /// Closes the file descriptor once an `execve` call is made.
        /// Also sets the file offset to the beginning of the file.
        const CLOEXEC = libc::O_CLOEXEC;
        /// Create the file if it does not exist.
        const CREAT = libc::O_CREAT;
        /// Try to minimize cache effects of the I/O for this file.
        #[cfg(any(target_os = "android",
                  target_os = "dragonfly",
                  target_os = "freebsd",
                  target_os = "linux",
                  target_os = "netbsd"))]
        const DIRECT = libc::O_DIRECT;
        /// If the specified path isn't a directory, fail.
        const DIRECTORY = libc::O_DIRECTORY;
        /// Implicitly follow each `write()` with an `fdatasync()`.
        #[cfg(any(target_os = "android",
                  target_os = "ios",
                  target_os = "linux",
                  target_os = "macos",
                  target_os = "netbsd",
                  target_os = "openbsd",
                  target_os = "emscripten"))]
        const DSYNC = libc::O_DSYNC;
        /// Error out if a file was not created.
        const EXCL = libc::O_EXCL;
        /// Open for execute only.
        #[cfg(target_os = "freebsd")]
        const EXEC = libc::O_EXEC;
        /// Open with an exclusive file lock.
        #[cfg(any(target_os = "dragonfly",
                  target_os = "freebsd",
                  target_os = "ios",
                  target_os = "macos",
                  target_os = "netbsd",
                  target_os = "openbsd"))]
        const EXLOCK = libc::O_EXLOCK;
        /// Same as `SYNC`.
        #[cfg(any(target_os = "dragonfly",
                  target_os = "freebsd",
                  target_os = "ios",
                  all(target_os = "linux", not(target_env = "musl")),
                  target_os = "macos",
                  target_os = "netbsd",
                  target_os = "openbsd"))]
        const FSYNC = libc::O_FSYNC;
        /// Allow files whose sizes can't be represented in an `off_t` to be opened.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        const LARGEFILE = libc::O_LARGEFILE;
        /// Do not update the file last access time during `read(2)`s.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        const NOATIME = libc::O_NOATIME;
        /// Don't attach the device as the process' controlling terminal.
        const NOCTTY = libc::O_NOCTTY;
        /// Same as `O_NONBLOCK`.
        const NDELAY = libc::O_NDELAY;
        /// `open()` will fail if the given path is a symbolic link.
        const NOFOLLOW = libc::O_NOFOLLOW;
        /// When possible, open the file in nonblocking mode.
        const NONBLOCK = libc::O_NONBLOCK;
        /// Don't deliver `SIGPIPE`.
        #[cfg(target_os = "netbsd")]
        const NOSIGPIPE = libc::O_NOSIGPIPE;
        /// Obtain a file descriptor for low-level access.
        /// The file itself is not opened and other file operations will fail.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        const PATH = libc::O_PATH;
        /// Only allow reading.
        /// This should not be combined with `O_WRONLY` or `O_RDWR`.
        const RDONLY = libc::O_RDONLY;
        /// Only allow writing.
        /// This should not be combined with `O_RDONLY` or `O_RDWR`.
        const WRONLY = libc::O_WRONLY;
        /// Allow both reading and writing.
        /// This should not be combined with `O_WRONLY` or `O_RDONLY`.
        const RDWR = libc::O_RDWR;
        /// Similar to `O_DSYNC` but applies to `read`s instead.
        #[cfg(any(target_os = "linux",
                  target_os = "netbsd",
                  target_os = "openbsd",
                  target_os = "emscripten"))]
        const RSYNC = libc::O_RSYNC;
        /// Skip search permission checks.
        #[cfg(target_os = "netbsd")]
        const SEARCH = libc::O_SEARCH;
        /// Open with a shared file lock.
        #[cfg(any(target_os = "dragonfly",
                  target_os = "freebsd",
                  target_os = "ios",
                  target_os = "macos",
                  target_os = "netbsd",
                  target_os = "openbsd"))]
        const SHLOCK = libc::O_SHLOCK;
        /// Implicitly follow each `write()` with an `fsync()`.
        const SYNC = libc::O_SYNC;
        /// Create an unnamed temporary file.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        const TMPFILE = libc::O_TMPFILE;
        /// Truncate an existing regular file to 0 length if it allows writing.
        const TRUNC = libc::O_TRUNC;
        /// Restore default TTY attributes.
        #[cfg(target_os = "freebsd")]
        const TTY_INIT = libc::O_TTY_INIT;
    }
}

bitflags! {
    pub struct SFlag: libc::mode_t {
        const IFIFO = libc::S_IFIFO;
        const IFCHR = libc::S_IFCHR;
        const IFDIR = libc::S_IFDIR;
        const IFBLK = libc::S_IFBLK;
        const IFREG = libc::S_IFREG;
        const IFLNK = libc::S_IFLNK;
        const IFSOCK = libc::S_IFSOCK;
        const IFMT = libc::S_IFMT;
    }
}

pub fn openat<P: AsRef<OsStr>>(dirfd: RawFd, path: P, oflag: OFlag, mode: Mode) -> Result<RawFd> {
    let path = CString::new(path.as_ref().as_bytes())?;
    Errno::from_result(unsafe {
        libc::openat(
            dirfd,
            path.as_ptr(),
            oflag.bits(),
            libc::c_uint::from(mode.bits()),
        )
    })
}

pub fn readlinkat<P: AsRef<OsStr>>(dirfd: RawFd, path: P) -> Result<OsString> {
    let path = CString::new(path.as_ref().as_bytes())?;
    let buffer = &mut [0u8; libc::PATH_MAX as usize + 1];
    Errno::from_result(unsafe {
        libc::readlinkat(
            dirfd,
            path.as_ptr(),
            buffer.as_mut_ptr() as *mut _,
            buffer.len(),
        )
    })
    .and_then(|nread| {
        let link = OsStr::from_bytes(&buffer[0..nread.try_into()?]);
        Ok(link.into())
    })
}

pub fn mkdirat<P: AsRef<OsStr>>(dirfd: RawFd, path: P, mode: Mode) -> Result<()> {
    let path = CString::new(path.as_ref().as_bytes())?;
    Errno::from_success_code(unsafe { libc::mkdirat(dirfd, path.as_ptr(), mode.bits()) })
}

pub fn linkat<P: AsRef<OsStr>>(
    old_dirfd: RawFd,
    old_path: P,
    new_dirfd: RawFd,
    new_path: P,
    flags: AtFlag,
) -> Result<()> {
    let old_path = CString::new(old_path.as_ref().as_bytes())?;
    let new_path = CString::new(new_path.as_ref().as_bytes())?;
    Errno::from_success_code(unsafe {
        libc::linkat(
            old_dirfd,
            old_path.as_ptr(),
            new_dirfd,
            new_path.as_ptr(),
            flags.bits(),
        )
    })
}

pub fn unlinkat<P: AsRef<OsStr>>(dirfd: RawFd, path: P, flags: AtFlag) -> Result<()> {
    let path = CString::new(path.as_ref().as_bytes())?;
    Errno::from_success_code(unsafe { libc::unlinkat(dirfd, path.as_ptr(), flags.bits()) })
}

pub fn renameat<P: AsRef<OsStr>>(
    old_dirfd: RawFd,
    old_path: P,
    new_dirfd: RawFd,
    new_path: P,
) -> Result<()> {
    let old_path = CString::new(old_path.as_ref().as_bytes())?;
    let new_path = CString::new(new_path.as_ref().as_bytes())?;
    Errno::from_success_code(unsafe {
        libc::renameat(old_dirfd, old_path.as_ptr(), new_dirfd, new_path.as_ptr())
    })
}

pub fn symlinkat<P: AsRef<OsStr>>(old_path: P, new_dirfd: RawFd, new_path: P) -> Result<()> {
    let old_path = CString::new(old_path.as_ref().as_bytes())?;
    let new_path = CString::new(new_path.as_ref().as_bytes())?;
    Errno::from_success_code(unsafe {
        libc::symlinkat(old_path.as_ptr(), new_dirfd, new_path.as_ptr())
    })
}

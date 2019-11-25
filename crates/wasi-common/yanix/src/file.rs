use super::{errno::Errno, libc_bitflags, Result};
use std::convert::TryInto;
use std::ffi::{CString, OsStr, OsString};
use std::os::unix::prelude::*;

libc_bitflags! {
    pub struct AtFlags: libc::c_int {
        AT_REMOVEDIR;
        AT_SYMLINK_FOLLOW;
        AT_SYMLINK_NOFOLLOW;
        #[cfg(any(target_os = "android", target_os = "linux"))]
        AT_NO_AUTOMOUNT;
        #[cfg(any(target_os = "android", target_os = "linux"))]
        AT_EMPTY_PATH;
    }
}

libc_bitflags! {
    pub struct Mode: libc::mode_t {
        S_IRWXU;
        S_IRUSR;
        S_IWUSR;
        S_IXUSR;
        S_IRWXG;
        S_IRGRP;
        S_IWGRP;
        S_IXGRP;
        S_IRWXO;
        S_IROTH;
        S_IWOTH;
        S_IXOTH;
        S_ISUID as libc::mode_t;
        S_ISGID as libc::mode_t;
        S_ISVTX as libc::mode_t;
    }
}

libc_bitflags!(
    /// Configuration options for opened files.
    pub struct OFlag: libc::c_int {
        /// Mask for the access mode of the file.
        O_ACCMODE;
        /// Use alternate I/O semantics.
        #[cfg(target_os = "netbsd")]
        O_ALT_IO;
        /// Open the file in append-only mode.
        O_APPEND;
        /// Generate a signal when input or output becomes possible.
        O_ASYNC;
        /// Closes the file descriptor once an `execve` call is made.
        ///
        /// Also sets the file offset to the beginning of the file.
        O_CLOEXEC;
        /// Create the file if it does not exist.
        O_CREAT;
        /// Try to minimize cache effects of the I/O for this file.
        #[cfg(any(target_os = "android",
                  target_os = "dragonfly",
                  target_os = "freebsd",
                  target_os = "linux",
                  target_os = "netbsd"))]
        O_DIRECT;
        /// If the specified path isn't a directory, fail.
        O_DIRECTORY;
        /// Implicitly follow each `write()` with an `fdatasync()`.
        #[cfg(any(target_os = "android",
                  target_os = "ios",
                  target_os = "linux",
                  target_os = "macos",
                  target_os = "netbsd",
                  target_os = "openbsd",
                  target_os = "emscripten"))]
        O_DSYNC;
        /// Error out if a file was not created.
        O_EXCL;
        /// Open for execute only.
        #[cfg(target_os = "freebsd")]
        O_EXEC;
        /// Open with an exclusive file lock.
        #[cfg(any(target_os = "dragonfly",
                  target_os = "freebsd",
                  target_os = "ios",
                  target_os = "macos",
                  target_os = "netbsd",
                  target_os = "openbsd"))]
        O_EXLOCK;
        /// Same as `O_SYNC`.
        #[cfg(any(target_os = "dragonfly",
                  target_os = "freebsd",
                  target_os = "ios",
                  all(target_os = "linux", not(target_env = "musl")),
                  target_os = "macos",
                  target_os = "netbsd",
                  target_os = "openbsd"))]
        O_FSYNC;
        /// Allow files whose sizes can't be represented in an `off_t` to be opened.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        O_LARGEFILE;
        /// Do not update the file last access time during `read(2)`s.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        O_NOATIME;
        /// Don't attach the device as the process' controlling terminal.
        O_NOCTTY;
        /// Same as `O_NONBLOCK`.
        O_NDELAY;
        /// `open()` will fail if the given path is a symbolic link.
        O_NOFOLLOW;
        /// When possible, open the file in nonblocking mode.
        O_NONBLOCK;
        /// Don't deliver `SIGPIPE`.
        #[cfg(target_os = "netbsd")]
        O_NOSIGPIPE;
        /// Obtain a file descriptor for low-level access.
        ///
        /// The file itself is not opened and other file operations will fail.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        O_PATH;
        /// Only allow reading.
        ///
        /// This should not be combined with `O_WRONLY` or `O_RDWR`.
        O_RDONLY;
        /// Allow both reading and writing.
        ///
        /// This should not be combined with `O_WRONLY` or `O_RDONLY`.
        O_RDWR;
        /// Similar to `O_DSYNC` but applies to `read`s instead.
        #[cfg(any(target_os = "linux",
                  target_os = "netbsd",
                  target_os = "openbsd",
                  target_os = "emscripten"))]
        O_RSYNC;
        /// Skip search permission checks.
        #[cfg(target_os = "netbsd")]
        O_SEARCH;
        /// Open with a shared file lock.
        #[cfg(any(target_os = "dragonfly",
                  target_os = "freebsd",
                  target_os = "ios",
                  target_os = "macos",
                  target_os = "netbsd",
                  target_os = "openbsd"))]
        O_SHLOCK;
        /// Implicitly follow each `write()` with an `fsync()`.
        O_SYNC;
        /// Create an unnamed temporary file.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        O_TMPFILE;
        /// Truncate an existing regular file to 0 length if it allows writing.
        O_TRUNC;
        /// Restore default TTY attributes.
        #[cfg(target_os = "freebsd")]
        O_TTY_INIT;
        /// Only allow writing.
        ///
        /// This should not be combined with `O_RDONLY` or `O_RDWR`.
        O_WRONLY;
    }
);

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
    flags: AtFlags,
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

pub fn unlinkat<P: AsRef<OsStr>>(dirfd: RawFd, path: P, flags: AtFlags) -> Result<()> {
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

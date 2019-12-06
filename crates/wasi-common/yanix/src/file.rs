use crate::{Errno, Result};
use bitflags::bitflags;
use std::{
    convert::TryInto,
    ffi::{CString, OsStr, OsString},
    os::unix::prelude::*,
};

pub use crate::sys::file::*;

bitflags! {
    pub struct FdFlag: libc::c_int {
        const CLOEXEC = libc::FD_CLOEXEC;
    }
}

bitflags! {
    pub struct AtFlag: libc::c_int {
        const REMOVEDIR = libc::AT_REMOVEDIR;
        const SYMLINK_FOLLOW = libc::AT_SYMLINK_FOLLOW;
        const SYMLINK_NOFOLLOW = libc::AT_SYMLINK_NOFOLLOW;
    }
}

bitflags! {
    pub struct Mode: libc::mode_t {
        const IRWXU = libc::S_IRWXU;
        const IRUSR = libc::S_IRUSR;
        const IWUSR = libc::S_IWUSR;
        const IXUSR = libc::S_IXUSR;
        const IRWXG = libc::S_IRWXG;
        const IRGRP = libc::S_IRGRP;
        const IWGRP = libc::S_IWGRP;
        const IXGRP = libc::S_IXGRP;
        const IRWXO = libc::S_IRWXO;
        const IROTH = libc::S_IROTH;
        const IWOTH = libc::S_IWOTH;
        const IXOTH = libc::S_IXOTH;
        const ISUID = libc::S_ISUID as libc::mode_t;
        const ISGID = libc::S_ISGID as libc::mode_t;
        const ISVTX = libc::S_ISVTX as libc::mode_t;
    }
}

bitflags! {
    pub struct OFlag: libc::c_int {
        const ACCMODE = libc::O_ACCMODE;
        const APPEND = libc::O_APPEND;
        const CREAT = libc::O_CREAT;
        const DIRECTORY = libc::O_DIRECTORY;
        #[cfg(any(target_os = "android",
                  target_os = "ios",
                  target_os = "linux",
                  target_os = "macos",
                  target_os = "netbsd",
                  target_os = "openbsd",
                  target_os = "emscripten"))]
        const DSYNC = libc::O_DSYNC;
        const EXCL = libc::O_EXCL;
        #[cfg(any(target_os = "dragonfly",
                  target_os = "freebsd",
                  target_os = "ios",
                  all(target_os = "linux", not(target_env = "musl")),
                  target_os = "macos",
                  target_os = "netbsd",
                  target_os = "openbsd"))]
        const FSYNC = libc::O_FSYNC;
        const NOFOLLOW = libc::O_NOFOLLOW;
        const NONBLOCK = libc::O_NONBLOCK;
        const RDONLY = libc::O_RDONLY;
        const WRONLY = libc::O_WRONLY;
        const RDWR = libc::O_RDWR;
        #[cfg(any(target_os = "linux",
                  target_os = "netbsd",
                  target_os = "openbsd",
                  target_os = "emscripten"))]
        const RSYNC = libc::O_RSYNC;
        const SYNC = libc::O_SYNC;
        const TRUNC = libc::O_TRUNC;
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

pub unsafe fn openat<P: AsRef<OsStr>>(
    dirfd: RawFd,
    path: P,
    oflag: OFlag,
    mode: Mode,
) -> Result<RawFd> {
    let path = CString::new(path.as_ref().as_bytes())?;
    Errno::from_result(libc::openat(
        dirfd,
        path.as_ptr(),
        oflag.bits(),
        libc::c_uint::from(mode.bits()),
    ))
}

pub unsafe fn readlinkat<P: AsRef<OsStr>>(dirfd: RawFd, path: P) -> Result<OsString> {
    let path = CString::new(path.as_ref().as_bytes())?;
    let buffer = &mut [0u8; libc::PATH_MAX as usize + 1];
    Errno::from_result(libc::readlinkat(
        dirfd,
        path.as_ptr(),
        buffer.as_mut_ptr() as *mut _,
        buffer.len(),
    ))
    .and_then(|nread| {
        let link = OsStr::from_bytes(&buffer[0..nread.try_into()?]);
        Ok(link.into())
    })
}

pub unsafe fn mkdirat<P: AsRef<OsStr>>(dirfd: RawFd, path: P, mode: Mode) -> Result<()> {
    let path = CString::new(path.as_ref().as_bytes())?;
    Errno::from_success_code(libc::mkdirat(dirfd, path.as_ptr(), mode.bits()))
}

pub unsafe fn linkat<P: AsRef<OsStr>>(
    old_dirfd: RawFd,
    old_path: P,
    new_dirfd: RawFd,
    new_path: P,
    flags: AtFlag,
) -> Result<()> {
    let old_path = CString::new(old_path.as_ref().as_bytes())?;
    let new_path = CString::new(new_path.as_ref().as_bytes())?;
    Errno::from_success_code(libc::linkat(
        old_dirfd,
        old_path.as_ptr(),
        new_dirfd,
        new_path.as_ptr(),
        flags.bits(),
    ))
}

pub unsafe fn unlinkat<P: AsRef<OsStr>>(dirfd: RawFd, path: P, flags: AtFlag) -> Result<()> {
    let path = CString::new(path.as_ref().as_bytes())?;
    Errno::from_success_code(libc::unlinkat(dirfd, path.as_ptr(), flags.bits()))
}

pub unsafe fn renameat<P: AsRef<OsStr>>(
    old_dirfd: RawFd,
    old_path: P,
    new_dirfd: RawFd,
    new_path: P,
) -> Result<()> {
    let old_path = CString::new(old_path.as_ref().as_bytes())?;
    let new_path = CString::new(new_path.as_ref().as_bytes())?;
    Errno::from_success_code(libc::renameat(
        old_dirfd,
        old_path.as_ptr(),
        new_dirfd,
        new_path.as_ptr(),
    ))
}

pub unsafe fn symlinkat<P: AsRef<OsStr>>(old_path: P, new_dirfd: RawFd, new_path: P) -> Result<()> {
    let old_path = CString::new(old_path.as_ref().as_bytes())?;
    let new_path = CString::new(new_path.as_ref().as_bytes())?;
    Errno::from_success_code(libc::symlinkat(
        old_path.as_ptr(),
        new_dirfd,
        new_path.as_ptr(),
    ))
}

pub unsafe fn fstatat<P: AsRef<OsStr>>(dirfd: RawFd, path: P, flags: AtFlag) -> Result<libc::stat> {
    use std::mem::MaybeUninit;
    let path = CString::new(path.as_ref().as_bytes())?;
    let mut filestat = MaybeUninit::<libc::stat>::uninit();
    Errno::from_result(libc::fstatat(
        dirfd,
        path.as_ptr(),
        filestat.as_mut_ptr(),
        flags.bits(),
    ))?;
    Ok(filestat.assume_init())
}

/// `fionread()` function, equivalent to `ioctl(fd, FIONREAD, *bytes)`.
pub unsafe fn fionread(fd: RawFd) -> Result<usize> {
    let mut nread: libc::c_int = 0;
    Errno::from_result(libc::ioctl(fd, libc::FIONREAD, &mut nread as *mut _))?;
    Ok(nread.try_into()?)
}

use crate::{cstr, from_result, from_success_code};
use bitflags::bitflags;
use cfg_if::cfg_if;
#[cfg(unix)]
use std::os::unix::prelude::*;
#[cfg(target_os = "wasi")]
use std::os::wasi::prelude::*;
use std::{
    convert::TryInto,
    ffi::{OsStr, OsString},
    io::Result,
    path::Path,
};

pub use crate::sys::file::*;

bitflags! {
    pub struct FdFlags: libc::c_int {
        const CLOEXEC = libc::FD_CLOEXEC;
    }
}

bitflags! {
    pub struct AtFlags: libc::c_int {
        const REMOVEDIR = libc::AT_REMOVEDIR;
        const SYMLINK_FOLLOW = libc::AT_SYMLINK_FOLLOW;
        const SYMLINK_NOFOLLOW = libc::AT_SYMLINK_NOFOLLOW;
        #[cfg(any(target_os = "linux",
                  target_os = "fuchsia"))]
        const EMPTY_PATH = libc::AT_EMPTY_PATH;
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
    pub struct OFlags: libc::c_int {
        const ACCMODE = libc::O_ACCMODE;
        const APPEND = libc::O_APPEND;
        const CREAT = libc::O_CREAT;
        const DIRECTORY = libc::O_DIRECTORY;
        const DSYNC = {
            // Have to use cfg_if: https://github.com/bitflags/bitflags/issues/137
            cfg_if! {
                if #[cfg(any(target_os = "android",
                             target_os = "ios",
                             target_os = "linux",
                             target_os = "macos",
                             target_os = "netbsd",
                             target_os = "openbsd",
                             target_os = "wasi",
                             target_os = "emscripten"))] {
                    libc::O_DSYNC
                } else if #[cfg(target_os = "freebsd")] {
                    // https://github.com/bytecodealliance/wasmtime/pull/756
                    libc::O_SYNC
                }
            }
        };
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
                  target_os = "wasi",
                  target_os = "emscripten"))]
        const RSYNC = libc::O_RSYNC;
        const SYNC = libc::O_SYNC;
        const TRUNC = libc::O_TRUNC;
        #[cfg(any(target_os = "linux",
                  target_os = "fuchsia",
                  target_os = "redox"))]
        const PATH = libc::O_PATH;
        #[cfg(any(target_os = "linux",
                  target_os = "fuchsia",
                  target_os = "hermit",
                  target_os = "solaris",
                  target_os = "haiku",
                  target_os = "netbsd",
                  target_os = "freebsd",
                  target_os = "openbsd",
                  target_os = "dragonfly",
                  target_os = "vxworks",
                  target_os = "macos",
                  target_os = "ios",
                  target_os = "redox"))]
        const CLOEXEC = libc::O_CLOEXEC;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileType {
    CharacterDevice,
    Directory,
    BlockDevice,
    RegularFile,
    Symlink,
    Fifo,
    Socket,
    Unknown,
}

impl FileType {
    pub fn from_stat_st_mode(st_mode: libc::mode_t) -> Self {
        match st_mode & libc::S_IFMT {
            libc::S_IFIFO => Self::Fifo,
            libc::S_IFCHR => Self::CharacterDevice,
            libc::S_IFDIR => Self::Directory,
            libc::S_IFBLK => Self::BlockDevice,
            libc::S_IFREG => Self::RegularFile,
            libc::S_IFLNK => Self::Symlink,
            libc::S_IFSOCK => Self::Socket,
            _ => Self::Unknown, // Should we actually panic here since this one *should* never happen?
        }
    }

    pub fn from_dirent_d_type(d_type: u8) -> Self {
        match d_type {
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
}

pub unsafe fn openat<P: AsRef<Path>>(
    dirfd: RawFd,
    path: P,
    oflag: OFlags,
    mode: Mode,
) -> Result<RawFd> {
    let path = cstr(path)?;
    from_result(libc::openat(
        dirfd,
        path.as_ptr(),
        oflag.bits(),
        libc::c_uint::from(mode.bits()),
    ))
}

pub unsafe fn readlinkat<P: AsRef<Path>>(dirfd: RawFd, path: P) -> Result<OsString> {
    let path = cstr(path)?;
    let buffer = &mut [0u8; libc::PATH_MAX as usize + 1];
    let nread = from_result(libc::readlinkat(
        dirfd,
        path.as_ptr(),
        buffer.as_mut_ptr() as *mut _,
        buffer.len(),
    ))?;
    // We can just unwrap() this, because readlinkat returns an ssize_t which is either -1
    // (handled above) or non-negative and will fit in a size_t/usize, which is what we're
    // converting it to here.
    let nread = nread.try_into().unwrap();
    let link = OsStr::from_bytes(&buffer[0..nread]);
    Ok(link.into())
}

pub unsafe fn mkdirat<P: AsRef<Path>>(dirfd: RawFd, path: P, mode: Mode) -> Result<()> {
    let path = cstr(path)?;
    from_success_code(libc::mkdirat(dirfd, path.as_ptr(), mode.bits()))
}

pub unsafe fn linkat<P: AsRef<Path>, Q: AsRef<Path>>(
    old_dirfd: RawFd,
    old_path: P,
    new_dirfd: RawFd,
    new_path: Q,
    flags: AtFlags,
) -> Result<()> {
    let old_path = cstr(old_path)?;
    let new_path = cstr(new_path)?;
    from_success_code(libc::linkat(
        old_dirfd,
        old_path.as_ptr(),
        new_dirfd,
        new_path.as_ptr(),
        flags.bits(),
    ))
}

pub unsafe fn unlinkat<P: AsRef<Path>>(dirfd: RawFd, path: P, flags: AtFlags) -> Result<()> {
    let path = cstr(path)?;
    from_success_code(libc::unlinkat(dirfd, path.as_ptr(), flags.bits()))
}

pub unsafe fn renameat<P: AsRef<Path>, Q: AsRef<Path>>(
    old_dirfd: RawFd,
    old_path: P,
    new_dirfd: RawFd,
    new_path: Q,
) -> Result<()> {
    let old_path = cstr(old_path)?;
    let new_path = cstr(new_path)?;
    from_success_code(libc::renameat(
        old_dirfd,
        old_path.as_ptr(),
        new_dirfd,
        new_path.as_ptr(),
    ))
}

pub unsafe fn symlinkat<P: AsRef<Path>, Q: AsRef<Path>>(
    old_path: P,
    new_dirfd: RawFd,
    new_path: Q,
) -> Result<()> {
    let old_path = cstr(old_path)?;
    let new_path = cstr(new_path)?;
    from_success_code(libc::symlinkat(
        old_path.as_ptr(),
        new_dirfd,
        new_path.as_ptr(),
    ))
}

pub unsafe fn fstatat<P: AsRef<Path>>(dirfd: RawFd, path: P, flags: AtFlags) -> Result<libc::stat> {
    use std::mem::MaybeUninit;
    let path = cstr(path)?;
    let mut filestat = MaybeUninit::<libc::stat>::uninit();
    from_result(libc::fstatat(
        dirfd,
        path.as_ptr(),
        filestat.as_mut_ptr(),
        flags.bits(),
    ))?;
    Ok(filestat.assume_init())
}

pub unsafe fn fstat(fd: RawFd) -> Result<libc::stat> {
    use std::mem::MaybeUninit;
    let mut filestat = MaybeUninit::<libc::stat>::uninit();
    from_result(libc::fstat(fd, filestat.as_mut_ptr()))?;
    Ok(filestat.assume_init())
}

/// `fionread()` function, equivalent to `ioctl(fd, FIONREAD, *bytes)`.
pub unsafe fn fionread(fd: RawFd) -> Result<u32> {
    let mut nread: libc::c_int = 0;
    from_result(libc::ioctl(fd, libc::FIONREAD, &mut nread as *mut _))?;
    // FIONREAD returns a non-negative int if it doesn't fail, or it'll fit in a u32 if it does.
    //
    // For the future, if we want to be super cautious and avoid assuming int is 32-bit, we could
    // widen fionread's return type here, since the one place that calls it wants a u64 anyway.
    Ok(nread.try_into().unwrap())
}

/// This function is unsafe because it operates on a raw file descriptor.
/// It's provided, because std::io::Seek requires a mutable borrow.
pub unsafe fn tell(fd: RawFd) -> Result<u64> {
    let offset = from_result(libc::lseek(fd, 0, libc::SEEK_CUR))?;
    // lseek returns an off_t, which we can assume is a non-negative i64 if it doesn't fail.
    // So we can unwrap() this conversion.
    Ok(offset.try_into().unwrap())
}

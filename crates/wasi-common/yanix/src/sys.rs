use super::{errno::Errno, file::AtFlag, Error, Result};
use bitflags::bitflags;
use std::ffi::{CString, OsStr};
use std::os::unix::prelude::*;

bitflags! {
    /// Additional configuration flags for `fcntl`'s `F_SETFD`.
    pub struct FdFlag: libc::c_int {
        /// The file descriptor will automatically be closed during a successful `execve(2)`.
        const CLOEXEC = libc::FD_CLOEXEC;
    }
}

/// The `fcntl` commands exposed as safe Rust functions.
pub mod fcntl {
    use super::super::{errno::Errno, file::OFlag, Result};
    use super::FdFlag;
    use std::os::unix::prelude::*;

    pub fn dup_fd(fd: RawFd, close_on_exec: bool) -> Result<RawFd> {
        // Both fcntl commands expect a RawFd arg which will specify
        // the minimum duplicated RawFd number. In our case, I don't
        // think we have to worry about this that much, so passing in
        // the RawFd descriptor we want duplicated
        Errno::from_result(unsafe {
            if close_on_exec {
                libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, fd)
            } else {
                libc::fcntl(fd, libc::F_DUPFD, fd)
            }
        })
    }

    pub fn get_fd(fd: RawFd) -> Result<FdFlag> {
        Errno::from_result(unsafe { libc::fcntl(fd, libc::F_GETFD) })
            .map(FdFlag::from_bits_truncate)
    }

    pub fn set_fd(fd: RawFd, flags: FdFlag) -> Result<()> {
        Errno::from_success_code(unsafe { libc::fcntl(fd, libc::F_SETFD, flags.bits()) })
    }

    pub fn get_fl(fd: RawFd) -> Result<OFlag> {
        Errno::from_result(unsafe { libc::fcntl(fd, libc::F_GETFL) }).map(OFlag::from_bits_truncate)
    }

    pub fn set_fl(fd: RawFd, flags: OFlag) -> Result<()> {
        Errno::from_success_code(unsafe { libc::fcntl(fd, libc::F_SETFL, flags.bits()) })
    }

    /// From macOS man pages:
    /// F_RDADVISE   Issue an advisory read async with no copy to user.
    ///
    /// The F_RDADVISE command operates on the following structure which holds information passed from
    /// the user to the system:
    ///
    /// struct radvisory {
    ///      off_t   ra_offset;  /* offset into the file */
    ///      int     ra_count;   /* size of the read     */
    /// };
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub fn rd_advise(fd: RawFd, offset: libc::off_t, count: i32) -> Result<()> {
        let advisory = libc::radvisory {
            ra_offset: offset,
            ra_count: count,
        };
        Errno::from_success_code(unsafe { libc::fcntl(fd, libc::F_RDADVISE, &advisory) })
    }
}

pub fn isatty(fd: RawFd) -> Result<bool> {
    let res = unsafe { libc::isatty(fd) };
    if res == 1 {
        // isatty() returns 1 if fd is an open file descriptor referring to a terminal...
        Ok(true)
    } else {
        // ... otherwise 0 is returned, and errno is set to indicate the error.
        match Errno::last() {
            Errno::ENOTTY => Ok(false),
            // While POSIX specifies ENOTTY if the passed
            // fd is *not* a tty, on Linux, some implementations
            // may return EINVAL instead.
            //
            // https://linux.die.net/man/3/isatty
            #[cfg(any(target_os = "linux", target_os = "android"))]
            Errno::EINVAL => Ok(false),
            x => Err(Error::Errno(x)),
        }
    }
}

pub fn fstatat<P: AsRef<OsStr>>(dirfd: RawFd, path: P, flags: AtFlag) -> Result<libc::stat> {
    use std::mem::MaybeUninit;
    let path = CString::new(path.as_ref().as_bytes())?;
    let mut filestat = MaybeUninit::<libc::stat>::uninit();
    Errno::from_result(unsafe {
        libc::fstatat(dirfd, path.as_ptr(), filestat.as_mut_ptr(), flags.bits())
    })?;
    Ok(unsafe { filestat.assume_init() })
}

/// `fionread()` function, equivalent to `ioctl(fd, FIONREAD, *bytes)`.
pub fn fionread(fd: RawFd) -> Result<usize> {
    use std::convert::TryInto;
    let mut nread: libc::c_int = 0;
    Errno::from_result(unsafe { libc::ioctl(fd, libc::FIONREAD, &mut nread as *mut _) })?;
    Ok(nread.try_into()?)
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "emscripten",
    target_os = "fuchsia",
    target_env = "uclibc",
    target_env = "freebsd"
))]
pub use posix_fadvise::*;

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "emscripten",
    target_os = "fuchsia",
    target_env = "uclibc",
    target_env = "freebsd"
))]
mod posix_fadvise {
    use super::super::{errno::Errno, Result};
    use std::os::unix::prelude::*;

    #[derive(Debug, Copy, Clone)]
    #[repr(i32)]
    pub enum PosixFadviseAdvice {
        Normal = libc::POSIX_FADV_NORMAL,
        Sequential = libc::POSIX_FADV_SEQUENTIAL,
        Random = libc::POSIX_FADV_RANDOM,
        NoReuse = libc::POSIX_FADV_NOREUSE,
        WillNeed = libc::POSIX_FADV_WILLNEED,
        DontNeed = libc::POSIX_FADV_DONTNEED,
    }

    pub fn posix_fadvise(
        fd: RawFd,
        offset: libc::off_t,
        len: libc::off_t,
        advice: PosixFadviseAdvice,
    ) -> Result<()> {
        Errno::from_success_code(unsafe {
            libc::posix_fadvise(fd, offset, len, advice as libc::c_int)
        })
    }
}

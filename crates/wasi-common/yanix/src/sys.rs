use super::{
    errno::Errno,
    file::{AtFlags, OFlag},
    libc_bitflags, Error, Result,
};
use std::ffi::{CString, OsStr};
use std::os::unix::prelude::*;

libc_bitflags! {
    /// Additional configuration flags for `fcntl`'s `F_SETFD`.
    pub struct FdFlag: libc::c_int {
        /// The file descriptor will automatically be closed during a successful `execve(2)`.
        FD_CLOEXEC;
    }
}

libc_bitflags! {
    pub struct SFlag: libc::mode_t {
        S_IFIFO;
        S_IFCHR;
        S_IFDIR;
        S_IFBLK;
        S_IFREG;
        S_IFLNK;
        S_IFSOCK;
        S_IFMT;
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
libc_bitflags!(
    /// Additional flags for file sealing, which allows for limiting operations on a file.
    pub struct SealFlag: libc::c_int {
        /// Prevents further calls to `fcntl()` with `F_ADD_SEALS`.
        F_SEAL_SEAL;
        /// The file cannot be reduced in size.
        F_SEAL_SHRINK;
        /// The size of the file cannot be increased.
        F_SEAL_GROW;
        /// The file contents cannot be modified.
        F_SEAL_WRITE;
    }
);

#[derive(Debug, Eq, Hash, PartialEq)]
#[allow(non_camel_case_types)]
pub enum FcntlCmd<'a> {
    F_DUPFD(RawFd),
    F_DUPFD_CLOEXEC(RawFd),
    F_GETFD,
    F_SETFD(FdFlag), // FD_FLAGS
    F_GETFL,
    F_SETFL(OFlag), // O_NONBLOCK
    F_SETLK(&'a libc::flock),
    F_SETLKW(&'a libc::flock),
    F_GETLK(&'a mut libc::flock),
    #[cfg(any(target_os = "linux", target_os = "android"))]
    F_OFD_SETLK(&'a libc::flock),
    #[cfg(any(target_os = "linux", target_os = "android"))]
    F_OFD_SETLKW(&'a libc::flock),
    #[cfg(any(target_os = "linux", target_os = "android"))]
    F_OFD_GETLK(&'a mut libc::flock),
    #[cfg(any(target_os = "android", target_os = "linux"))]
    F_ADD_SEALS(SealFlag),
    #[cfg(any(target_os = "android", target_os = "linux"))]
    F_GET_SEALS,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    F_FULLFSYNC,
    #[cfg(any(target_os = "linux", target_os = "android"))]
    F_GETPIPE_SZ,
    #[cfg(any(target_os = "linux", target_os = "android"))]
    F_SETPIPE_SZ(libc::c_int),
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    // From macOS man pages:
    // F_RDADVISE   Issue an advisory read async with no copy to user.
    //
    // The F_RDADVISE command operates on the following structure which holds information passed from
    // the user to the system:
    //
    // struct radvisory {
    //      off_t   ra_offset;  /* offset into the file */
    //      int     ra_count;   /* size of the read     */
    // };
    F_RDADVISE(libc::off_t, libc::c_int),
    // TODO: Rest of flags
}

pub use self::FcntlCmd::*;

pub fn fcntl(fd: RawFd, cmd: FcntlCmd) -> Result<libc::c_int> {
    let res = unsafe {
        match cmd {
            F_DUPFD(rawfd) => libc::fcntl(fd, libc::F_DUPFD, rawfd),
            F_DUPFD_CLOEXEC(rawfd) => libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, rawfd),
            F_GETFD => libc::fcntl(fd, libc::F_GETFD),
            F_SETFD(flag) => libc::fcntl(fd, libc::F_SETFD, flag.bits()),
            F_GETFL => libc::fcntl(fd, libc::F_GETFL),
            F_SETFL(flag) => libc::fcntl(fd, libc::F_SETFL, flag.bits()),
            F_SETLK(flock) => libc::fcntl(fd, libc::F_SETLK, flock),
            F_SETLKW(flock) => libc::fcntl(fd, libc::F_SETLKW, flock),
            F_GETLK(flock) => libc::fcntl(fd, libc::F_GETLK, flock),
            #[cfg(any(target_os = "android", target_os = "linux"))]
            F_ADD_SEALS(flag) => libc::fcntl(fd, libc::F_ADD_SEALS, flag.bits()),
            #[cfg(any(target_os = "android", target_os = "linux"))]
            F_GET_SEALS => libc::fcntl(fd, libc::F_GET_SEALS),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            F_FULLFSYNC => libc::fcntl(fd, libc::F_FULLFSYNC),
            #[cfg(any(target_os = "linux", target_os = "android"))]
            F_GETPIPE_SZ => libc::fcntl(fd, libc::F_GETPIPE_SZ),
            #[cfg(any(target_os = "linux", target_os = "android"))]
            F_SETPIPE_SZ(size) => libc::fcntl(fd, libc::F_SETPIPE_SZ, size),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            F_RDADVISE(ra_offset, ra_count) => {
                let advisory = libc::radvisory {
                    ra_offset,
                    ra_count,
                };
                libc::fcntl(fd, libc::F_RDADVISE, &advisory)
            }
            #[cfg(any(target_os = "linux", target_os = "android"))]
            _ => unimplemented!(),
        }
    };

    Errno::from_result(res)
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

pub fn fstatat<P: AsRef<OsStr>>(dirfd: RawFd, path: P, flags: AtFlags) -> Result<libc::stat> {
    use std::mem::MaybeUninit;
    let path = CString::new(path.as_ref().as_bytes())?;
    let mut filestat = MaybeUninit::<libc::stat>::uninit();
    Errno::from_result(unsafe {
        libc::fstatat(dirfd, path.as_ptr(), filestat.as_mut_ptr(), flags.bits())
    })?;
    Ok(unsafe { filestat.assume_init() })
}

// define the `fionread()` function, equivalent to `ioctl(fd, FIONREAD, *bytes)`
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

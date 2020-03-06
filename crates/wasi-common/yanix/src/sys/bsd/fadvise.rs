use crate::{Error, Result};
use std::{convert::TryInto, os::unix::prelude::*};

#[cfg(not(any(target_os = "freebsd", target_os = "netbsd")))]
#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum PosixFadviseAdvice {
    Normal,
    Sequential,
    Random,
    NoReuse,
    WillNeed,
    DontNeed,
}

#[cfg(any(target_os = "freebsd", target_os = "netbsd"))]
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

// There's no posix_fadvise on macOS but we can use fcntl with F_RDADVISE
// command instead to achieve the same
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub unsafe fn posix_fadvise(
    fd: RawFd,
    offset: libc::off_t,
    len: libc::off_t,
    _advice: PosixFadviseAdvice,
) -> Result<()> {
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
    let advisory = libc::radvisory {
        ra_offset: offset,
        ra_count: len.try_into()?,
    };
    Error::from_success_code(libc::fcntl(fd, libc::F_RDADVISE, &advisory))
}

#[cfg(any(target_os = "freebsd", target_os = "netbsd"))]
pub unsafe fn posix_fadvise(
    fd: RawFd,
    offset: libc::off_t,
    len: libc::off_t,
    advice: PosixFadviseAdvice,
) -> Result<()> {
    Error::from_success_code(libc::posix_fadvise(fd, offset, len, advice as libc::c_int))
}

// On BSDs without support we leave it as no-op
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd"
)))]
pub unsafe fn posix_fadvise(
    _fd: RawFd,
    _offset: libc::off_t,
    _len: libc::off_t,
    _advice: PosixFadviseAdvice,
) -> Result<()> {
    Ok(())
}

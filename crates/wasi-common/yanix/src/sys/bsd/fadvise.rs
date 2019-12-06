use crate::{Errno, Result};
use std::{convert::TryInto, os::unix::prelude::*};

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
    Errno::from_success_code(libc::fcntl(fd, libc::F_RDADVISE, &advisory))
}

// TODO
// On non-macOS BSD's we leave it as no-op for now
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub unsafe fn posix_fadvise(
    _fd: RawFd,
    _offset: libc::off_t,
    _len: libc::off_t,
    _advice: PosixFadviseAdvice,
) -> Result<()> {
    Ok(())
}

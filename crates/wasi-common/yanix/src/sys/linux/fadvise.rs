use crate::{Error, Result};
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

pub unsafe fn posix_fadvise(
    fd: RawFd,
    offset: libc::off_t,
    len: libc::off_t,
    advice: PosixFadviseAdvice,
) -> Result<()> {
    Error::from_success_code(libc::posix_fadvise(fd, offset, len, advice as libc::c_int))
}

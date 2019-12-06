use crate::{Errno, Result};
use std::os::unix::prelude::*;

pub unsafe fn isatty(fd: RawFd) -> Result<bool> {
    let res = libc::isatty(fd);
    if res == 1 {
        // isatty() returns 1 if fd is an open file descriptor referring to a terminal...
        Ok(true)
    } else {
        // ... otherwise 0 is returned, and errno is set to indicate the error.
        let errno = Errno::last();
        // While POSIX specifies ENOTTY if the passed
        // fd is *not* a tty, on Linux, some implementations
        // may return EINVAL instead.
        //
        // https://linux.die.net/man/3/isatty
        if errno == Errno::ENOTTY || errno == Errno::EINVAL {
            Ok(false)
        } else {
            Err(errno.into())
        }
    }
}

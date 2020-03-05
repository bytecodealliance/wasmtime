use crate::Result;
use std::{io, os::unix::prelude::*};

pub unsafe fn isatty(fd: RawFd) -> Result<bool> {
    let res = libc::isatty(fd);
    if res == 1 {
        // isatty() returns 1 if fd is an open file descriptor referring to a terminal...
        Ok(true)
    } else {
        // ... otherwise 0 is returned, and errno is set to indicate the error.
        let errno = io::Error::last_os_error();
        if let Some(raw_errno) = errno.raw_os_error() {
            if raw_errno == libc::ENOTTY {
                Ok(false)
            } else {
                Err(errno.into())
            }
        } else {
            Err(errno.into())
        }
    }
}

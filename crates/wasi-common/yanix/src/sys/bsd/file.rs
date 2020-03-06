use std::{
    io::{Error, Result},
    os::unix::prelude::*,
};

pub unsafe fn isatty(fd: RawFd) -> Result<bool> {
    let res = libc::isatty(fd);
    if res == 1 {
        // isatty() returns 1 if fd is an open file descriptor referring to a terminal...
        Ok(true)
    } else {
        // ... otherwise 0 is returned, and errno is set to indicate the error.
        let errno = Error::last_os_error();
        if errno.raw_os_error().unwrap() == libc::ENOTTY {
            Ok(false)
        } else {
            Err(errno)
        }
    }
}

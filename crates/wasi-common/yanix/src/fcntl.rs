use crate::{
    file::{FdFlags, OFlags},
    from_result, from_success_code,
};
use std::io::Result;
use std::os::unix::prelude::*;

pub unsafe fn dup_fd(fd: RawFd, close_on_exec: bool) -> Result<RawFd> {
    // Both fcntl commands expect a RawFd arg which will specify
    // the minimum duplicated RawFd number. In our case, I don't
    // think we have to worry about this that much, so passing in
    // the RawFd descriptor we want duplicated
    from_result(if close_on_exec {
        libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, fd)
    } else {
        libc::fcntl(fd, libc::F_DUPFD, fd)
    })
}

pub unsafe fn get_fd_flags(fd: RawFd) -> Result<FdFlags> {
    from_result(libc::fcntl(fd, libc::F_GETFD)).map(FdFlags::from_bits_truncate)
}

pub unsafe fn set_fd_flags(fd: RawFd, flags: FdFlags) -> Result<()> {
    from_success_code(libc::fcntl(fd, libc::F_SETFD, flags.bits()))
}

pub unsafe fn get_status_flags(fd: RawFd) -> Result<OFlags> {
    from_result(libc::fcntl(fd, libc::F_GETFL)).map(OFlags::from_bits_truncate)
}

pub unsafe fn set_status_flags(fd: RawFd, flags: OFlags) -> Result<()> {
    from_success_code(libc::fcntl(fd, libc::F_SETFL, flags.bits()))
}

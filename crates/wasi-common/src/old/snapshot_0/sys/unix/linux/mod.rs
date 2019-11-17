pub(crate) mod filetime;
pub(crate) mod hostcalls_impl;
pub(crate) mod osfile;

pub(crate) mod fdentry_impl {
    use crate::old::snapshot_0::{sys::host_impl, Result};
    use std::os::unix::prelude::AsRawFd;

    pub(crate) unsafe fn isatty(fd: &impl AsRawFd) -> Result<bool> {
        use nix::errno::Errno;

        let res = libc::isatty(fd.as_raw_fd());
        if res == 1 {
            // isatty() returns 1 if fd is an open file descriptor referring to a terminal...
            Ok(true)
        } else {
            // ... otherwise 0 is returned, and errno is set to indicate the error.
            match Errno::last() {
                // While POSIX specifies ENOTTY if the passed
                // fd is *not* a tty, on Linux, some implementations
                // may return EINVAL instead.
                //
                // https://linux.die.net/man/3/isatty
                Errno::ENOTTY | Errno::EINVAL => Ok(false),
                x => Err(host_impl::errno_from_nix(x)),
            }
        }
    }
}

pub(crate) mod host_impl {
    pub(crate) const O_RSYNC: nix::fcntl::OFlag = nix::fcntl::OFlag::O_RSYNC;
}

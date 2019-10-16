pub(crate) mod hostcalls_impl;
pub(crate) mod osfile;

pub(crate) mod fdentry_impl {
    use crate::{sys::host_impl, Result};
    use std::os::unix::prelude::AsRawFd;

    pub(crate) unsafe fn isatty(fd: &impl AsRawFd) -> Result<bool> {
        let res = libc::isatty(fd.as_raw_fd());
        if res == 0 {
            Ok(true)
        } else {
            match nix::errno::Errno::last() {
                nix::errno::Errno::ENOTTY => Ok(false),
                x => Err(host_impl::errno_from_nix(x)),
            }
        }
    }
}

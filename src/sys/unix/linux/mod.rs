pub(crate) mod hostcalls_impl;
pub(crate) mod osfile;

pub(crate) mod fdentry_impl {
    use crate::{sys::host_impl, Result};
    use std::os::unix::prelude::AsRawFd;

    pub(crate) unsafe fn isatty(fd: &impl AsRawFd) -> Result<bool> {
        use nix::errno::Errno;

        let res = libc::isatty(fd.as_raw_fd());
        if res == 0 {
            Ok(true)
        } else {
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
    use super::super::host_impl::dirent_filetype_from_host;
    use crate::{wasi, Error, Result};
    use std::convert::TryFrom;

    pub(crate) const O_RSYNC: nix::fcntl::OFlag = nix::fcntl::OFlag::O_RSYNC;

    pub(crate) fn dirent_from_host(
        host_entry: &nix::libc::dirent,
    ) -> Result<wasi::__wasi_dirent_t> {
        let mut entry = unsafe { std::mem::zeroed::<wasi::__wasi_dirent_t>() };
        let d_namlen = unsafe { std::ffi::CStr::from_ptr(host_entry.d_name.as_ptr()) }
            .to_bytes()
            .len();
        if d_namlen > u32::max_value() as usize {
            return Err(Error::EIO);
        }
        let d_type = dirent_filetype_from_host(host_entry)?;
        entry.d_ino = host_entry.d_ino;
        entry.d_next = u64::try_from(host_entry.d_off).map_err(|_| Error::EOVERFLOW)?;
        entry.d_namlen = u32::try_from(d_namlen).map_err(|_| Error::EOVERFLOW)?;
        entry.d_type = d_type;
        Ok(entry)
    }
}

pub(crate) mod fs_helpers {
    pub(crate) fn utime_now() -> libc::c_long {
        libc::UTIME_NOW
    }

    pub(crate) fn utime_omit() -> libc::c_long {
        libc::UTIME_OMIT
    }
}

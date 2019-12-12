//! This internal module consists of helper types and functions for dealing
//! with setting the file times specific to Linux.
use crate::old::snapshot_0::{sys::unix::filetime::FileTime, Result};
use std::fs::File;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

pub(crate) const UTIME_NOW: i64 = 1_073_741_823;
pub(crate) const UTIME_OMIT: i64 = 1_073_741_822;

/// Wrapper for `utimensat` syscall, however, with an added twist such that `utimensat` symbol
/// is firstly resolved (i.e., we check whether it exists on the host), and only used if that is
/// the case. Otherwise, the syscall resorts to a less accurate `utimesat` emulated syscall.
/// The original implementation can be found here: [filetime::unix::linux::set_times]
///
/// [filetime::unix::linux::set_times]: https://github.com/alexcrichton/filetime/blob/master/src/unix/linux.rs#L64
pub(crate) fn utimensat(
    dirfd: &File,
    path: &str,
    atime: FileTime,
    mtime: FileTime,
    symlink_nofollow: bool,
) -> Result<()> {
    use crate::old::snapshot_0::sys::unix::filetime::to_timespec;
    use std::ffi::CString;
    use std::os::unix::prelude::*;

    let flags = if symlink_nofollow {
        libc::AT_SYMLINK_NOFOLLOW
    } else {
        0
    };

    // Attempt to use the `utimensat` syscall, but if it's not supported by the
    // current kernel then fall back to an older syscall.
    static INVALID: AtomicBool = AtomicBool::new(false);
    if !INVALID.load(Relaxed) {
        let p = CString::new(path.as_bytes())?;
        let times = [to_timespec(&atime), to_timespec(&mtime)];
        let rc = unsafe {
            libc::syscall(
                libc::SYS_utimensat,
                dirfd.as_raw_fd(),
                p.as_ptr(),
                times.as_ptr(),
                flags,
            )
        };
        if rc == 0 {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ENOSYS) {
            INVALID.store(true, Relaxed);
        } else {
            return Err(err.into());
        }
    }

    super::utimesat::utimesat(dirfd, path, atime, mtime, symlink_nofollow)
}

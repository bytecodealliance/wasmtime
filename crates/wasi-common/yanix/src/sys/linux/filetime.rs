//! This module consists of helper types and functions for dealing
//! with setting the file times specific to Linux.
use crate::filetime::FileTime;
use crate::{cstr, from_success_code};
use std::fs::File;
use std::io::Result;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

/// Wrapper for `utimensat` syscall, however, with an added twist such that `utimensat` symbol
/// is firstly resolved (i.e., we check whether it exists on the host), and only used if that is
/// the case. Otherwise, the syscall resorts to a less accurate `utimesat` emulated syscall.
/// The original implementation can be found here: [filetime::unix::linux::set_times]
///
/// [filetime::unix::linux::set_times]: https://github.com/alexcrichton/filetime/blob/master/src/unix/linux.rs#L64
pub fn utimensat(
    dirfd: &File,
    path: &str,
    atime: FileTime,
    mtime: FileTime,
    symlink_nofollow: bool,
) -> Result<()> {
    use crate::filetime::to_timespec;
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
        let path = cstr(path)?;
        let times = [to_timespec(&atime)?, to_timespec(&mtime)?];
        let res = from_success_code(unsafe {
            libc::syscall(
                libc::SYS_utimensat,
                dirfd.as_raw_fd(),
                path.as_ptr(),
                times.as_ptr(),
                flags,
            )
        });
        let err = match res {
            Ok(()) => return Ok(()),
            Err(e) => e,
        };
        if err.raw_os_error().unwrap() == libc::ENOSYS {
            INVALID.store(true, Relaxed);
        }
        return Err(err);
    }

    #[cfg(not(target_os = "android"))]
    return super::utimesat::utimesat(dirfd, path, atime, mtime, symlink_nofollow);
    #[cfg(target_os = "android")]
    unreachable!();
}

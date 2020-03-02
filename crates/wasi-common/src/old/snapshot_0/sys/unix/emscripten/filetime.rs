//! This internal module consists of helper types and functions for dealing
//! with setting the file times specific to Emscripten.
use crate::old::snapshot_0::{sys::unix::filetime::FileTime, Result};
use std::fs::File;
use std::io;

pub(crate) const UTIME_NOW: i32 = 1_073_741_823;
pub(crate) const UTIME_OMIT: i32 = 1_073_741_822;

/// Wrapper for `utimensat` syscall. In Emscripten, there is no point in dynamically resolving
/// if `utimensat` is available as it always was and will be.
pub(crate) fn utimensat<P: AsRef<CStr>>(
    dirfd: &File,
    path: P,
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
    let times = [to_timespec(&atime)?, to_timespec(&mtime)?];
    let rc = unsafe {
        libc::utimensat(
            dirfd.as_raw_fd(),
            path.as_ref().as_ptr(),
            times.as_ptr(),
            flags,
        )
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error().into())
    }
}

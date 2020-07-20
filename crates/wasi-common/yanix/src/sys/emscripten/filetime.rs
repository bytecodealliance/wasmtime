//! This module consists of helper types and functions for dealing
//! with setting the file times specific to Emscripten.
use crate::filetime::FileTime;
use crate::{cstr, from_success_code};
use std::fs::File;
use std::io::Result;

/// Wrapper for `utimensat` syscall. In Emscripten, there is no point in dynamically resolving
/// if `utimensat` is available as it always was and will be.
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
    let path = cstr(path)?;
    let times = [to_timespec(&atime)?, to_timespec(&mtime)?];
    from_success_code(unsafe {
        libc::utimensat(dirfd.as_raw_fd(), path.as_ptr(), times.as_ptr(), flags)
    })
}

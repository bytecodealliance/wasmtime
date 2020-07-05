use crate::filetime::FileTime;
use crate::from_success_code;
use std::fs::File;
use std::io;

pub fn utimensat(
    dirfd: &File,
    path: &str,
    atime: FileTime,
    mtime: FileTime,
    symlink_nofollow: bool,
) -> io::Result<()> {
    use crate::filetime::to_timespec;
    use std::ffi::CString;
    use std::os::wasi::prelude::*;

    let p = CString::new(path.as_bytes())?;
    let times = [to_timespec(&atime)?, to_timespec(&mtime)?];
    let flags = if symlink_nofollow {
        libc::AT_SYMLINK_NOFOLLOW
    } else {
        0
    };

    from_success_code(unsafe {
        libc::utimensat(
            dirfd.as_raw_fd() as libc::c_int,
            p.as_ptr(),
            times.as_ptr(),
            flags,
        )
    })
}

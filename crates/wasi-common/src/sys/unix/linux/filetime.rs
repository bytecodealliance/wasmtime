use super::super::filetime::FileTime;
use std::fs::File;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

pub(crate) const UTIME_NOW: i64 = libc::UTIME_NOW;
pub(crate) const UTIME_OMIT: i64 = libc::UTIME_OMIT;

pub(crate) fn utimensat(
    dirfd: &File,
    path: &str,
    atime: FileTime,
    mtime: FileTime,
    symlink: bool,
) -> io::Result<()> {
    use super::super::filetime::{to_timespec, utimesat};
    use std::ffi::CString;
    use std::os::unix::prelude::*;

    let flags = if symlink {
        libc::AT_SYMLINK_NOFOLLOW
    } else {
        0
    };

    // Attempt to use the `utimensat` syscall, but if it's not supported by the
    // current kernel then fall back to an older syscall.
    static INVALID: AtomicBool = AtomicBool::new(false);
    if !INVALID.load(SeqCst) {
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
            INVALID.store(true, SeqCst);
        } else {
            return Err(err);
        }
    }

    utimesat(dirfd, path, atime, mtime, symlink)
}

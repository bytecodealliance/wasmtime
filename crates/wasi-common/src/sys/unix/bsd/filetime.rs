//! This internal module consists of helper types and functions for dealing
//! with setting the file times specific to BSD-style *nixes.
use crate::{sys::unix::filetime::FileTime, Result};
use cfg_if::cfg_if;
use std::ffi::CStr;
use std::fs::File;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

cfg_if! {
    if #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "ios",
            target_os = "dragonfly"
    ))] {
        pub(crate) const UTIME_NOW: i64 = -1;
        pub(crate) const UTIME_OMIT: i64 = -2;
    } else if #[cfg(target_os = "openbsd")] {
        // These are swapped compared to macos, freebsd, ios, and dragonfly.
        // https://github.com/openbsd/src/blob/master/sys/sys/stat.h#L187
        pub(crate) const UTIME_NOW: i64 = -2;
        pub(crate) const UTIME_OMIT: i64 = -1;
    } else if #[cfg(target_os = "netbsd" )] {
        // These are the same as for Linux.
        // http://cvsweb.netbsd.org/bsdweb.cgi/src/sys/sys/stat.h?rev=1.69&content-type=text/x-cvsweb-markup&only_with_tag=MAIN
        pub(crate) const UTIME_NOW: i64 = 1_073_741_823;
        pub(crate) const UTIME_OMIT: i64 = 1_073_741_822;
    }
}

/// Wrapper for `utimensat` syscall, however, with an added twist such that `utimensat` symbol
/// is firstly resolved (i.e., we check whether it exists on the host), and only used if that is
/// the case. Otherwise, the syscall resorts to a less accurate `utimesat` emulated syscall.
/// The original implementation can be found here: [filetime::unix::macos::set_times]
///
/// [filetime::unix::macos::set_times]: https://github.com/alexcrichton/filetime/blob/master/src/unix/macos.rs#L49
pub(crate) fn utimensat(
    dirfd: &File,
    path: &str,
    atime: FileTime,
    mtime: FileTime,
    symlink_nofollow: bool,
) -> Result<()> {
    use crate::sys::unix::filetime::to_timespec;
    use std::ffi::CString;
    use std::os::unix::prelude::*;

    // Attempt to use the `utimensat` syscall, but if it's not supported by the
    // current kernel then fall back to an older syscall.
    if let Some(func) = fetch_utimensat() {
        let flags = if symlink_nofollow {
            libc::AT_SYMLINK_NOFOLLOW
        } else {
            0
        };

        let p = CString::new(path.as_bytes())?;
        let times = [to_timespec(&atime)?, to_timespec(&mtime)?];
        let rc = unsafe { func(dirfd.as_raw_fd(), p.as_ptr(), times.as_ptr(), flags) };
        if rc == 0 {
            return Ok(());
        } else {
            return Err(io::Error::last_os_error().into());
        }
    }

    super::utimesat::utimesat(dirfd, path, atime, mtime, symlink_nofollow)
}

/// Wraps `fetch` specifically targetting `utimensat` symbol. If the symbol exists
/// on the host, then returns an `Some(unsafe fn)`.
fn fetch_utimensat() -> Option<
    unsafe extern "C" fn(
        libc::c_int,
        *const libc::c_char,
        *const libc::timespec,
        libc::c_int,
    ) -> libc::c_int,
> {
    static ADDR: AtomicUsize = AtomicUsize::new(0);
    unsafe {
        fetch(&ADDR, CStr::from_bytes_with_nul_unchecked(b"utimensat\0"))
            .map(|sym| std::mem::transmute(sym))
    }
}

/// Fetches a symbol by `name` and stores it in `cache`.
fn fetch(cache: &AtomicUsize, name: &CStr) -> Option<usize> {
    match cache.load(SeqCst) {
        0 => {}
        1 => return None,
        n => return Some(n),
    }
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr() as *const _) };
    let (val, ret) = if sym.is_null() {
        (1, None)
    } else {
        (sym as usize, Some(sym as usize))
    };
    cache.store(val, SeqCst);
    return ret;
}

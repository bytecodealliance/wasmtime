use super::super::filetime::FileTime;
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
    } else if #[cfg(target_os = "openbsd")] {
        // https://github.com/openbsd/src/blob/master/sys/sys/stat.h#L187
        pub(crate) const UTIME_NOW: i64 = -2;
    } else if #[cfg(target_os = "netbsd" )] {
        // http://cvsweb.netbsd.org/bsdweb.cgi/src/sys/sys/stat.h?rev=1.69&content-type=text/x-cvsweb-markup&only_with_tag=MAIN
        pub(crate) const UTIME_NOW: i64 = 1_073_741_823;
    }
}

cfg_if! {
    if #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "ios",
            target_os = "dragonfly"
    ))] {
        pub(crate) const UTIME_OMIT: i64 = -2;
    } else if #[cfg(target_os = "openbsd")] {
        // https://github.com/openbsd/src/blob/master/sys/sys/stat.h#L187
        pub(crate) const UTIME_OMIT: i64 = -1;
    } else if #[cfg(target_os = "netbsd")] {
        // http://cvsweb.netbsd.org/bsdweb.cgi/src/sys/sys/stat.h?rev=1.69&content-type=text/x-cvsweb-markup&only_with_tag=MAIN
        pub(crate) const UTIME_OMIT: i64 = 1_073_741_822;
    }
}

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

    // Attempt to use the `utimensat` syscall, but if it's not supported by the
    // current kernel then fall back to an older syscall.
    if let Some(func) = fetch_utimensat() {
        let flags = if symlink {
            libc::AT_SYMLINK_NOFOLLOW
        } else {
            0
        };

        let p = CString::new(path.as_bytes())?;
        let times = [to_timespec(&atime), to_timespec(&mtime)];
        let rc = unsafe { func(dirfd.as_raw_fd(), p.as_ptr(), times.as_ptr(), flags) };
        if rc == 0 {
            return Ok(());
        } else {
            return Err(io::Error::last_os_error());
        }
    }

    utimesat(dirfd, path, atime, mtime, symlink)
}

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

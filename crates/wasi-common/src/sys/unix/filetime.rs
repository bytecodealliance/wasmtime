//! This internal module consists of helper types and functions for dealing
//! with setting the file times (mainly in `path_filestat_set_times` syscall for now).
//!
//! The vast majority of the code contained within and in platform-specific implementations
//! (`super::linux::filetime` and `super::bsd::filetime`) is based on the [filetime] crate.
//! Kudos @alexcrichton!
//!
//! [filetime]: https://github.com/alexcrichton/filetime
use std::fs::{self, File};
use std::io;

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        pub(crate) use super::linux::filetime::*;
    } else if #[cfg(any(
            target_os = "macos",
            target_os = "netbsd",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "ios",
            target_os = "dragonfly"
    ))] {
        pub(crate) use super::bsd::filetime::*;
    }
}

/// A wrapper `enum` around `filetime::FileTime` struct, but unlike the original, this
/// type allows the possibility of specifying `FileTime::Now` as a valid enumeration which,
/// in turn, if `utimensat` is available on the host, will use a special const setting
/// `UTIME_NOW`.
#[derive(Debug, Copy, Clone)]
pub(crate) enum FileTime {
    Now,
    Omit,
    FileTime(filetime::FileTime),
}

/// For a provided pair of access and modified `FileTime`s, converts the input to
/// `filetime::FileTime` used later in `utimensat` function. For variants `FileTime::Now`
/// and `FileTime::Omit`, this function will make two syscalls: either accessing current
/// system time, or accessing the file's metadata.
///
/// The original implementation can be found here: [filetime::unix::get_times].
///
/// [filetime::unix::get_times]: https://github.com/alexcrichton/filetime/blob/master/src/unix/utimes.rs#L42
fn get_times(
    atime: FileTime,
    mtime: FileTime,
    current: impl Fn() -> io::Result<fs::Metadata>,
) -> io::Result<(filetime::FileTime, filetime::FileTime)> {
    use std::time::SystemTime;

    let atime = match atime {
        FileTime::Now => {
            let time = SystemTime::now();
            filetime::FileTime::from_system_time(time)
        }
        FileTime::Omit => {
            let meta = current()?;
            filetime::FileTime::from_last_access_time(&meta)
        }
        FileTime::FileTime(ft) => ft,
    };

    let mtime = match mtime {
        FileTime::Now => {
            let time = SystemTime::now();
            filetime::FileTime::from_system_time(time)
        }
        FileTime::Omit => {
            let meta = current()?;
            filetime::FileTime::from_last_modification_time(&meta)
        }
        FileTime::FileTime(ft) => ft,
    };

    Ok((atime, mtime))
}

/// Combines `openat` with `utimes` to emulate `utimensat` on platforms where it is
/// not available. The logic for setting file times is based on [filetime::unix::set_file_handles_times].
///
/// [filetime::unix::set_file_handles_times]: https://github.com/alexcrichton/filetime/blob/master/src/unix/utimes.rs#L24
pub(crate) fn utimesat(
    dirfd: &File,
    path: &str,
    atime: FileTime,
    mtime: FileTime,
    symlink_nofollow: bool,
) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::prelude::*;
    // emulate *at syscall by reading the path from a combination of
    // (fd, path)
    let p = CString::new(path.as_bytes())?;
    let mut flags = libc::O_RDWR;
    if symlink_nofollow {
        flags |= libc::O_NOFOLLOW;
    }
    let fd = unsafe { libc::openat(dirfd.as_raw_fd(), p.as_ptr(), flags) };
    let f = unsafe { File::from_raw_fd(fd) };
    let (atime, mtime) = get_times(atime, mtime, || f.metadata())?;
    let times = [to_timeval(atime), to_timeval(mtime)];
    let rc = unsafe { libc::futimes(f.as_raw_fd(), times.as_ptr()) };
    return if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    };
}

/// Converts `filetime::FileTime` to `libc::timeval`. This function was taken directly from
/// [filetime] crate.
///
/// [filetime]: https://github.com/alexcrichton/filetime/blob/master/src/unix/utimes.rs#L93
fn to_timeval(ft: filetime::FileTime) -> libc::timeval {
    libc::timeval {
        tv_sec: ft.seconds(),
        tv_usec: (ft.nanoseconds() / 1000) as libc::suseconds_t,
    }
}

/// Converts `FileTime` to `libc::timespec`. If `FileTime::Now` variant is specified, this
/// resolves to `UTIME_NOW` special const, `FileTime::Omit` variant resolves to `UTIME_OMIT`, and
/// `FileTime::FileTime(ft)` where `ft := filetime::FileTime` uses [filetime] crate's original
/// implementation which can be found here: [filetime::unix::to_timespec].
///
/// [filetime]: https://github.com/alexcrichton/filetime
/// [filetime::unix::to_timespec]: https://github.com/alexcrichton/filetime/blob/master/src/unix/mod.rs#L30
pub(crate) fn to_timespec(ft: &FileTime) -> libc::timespec {
    match ft {
        FileTime::Now => libc::timespec {
            tv_sec: 0,
            tv_nsec: UTIME_NOW,
        },
        FileTime::Omit => libc::timespec {
            tv_sec: 0,
            tv_nsec: UTIME_OMIT,
        },
        // `filetime::FileTime`'s fields are normalised by definition. `ft.seconds()` return the number
        // of whole seconds, while `ft.nanoseconds()` returns only fractional part expressed in
        // nanoseconds, as underneath it uses `std::time::Duration::subsec_nanos` to populate the
        // `filetime::FileTime::nanoseconds` field. It is, therefore, OK to do an `as` cast here.
        FileTime::FileTime(ft) => libc::timespec {
            tv_sec: ft.seconds(),
            tv_nsec: ft.nanoseconds() as _,
        },
    }
}

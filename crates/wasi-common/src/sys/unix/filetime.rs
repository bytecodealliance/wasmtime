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

#[derive(Debug, Copy, Clone)]
pub(crate) enum FileTime {
    Now,
    Omit,
    FileTime(filetime::FileTime),
}

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

pub(crate) fn utimesat(
    dirfd: &File,
    path: &str,
    atime: FileTime,
    mtime: FileTime,
    symlink: bool,
) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::prelude::*;
    // emulate *at syscall by reading the path from a combination of
    // (fd, path)
    let p = CString::new(path.as_bytes())?;
    let mut flags = libc::O_RDWR;
    if !symlink {
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

fn to_timeval(ft: filetime::FileTime) -> libc::timeval {
    libc::timeval {
        tv_sec: ft.seconds(),
        tv_usec: (ft.nanoseconds() / 1000) as libc::suseconds_t,
    }
}

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
        FileTime::FileTime(ft) => libc::timespec {
            tv_sec: ft.seconds(),
            tv_nsec: ft.nanoseconds() as _,
        },
    }
}

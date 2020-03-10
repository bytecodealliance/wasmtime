//! This module consists of helper types and functions for dealing
//! with setting the file times (mainly in `path_filestat_set_times` syscall for now).
//!
//! The vast majority of the code contained within and in platform-specific implementations
//! (`super::linux::filetime` and `super::bsd::filetime`) is based on the [filetime] crate.
//! Kudos @alexcrichton!
//!
//! [filetime]: https://github.com/alexcrichton/filetime
use std::io::Result;

pub use super::sys::filetime::*;

pub(crate) trait FileTimeExt {
    #[cfg(target_pointer_width = "32")]
    fn seconds_checked(&self) -> Result<i32>;
    #[cfg(target_pointer_width = "32")]
    fn nanoseconds_checked(&self) -> Result<i32>;
    #[cfg(target_pointer_width = "64")]
    fn seconds_checked(&self) -> Result<i64>;
    #[cfg(target_pointer_width = "64")]
    fn nanoseconds_checked(&self) -> Result<i64>;
}

impl FileTimeExt for filetime::FileTime {
    #[cfg(target_pointer_width = "32")]
    fn seconds_checked(&self) -> Result<i32> {
        use std::convert::TryInto;
        use std::io::Error;
        // 32bit OS expects `tv_sec` field to be `i32`.
        // Perform a checked conversion, log error if any, and convert to libc::EOVERFLOW.
        let sec = match self.seconds().try_into() {
            Ok(sec) => sec,
            Err(_) => {
                log::debug!("filetime_to_timespec failed converting seconds to required width");
                return Err(Error::from_raw_os_error(libc::EOVERFLOW));
            }
        };
        Ok(sec)
    }
    #[cfg(target_pointer_width = "32")]
    fn nanoseconds_checked(&self) -> Result<i32> {
        use std::convert::TryInto;
        use std::io::Error;
        // 32bit OS expects `tv_nsec` field to be `i32`.
        // Perform a checked conversion, log error if any, and convert to libc::EOVERFLOW.
        let nsec = match self.nanoseconds().try_into() {
            Ok(nsec) => nsec,
            Err(_) => {
                log::debug!("filetime_to_timespec failed converting nanoseconds to required width");
                return Err(Error::from_raw_os_error(libc::EOVERFLOW));
            }
        };
        Ok(nsec)
    }
    #[cfg(target_pointer_width = "64")]
    fn seconds_checked(&self) -> Result<i64> {
        Ok(self.seconds())
    }
    #[cfg(target_pointer_width = "64")]
    fn nanoseconds_checked(&self) -> Result<i64> {
        Ok(i64::from(self.nanoseconds()))
    }
}

/// A wrapper `enum` around `filetime::FileTime` struct, but unlike the original, this
/// type allows the possibility of specifying `FileTime::Now` as a valid enumeration which,
/// in turn, if `utimensat` is available on the host, will use a special const setting
/// `UTIME_NOW`.
#[derive(Debug, Copy, Clone)]
pub enum FileTime {
    Now,
    Omit,
    FileTime(filetime::FileTime),
}

/// Converts `FileTime` to `libc::timespec`. If `FileTime::Now` variant is specified, this
/// resolves to `UTIME_NOW` special const, `FileTime::Omit` variant resolves to `UTIME_OMIT`, and
/// `FileTime::FileTime(ft)` where `ft := filetime::FileTime` uses [filetime] crate's original
/// implementation which can be found here: [filetime::unix::to_timespec].
///
/// [filetime]: https://github.com/alexcrichton/filetime
/// [filetime::unix::to_timespec]: https://github.com/alexcrichton/filetime/blob/master/src/unix/mod.rs#L30
pub(crate) fn to_timespec(ft: &FileTime) -> Result<libc::timespec> {
    let ts = match ft {
        FileTime::Now => libc::timespec {
            tv_sec: 0,
            tv_nsec: UTIME_NOW,
        },
        FileTime::Omit => libc::timespec {
            tv_sec: 0,
            tv_nsec: UTIME_OMIT,
        },
        FileTime::FileTime(ft) => libc::timespec {
            tv_sec: ft.seconds_checked()?,
            tv_nsec: ft.nanoseconds_checked()?,
        },
    };
    Ok(ts)
}

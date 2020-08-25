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

/// Internal trait which specialises `filetime::FileTime`'s
/// `seconds` and `nanoseconds` accessors for different
/// pointer widths (32 and 64bit currently).
pub(crate) trait FileTimeExt {
    fn seconds_(&self) -> Result<libc::time_t>;
    fn nanoseconds_(&self) -> libc::c_long;
}

impl FileTimeExt for filetime::FileTime {
    fn seconds_(&self) -> Result<libc::time_t> {
        use std::convert::TryInto;
        use std::io::Error;
        let sec = match self.seconds().try_into() {
            Ok(sec) => sec,
            Err(_) => {
                tracing::debug!("filetime_to_timespec failed converting seconds to required width");
                return Err(Error::from_raw_os_error(libc::EOVERFLOW));
            }
        };
        Ok(sec)
    }
    fn nanoseconds_(&self) -> libc::c_long {
        use std::convert::TryInto;
        // According to [filetime] docs, since the nanoseconds value is always less than 1 billion,
        // any value should be convertible to `libc::c_long`, hence we can `unwrap` outright.
        //
        // [filetime]: https://docs.rs/filetime/0.2.8/filetime/struct.FileTime.html#method.nanoseconds
        self.nanoseconds().try_into().unwrap()
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
            tv_nsec: libc::UTIME_NOW,
        },
        FileTime::Omit => libc::timespec {
            tv_sec: 0,
            tv_nsec: libc::UTIME_OMIT,
        },
        FileTime::FileTime(ft) => libc::timespec {
            tv_sec: ft.seconds_()?,
            tv_nsec: ft.nanoseconds_(),
        },
    };
    Ok(ts)
}

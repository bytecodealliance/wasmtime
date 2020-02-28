//! This internal module consists of helper types and functions for dealing
//! with setting the file times (mainly in `path_filestat_set_times` syscall for now).
//!
//! The vast majority of the code contained within and in platform-specific implementations
//! (`super::linux::filetime` and `super::bsd::filetime`) is based on the [filetime] crate.
//! Kudos @alexcrichton!
//!
//! [filetime]: https://github.com/alexcrichton/filetime
use crate::Result;
use std::convert::TryInto;

pub(crate) use super::sys_impl::filetime::*;

cfg_if::cfg_if! {
    if #[cfg(not(target_os = "emscripten"))] {
        fn filetime_to_timespec(ft: &filetime::FileTime) -> Result<libc::timespec> {
            Ok(
                libc::timespec {
                    tv_sec: ft.seconds(),
                    tv_nsec: ft.nanoseconds().try_into()?,
                }
            )
        }
    } else {
        fn filetime_to_timespec(ft: &filetime::FileTime) -> Result<libc::timespec> {
            Ok(
                libc::timespec {
                    tv_sec: ft.seconds().try_into()?,
                    tv_nsec: ft.nanoseconds().try_into()?,
                }
            )
        }
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
        FileTime::FileTime(ft) => filetime_to_timespec(ft)?,
    };
    Ok(ts)
}

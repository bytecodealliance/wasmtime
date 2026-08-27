use crate::filesystem::primitives::SystemTimeSpec;
use io_lifetimes::BorrowedFd;
use rustix::fs::{AtFlags, Timestamps, UTIME_NOW, UTIME_OMIT, utimensat};
use rustix::time::Timespec;
use std::path::Path;
use std::time::SystemTime;
use std::{fs, io};

#[allow(clippy::useless_conversion)]
pub(crate) fn to_timespec(ft: Option<SystemTimeSpec>) -> io::Result<Timespec> {
    Ok(match ft {
        None => Timespec {
            tv_sec: 0,
            tv_nsec: UTIME_OMIT.into(),
        },
        Some(SystemTimeSpec::Absolute(ft)) => {
            let duration = ft.duration_since(SystemTime::UNIX_EPOCH).unwrap();
            let nanoseconds = duration.subsec_nanos();
            assert_ne!(i64::from(nanoseconds), i64::from(UTIME_OMIT));
            assert_ne!(i64::from(nanoseconds), i64::from(UTIME_NOW));
            Timespec {
                tv_sec: duration
                    .as_secs()
                    .try_into()
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?,
                tv_nsec: nanoseconds.try_into().unwrap(),
            }
        }
    })
}

#[allow(dead_code)]
pub(crate) fn set_times_nofollow_unchecked(
    start: &fs::File,
    path: &Path,
    atime: Option<SystemTimeSpec>,
    mtime: Option<SystemTimeSpec>,
) -> io::Result<()> {
    let times = Timestamps {
        last_access: to_timespec(atime)?,
        last_modification: to_timespec(mtime)?,
    };
    Ok(utimensat(start, path, &times, AtFlags::SYMLINK_NOFOLLOW)?)
}

#[allow(dead_code)]
pub(crate) fn set_times_follow_unchecked(
    start: BorrowedFd<'_>,
    path: &Path,
    atime: Option<SystemTimeSpec>,
    mtime: Option<SystemTimeSpec>,
) -> io::Result<()> {
    let times = Timestamps {
        last_access: to_timespec(atime)?,
        last_modification: to_timespec(mtime)?,
    };
    Ok(utimensat(start, path, &times, AtFlags::empty())?)
}

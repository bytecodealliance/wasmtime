use rustix::fs::{UTIME_NOW, UTIME_OMIT};
use rustix::time::Timespec;
use std::io;
use std::time::SystemTime;

pub(crate) fn to_timespec(ft: Option<SystemTime>) -> io::Result<Timespec> {
    Ok(match ft {
        None => Timespec {
            tv_sec: 0,
            tv_nsec: UTIME_OMIT.into(),
        },
        Some(ft) => {
            let date_time =
                crate::clocks::Datetime::try_from(ft).map_err(|e| io::Error::other(e))?;
            let Ok(nanoseconds) = date_time.nanoseconds.try_into();
            assert_ne!(i64::from(nanoseconds), i64::from(UTIME_OMIT));
            assert_ne!(i64::from(nanoseconds), i64::from(UTIME_NOW));
            Timespec {
                tv_sec: date_time.seconds,
                tv_nsec: nanoseconds,
            }
        }
    })
}

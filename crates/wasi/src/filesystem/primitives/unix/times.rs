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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn before_unix_epoch() {
        let epoch = SystemTime::UNIX_EPOCH;
        let cases = [
            (
                epoch.checked_sub(Duration::from_secs(1)),
                Timespec {
                    tv_sec: -1,
                    tv_nsec: 0,
                },
            ),
            (
                epoch.checked_sub(Duration::from_millis(500)),
                Timespec {
                    tv_sec: -1,
                    tv_nsec: 500_000_000,
                },
            ),
            (
                epoch.checked_sub(Duration::from_nanos(1)),
                Timespec {
                    tv_sec: -1,
                    tv_nsec: 999_999_999,
                },
            ),
            (
                epoch.checked_add(Duration::from_millis(500)),
                Timespec {
                    tv_sec: 0,
                    tv_nsec: 500_000_000,
                },
            ),
        ];

        for (time, expected) in cases {
            assert_eq!(to_timespec(time).unwrap(), expected);
        }
    }
}

use super::{HostMonotonicClock, HostTimezone, HostWallClock};
use crate::bindings::clocks::timezone::TimezoneDisplay;
use cap_std::time::{Duration, Instant, SystemClock};
use cap_std::{ambient_authority, AmbientAuthority};
use cap_time_ext::{MonotonicClockExt, SystemClockExt};
use chrono::{NaiveDateTime, TimeZone};
use chrono_tz::{OffsetComponents, Tz, TZ_VARIANTS};

pub struct WallClock {
    /// The underlying system clock.
    clock: cap_std::time::SystemClock,
}

impl WallClock {
    pub fn new(ambient_authority: AmbientAuthority) -> Self {
        Self {
            clock: cap_std::time::SystemClock::new(ambient_authority),
        }
    }
}

impl HostWallClock for WallClock {
    fn resolution(&self) -> Duration {
        self.clock.resolution()
    }

    fn now(&self) -> Duration {
        // WASI defines wall clocks to return "Unix time".
        self.clock
            .now()
            .duration_since(SystemClock::UNIX_EPOCH)
            .unwrap()
    }
}

pub struct MonotonicClock {
    /// The underlying system clock.
    clock: cap_std::time::MonotonicClock,

    /// The `Instant` this clock was created. All returned times are
    /// durations since that time.
    initial: Instant,
}

impl MonotonicClock {
    pub fn new(ambient_authority: AmbientAuthority) -> Self {
        let clock = cap_std::time::MonotonicClock::new(ambient_authority);
        let initial = clock.now();
        Self { clock, initial }
    }
}

impl HostMonotonicClock for MonotonicClock {
    fn resolution(&self) -> u64 {
        self.clock.resolution().as_nanos().try_into().unwrap()
    }

    fn now(&self) -> u64 {
        // Unwrap here and in `resolution` above; a `u64` is wide enough to
        // hold over 584 years of nanoseconds.
        self.clock
            .now()
            .duration_since(self.initial)
            .as_nanos()
            .try_into()
            .unwrap()
    }
}

pub struct Timezone {
    // The underlying system timezone.
    timezone: cap_time_ext::Timezone,
}

impl Timezone {
    pub fn new(ambient_authority: AmbientAuthority) -> Self {
        Self {
            timezone: cap_time_ext::Timezone::new(ambient_authority),
        }
    }

    fn timezone_from_duration(&self, datetime: Duration) -> Option<TimezoneDisplay> {
        let name = self.timezone.timezone_name().ok()?;
        let tz: Tz = TZ_VARIANTS.into_iter().find(|tz| tz.to_string() == name)?;
        let naive_datetime = NaiveDateTime::from_timestamp_opt(datetime.as_secs() as i64, 0)?;
        let tz_offset = tz.offset_from_local_datetime(&naive_datetime).single()?;
        let utc_offset = tz_offset.base_utc_offset().num_hours() as i32;
        let in_daylight_saving_time = !tz_offset.dst_offset().is_zero();
        Some(TimezoneDisplay {
            utc_offset,
            name,
            in_daylight_saving_time,
        })
    }
}

impl HostTimezone for Timezone {
    fn display(&self, datetime: Duration) -> TimezoneDisplay {
        match self.timezone_from_duration(datetime) {
            None => TimezoneDisplay {
                utc_offset: 0,
                name: "UTC".to_string(),
                in_daylight_saving_time: false,
            },
            Some(timezone_display) => timezone_display,
        }
    }

    fn utc_offset(&self, datetime: Duration) -> i32 {
        match self.timezone_from_duration(datetime) {
            None => 0,
            Some(timezone_display) => timezone_display.utc_offset,
        }
    }
}

pub fn monotonic_clock() -> Box<dyn HostMonotonicClock + Send> {
    Box::new(MonotonicClock::new(ambient_authority()))
}

pub fn wall_clock() -> Box<dyn HostWallClock + Send> {
    Box::new(WallClock::new(ambient_authority()))
}

pub fn timezone() -> Box<dyn HostTimezone + Send> {
    Box::new(Timezone::new(ambient_authority()))
}

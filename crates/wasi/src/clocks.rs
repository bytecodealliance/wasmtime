use crate::p2::bindings::clocks::timezone::TimezoneDisplay;
use cap_std::time::{Duration, Instant, SystemClock, SystemTime};
use cap_std::{AmbientAuthority, ambient_authority};
use cap_time_ext::{MonotonicClockExt as _, SystemClockExt as _};
use jiff::Timestamp;
use jiff::tz::TimeZone as JiffTimeZone;
use std::{convert::TryFrom, str::FromStr};
use wasmtime::component::{HasData, ResourceTable};

pub(crate) struct WasiClocks;

impl HasData for WasiClocks {
    type Data<'a> = WasiClocksCtxView<'a>;
}

pub struct WasiClocksCtx {
    pub wall_clock: Box<dyn HostWallClock + Send>,
    pub monotonic_clock: Box<dyn HostMonotonicClock + Send>,
    pub timezone: Box<dyn HostTimezone + Send>,
}

impl Default for WasiClocksCtx {
    fn default() -> Self {
        Self {
            wall_clock: wall_clock(),
            monotonic_clock: monotonic_clock(),
            timezone: timezone(),
        }
    }
}

pub trait WasiClocksView: Send {
    fn clocks(&mut self) -> WasiClocksCtxView<'_>;
}

pub struct WasiClocksCtxView<'a> {
    pub ctx: &'a mut WasiClocksCtx,
    pub table: &'a mut ResourceTable,
}

pub trait HostWallClock: Send {
    fn resolution(&self) -> Duration;
    fn now(&self) -> Duration;
}

pub trait HostMonotonicClock: Send {
    fn resolution(&self) -> u64;
    fn now(&self) -> u64;
}

pub trait HostTimezone: Send {
    fn display(&self, datetime: Duration) -> TimezoneDisplay;
    fn utc_offset(&self, datetime: Duration) -> i32;
}

pub struct WallClock {
    /// The underlying system clock.
    clock: cap_std::time::SystemClock,
}

impl Default for WallClock {
    fn default() -> Self {
        Self::new(ambient_authority())
    }
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

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new(ambient_authority())
    }
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

pub fn monotonic_clock() -> Box<dyn HostMonotonicClock + Send> {
    Box::new(MonotonicClock::default())
}

pub fn wall_clock() -> Box<dyn HostWallClock + Send> {
    Box::new(WallClock::default())
}

pub fn timezone() -> Box<dyn HostTimezone + Send> {
    Box::new(Timezone::default())
}

pub(crate) struct Datetime {
    pub seconds: u64,
    pub nanoseconds: u32,
}

impl TryFrom<SystemTime> for Datetime {
    type Error = wasmtime::Error;

    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        let duration =
            time.duration_since(SystemTime::from_std(std::time::SystemTime::UNIX_EPOCH))?;

        Ok(Self {
            seconds: duration.as_secs(),
            nanoseconds: duration.subsec_nanos(),
        })
    }
}

pub struct Timezone {
    timezone: JiffTimeZone,
}

impl Default for Timezone {
    fn default() -> Self {
        Self::new()
    }
}

impl Timezone {
    pub fn new() -> Self {
        Self {
            timezone: JiffTimeZone::try_system().unwrap_or(JiffTimeZone::UTC),
        }
    }

    fn timezone_from_duration(&self, datetime: Duration) -> Option<TimezoneDisplay> {
        let timestamp = Timestamp::from_second(datetime.as_secs() as i64).ok()?;
        let localtime = self.timezone.to_offset_info(timestamp);
        let utc_offset = localtime.offset().seconds();
        let name = self.timezone.iana_name().unwrap_or("UTC").to_string();
        let in_daylight_saving_time = jiff::tz::Dst::Yes == localtime.dst();
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

impl FromStr for Timezone {
    type Err = wasmtime::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let timezone = JiffTimeZone::get(s)?;
        Ok(Timezone { timezone })
    }
}

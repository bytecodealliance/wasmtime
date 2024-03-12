pub mod host;
use crate::bindings::clocks::timezone::TimezoneDisplay;
use cap_std::time::Duration;

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

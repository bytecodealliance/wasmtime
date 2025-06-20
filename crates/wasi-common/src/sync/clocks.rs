use crate::clocks::{WasiClocks, WasiMonotonicClock, WasiSystemClock};
use cap_std::time::{Duration, Instant, SystemTime};
use cap_std::{AmbientAuthority, ambient_authority};
use cap_time_ext::{MonotonicClockExt, SystemClockExt};

pub struct SystemClock(cap_std::time::SystemClock);

impl SystemClock {
    pub fn new(ambient_authority: AmbientAuthority) -> Self {
        SystemClock(cap_std::time::SystemClock::new(ambient_authority))
    }
}
impl WasiSystemClock for SystemClock {
    fn resolution(&self) -> Duration {
        self.0.resolution()
    }
    fn now(&self, precision: Duration) -> SystemTime {
        self.0.now_with(precision)
    }
}

pub struct MonotonicClock(cap_std::time::MonotonicClock);
impl MonotonicClock {
    pub fn new(ambient_authority: AmbientAuthority) -> Self {
        MonotonicClock(cap_std::time::MonotonicClock::new(ambient_authority))
    }
}
impl WasiMonotonicClock for MonotonicClock {
    fn resolution(&self) -> Duration {
        self.0.resolution()
    }
    fn now(&self, precision: Duration) -> Instant {
        self.0.now_with(precision)
    }
}

pub fn clocks_ctx() -> WasiClocks {
    WasiClocks::new()
        .with_system(SystemClock::new(ambient_authority()))
        .with_monotonic(MonotonicClock::new(ambient_authority()))
}

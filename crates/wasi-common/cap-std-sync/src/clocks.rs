use cap_std::time::{Duration, Instant, SystemTime};
use cap_std::{ambient_authority, AmbientAuthority};
use cap_time_ext::{MonotonicClockExt, SystemClockExt};
use wasi_common::clocks::{WasiClocks, WasiMonotonicClock, WasiSystemClock};

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
    let system = Box::new(SystemClock::new(ambient_authority()));
    let monotonic = cap_std::time::MonotonicClock::new(ambient_authority());
    let creation_time = monotonic.now();
    let monotonic = Box::new(MonotonicClock(monotonic));
    WasiClocks {
        system,
        monotonic,
        creation_time,
    }
}

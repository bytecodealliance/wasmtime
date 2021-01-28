use cap_std::time::{Duration, Instant, SystemTime};
use cap_time_ext::{MonotonicClockExt, SystemClockExt};
use wasi_common::clocks::{WasiClocks, WasiMonotonicClock, WasiSystemClock};

pub struct SystemClock(cap_std::time::SystemClock);

impl SystemClock {
    pub unsafe fn new() -> Self {
        SystemClock(cap_std::time::SystemClock::new())
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
    pub unsafe fn new() -> Self {
        MonotonicClock(cap_std::time::MonotonicClock::new())
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

pub fn clocks() -> WasiClocks {
    let system = Box::new(unsafe { SystemClock::new() });
    let monotonic = unsafe { cap_std::time::MonotonicClock::new() };
    let creation_time = monotonic.now();
    let monotonic = Box::new(MonotonicClock(monotonic));
    WasiClocks {
        system,
        monotonic,
        creation_time,
    }
}

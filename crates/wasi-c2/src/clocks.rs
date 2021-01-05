use cap_std::time::{Duration, Instant, MonotonicClock, SystemClock, SystemTime};
use cap_time_ext::{MonotonicClockExt, SystemClockExt};

pub trait WasiSystemClock {
    fn resolution(&self) -> Duration;
    fn now(&self, precision: Duration) -> SystemTime;
}

impl WasiSystemClock for SystemClock {
    fn resolution(&self) -> Duration {
        SystemClockExt::resolution(self)
    }
    fn now(&self, precision: Duration) -> SystemTime {
        self.now_with(precision)
    }
}

pub trait WasiMonotonicClock {
    fn resolution(&self) -> Duration;
    fn now(&self, precision: Duration) -> Instant;
}

impl WasiMonotonicClock for MonotonicClock {
    fn resolution(&self) -> Duration {
        MonotonicClockExt::resolution(self)
    }
    fn now(&self, precision: Duration) -> Instant {
        self.now_with(precision)
    }
}

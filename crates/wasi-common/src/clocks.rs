use crate::{Error, ErrorExt};
use cap_std::time::{Duration, Instant, SystemTime};

pub enum SystemTimeSpec {
    SymbolicNow,
    Absolute(SystemTime),
}

pub trait WasiSystemClock: Send + Sync {
    fn resolution(&self) -> Duration;
    fn now(&self, precision: Duration) -> SystemTime;
}

pub trait WasiMonotonicClock: Send + Sync {
    fn resolution(&self) -> Duration;
    fn now(&self, precision: Duration) -> Instant;
}

pub struct WasiMonotonicOffsetClock {
    pub creation_time: cap_std::time::Instant,
    pub abs_clock: Box<dyn WasiMonotonicClock>,
}

impl WasiMonotonicOffsetClock {
    pub fn new(clock: impl 'static + WasiMonotonicClock) -> Self {
        Self {
            creation_time: clock.now(clock.resolution()),
            abs_clock: Box::new(clock),
        }
    }
}

pub struct WasiClocks {
    pub system: Option<Box<dyn WasiSystemClock>>,
    pub monotonic: Option<WasiMonotonicOffsetClock>,
}

impl WasiClocks {
    pub fn new() -> Self {
        Self {
            system: None,
            monotonic: None,
        }
    }

    pub fn with_system(mut self, clock: impl 'static + WasiSystemClock) -> Self {
        self.system = Some(Box::new(clock));
        self
    }

    pub fn with_monotonic(mut self, clock: impl 'static + WasiMonotonicClock) -> Self {
        self.monotonic = Some(WasiMonotonicOffsetClock::new(clock));
        self
    }

    pub fn system(&self) -> Result<&dyn WasiSystemClock, Error> {
        self.system
            .as_deref()
            .ok_or_else(|| Error::badf().context("system clock is not supported"))
    }

    pub fn monotonic(&self) -> Result<&WasiMonotonicOffsetClock, Error> {
        self.monotonic
            .as_ref()
            .ok_or_else(|| Error::badf().context("monotonic clock is not supported"))
    }
}

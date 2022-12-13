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

pub struct WasiClocks {
    pub system: Box<dyn WasiSystemClock>,
    pub monotonic: Box<dyn WasiMonotonicClock>,
    pub creation_time: cap_std::time::Instant,
    pub default_monotonic: u32,
    pub default_wall: u32,
}

pub struct MonotonicClock {
    start: Instant,
}

impl From<&dyn WasiMonotonicClock> for MonotonicClock {
    fn from(clock: &dyn WasiMonotonicClock) -> MonotonicClock {
        MonotonicClock {
            start: clock.now(Duration::from_millis(1)),
        }
    }
}

impl MonotonicClock {
    pub fn now(&self, clock: &dyn WasiMonotonicClock) -> Duration {
        clock.now(self.resolution()).duration_since(self.start)
    }
    pub fn resolution(&self) -> Duration {
        // FIXME bogus value
        Duration::from_millis(1)
    }
    pub fn new_timer(&self, initial: Duration) -> MonotonicTimer {
        MonotonicTimer { initial }
    }
}

pub struct MonotonicTimer {
    initial: Duration,
}

impl MonotonicTimer {
    pub fn current(&self) -> Duration {
        // FIXME totally bogus implementation
        self.initial
    }
}

#[derive(Default)]
pub struct WallClock;

impl WallClock {
    pub fn now(&self, clock: &dyn WasiSystemClock) -> SystemTime {
        clock.now(self.resolution())
    }
    pub fn resolution(&self) -> Duration {
        todo!()
    }
}

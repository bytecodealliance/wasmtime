use cap_std::time::{Duration, Instant, SystemTime};

pub enum SystemTimeSpec {
    SymbolicNow,
    Absolute(SystemTime),
}

pub trait WasiSystemClock {
    fn resolution(&self) -> Duration;
    fn now(&self, precision: Duration) -> SystemTime;
}

pub trait WasiMonotonicClock {
    fn resolution(&self) -> Duration;
    fn now(&self, precision: Duration) -> Instant;
}

pub struct WasiClocks {
    pub system: Box<dyn WasiSystemClock>,
    pub monotonic: Box<dyn WasiMonotonicClock>,
    pub creation_time: cap_std::time::Instant,
}

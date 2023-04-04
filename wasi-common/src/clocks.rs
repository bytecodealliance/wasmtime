use cap_std::time::Duration;

pub trait WasiWallClock: Send + Sync {
    fn resolution(&self) -> Duration;
    fn now(&self) -> Duration;
}

pub trait WasiMonotonicClock: Send + Sync {
    fn resolution(&self) -> u64;
    fn now(&self) -> u64;
}

pub struct WasiClocks {
    pub wall: Box<dyn WasiWallClock + Send + Sync>,
    pub monotonic: Box<dyn WasiMonotonicClock + Send + Sync>,
}

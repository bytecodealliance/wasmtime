use crate::Error;
use cap_std::time::Duration;

pub trait WasiWallClock: Send + Sync {
    fn resolution(&self) -> Duration;
    fn now(&self) -> Duration;
    fn dup(&self) -> Box<dyn WasiWallClock + Send + Sync>;
}

pub trait WasiMonotonicClock: Send + Sync {
    fn resolution(&self) -> u64;
    fn now(&self) -> u64;
    fn dup(&self) -> Box<dyn WasiMonotonicClock + Send + Sync>;
}

pub struct WasiClocks {
    pub default_wall_clock: Box<dyn WasiWallClock + Send + Sync>,
    pub default_monotonic_clock: Box<dyn WasiMonotonicClock + Send + Sync>,
}

pub trait TableWallClockExt {
    fn get_wall_clock(&self, fd: u32) -> Result<&(dyn WasiWallClock + Send + Sync), Error>;
    fn get_wall_clock_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut Box<dyn WasiWallClock + Send + Sync>, Error>;
}
impl TableWallClockExt for crate::table::Table {
    fn get_wall_clock(&self, fd: u32) -> Result<&(dyn WasiWallClock + Send + Sync), Error> {
        self.get::<Box<dyn WasiWallClock + Send + Sync>>(fd)
            .map(|f| f.as_ref())
    }
    fn get_wall_clock_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut Box<dyn WasiWallClock + Send + Sync>, Error> {
        self.get_mut::<Box<dyn WasiWallClock + Send + Sync>>(fd)
    }
}

pub trait TableMonotonicClockExt {
    fn get_monotonic_clock(
        &self,
        fd: u32,
    ) -> Result<&(dyn WasiMonotonicClock + Send + Sync), Error>;
    fn get_monotonic_clock_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut Box<dyn WasiMonotonicClock + Send + Sync>, Error>;
}
impl TableMonotonicClockExt for crate::table::Table {
    fn get_monotonic_clock(
        &self,
        fd: u32,
    ) -> Result<&(dyn WasiMonotonicClock + Send + Sync), Error> {
        self.get::<Box<dyn WasiMonotonicClock + Send + Sync>>(fd)
            .map(|f| f.as_ref())
    }
    fn get_monotonic_clock_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut Box<dyn WasiMonotonicClock + Send + Sync>, Error> {
        self.get_mut::<Box<dyn WasiMonotonicClock + Send + Sync>>(fd)
    }
}

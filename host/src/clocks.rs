use crate::{wasi_clocks, wasi_default_clocks, WasiCtx};
use cap_std::time::SystemTime;
use wasi_common::clocks::{TableMonotonicClockExt, TableWallClockExt};

impl TryFrom<SystemTime> for wasi_clocks::Datetime {
    type Error = anyhow::Error;

    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        let duration =
            time.duration_since(SystemTime::from_std(std::time::SystemTime::UNIX_EPOCH))?;

        Ok(wasi_clocks::Datetime {
            seconds: duration.as_secs(),
            nanoseconds: duration.subsec_nanos(),
        })
    }
}

#[async_trait::async_trait]
impl wasi_default_clocks::WasiDefaultClocks for WasiCtx {
    async fn default_wall_clock(&mut self) -> anyhow::Result<wasi_clocks::WallClock> {
        // Create a new handle to the default wall clock.
        let new = self.clocks.default_wall_clock.dup();
        Ok(self.table_mut().push(Box::new(new))?)
    }

    async fn default_monotonic_clock(&mut self) -> anyhow::Result<wasi_clocks::MonotonicClock> {
        // Create a new handle to the default monotonic clock.
        let new = self.clocks.default_monotonic_clock.dup();
        Ok(self.table_mut().push(Box::new(new))?)
    }
}

#[async_trait::async_trait]
impl wasi_clocks::WasiClocks for WasiCtx {
    async fn monotonic_clock_now(
        &mut self,
        fd: wasi_clocks::MonotonicClock,
    ) -> anyhow::Result<wasi_clocks::Instant> {
        Ok(self.table().get_monotonic_clock(fd)?.now())
    }

    async fn monotonic_clock_resolution(
        &mut self,
        fd: wasi_clocks::MonotonicClock,
    ) -> anyhow::Result<wasi_clocks::Instant> {
        Ok(self.table().get_monotonic_clock(fd)?.now())
    }

    async fn wall_clock_now(
        &mut self,
        fd: wasi_clocks::WallClock,
    ) -> anyhow::Result<wasi_clocks::Datetime> {
        let clock = self.table().get_wall_clock(fd)?;
        let now = clock.now();
        Ok(wasi_clocks::Datetime {
            seconds: now.as_secs(),
            nanoseconds: now.subsec_nanos(),
        })
    }

    async fn wall_clock_resolution(
        &mut self,
        fd: wasi_clocks::WallClock,
    ) -> anyhow::Result<wasi_clocks::Datetime> {
        let clock = self.table().get_wall_clock(fd)?;
        let res = clock.resolution();
        Ok(wasi_clocks::Datetime {
            seconds: res.as_secs(),
            nanoseconds: res.subsec_nanos(),
        })
    }
}

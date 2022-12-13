use crate::{wasi_clocks, wasi_default_clocks, WasiCtx};
use anyhow::Context;
use cap_std::time::SystemTime;
use wasi_common::clocks::{MonotonicClock, MonotonicTimer, WallClock};

impl TryFrom<SystemTime> for wasi_clocks::Datetime {
    type Error = anyhow::Error;

    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        drop(time);
        todo!()
    }
}

#[async_trait::async_trait]
impl wasi_default_clocks::WasiDefaultClocks for WasiCtx {
    async fn default_monotonic_clock(&mut self) -> anyhow::Result<wasi_clocks::MonotonicClock> {
        Ok(self.clocks.default_monotonic)
    }
    async fn default_wall_clock(&mut self) -> anyhow::Result<wasi_clocks::WallClock> {
        Ok(self.clocks.default_wall)
    }
}

#[async_trait::async_trait]
impl wasi_clocks::WasiClocks for WasiCtx {
    async fn subscribe_wall_clock(
        &mut self,
        when: wasi_clocks::Datetime,
        absolute: bool,
    ) -> anyhow::Result<wasi_clocks::WasiFuture> {
        drop((when, absolute));
        todo!()
    }

    async fn subscribe_monotonic_clock(
        &mut self,
        when: wasi_clocks::Instant,
        absolute: bool,
    ) -> anyhow::Result<wasi_clocks::WasiFuture> {
        drop((when, absolute));
        todo!()
    }

    async fn monotonic_clock_now(
        &mut self,
        fd: wasi_clocks::MonotonicClock,
    ) -> anyhow::Result<wasi_clocks::Instant> {
        let clock = self.table.get::<MonotonicClock>(fd)?;
        let now = clock.now(self.clocks.monotonic.as_ref());
        Ok(now
            .as_nanos()
            .try_into()
            .context("converting monotonic time to nanos u64")?)
    }

    async fn monotonic_clock_resolution(
        &mut self,
        fd: wasi_clocks::MonotonicClock,
    ) -> anyhow::Result<wasi_clocks::Instant> {
        let clock = self.table.get::<MonotonicClock>(fd)?;
        let res = clock.resolution();
        Ok(res
            .as_nanos()
            .try_into()
            .context("converting monotonic resolution to nanos u64")?)
    }

    async fn monotonic_clock_new_timer(
        &mut self,
        fd: wasi_clocks::MonotonicClock,
        initial: wasi_clocks::Instant,
    ) -> anyhow::Result<wasi_clocks::MonotonicTimer> {
        let clock = self.table.get::<MonotonicClock>(fd)?;
        let timer = clock.new_timer(std::time::Duration::from_micros(initial));
        drop(clock);
        let timer_fd = self.table.push(Box::new(timer))?;
        Ok(timer_fd)
    }

    async fn wall_clock_now(
        &mut self,
        fd: wasi_clocks::WallClock,
    ) -> anyhow::Result<wasi_clocks::Datetime> {
        let clock = self.table.get::<WallClock>(fd)?;
        Ok(clock.now(self.clocks.system.as_ref()).try_into()?)
    }

    async fn wall_clock_resolution(
        &mut self,
        fd: wasi_clocks::WallClock,
    ) -> anyhow::Result<wasi_clocks::Datetime> {
        let clock = self.table.get::<WallClock>(fd)?;
        let nanos = clock.resolution().as_nanos();
        Ok(wasi_clocks::Datetime {
            seconds: (nanos / 1_000_000_000_u128)
                .try_into()
                .context("converting wall clock resolution to seconds u64")?,
            nanoseconds: (nanos % 1_000_000_000_u128).try_into().unwrap(),
        })
    }

    async fn monotonic_timer_current(
        &mut self,
        fd: wasi_clocks::MonotonicTimer,
    ) -> anyhow::Result<wasi_clocks::Instant> {
        let timer = self.table.get::<MonotonicTimer>(fd)?;
        Ok(timer
            .current()
            .as_nanos()
            .try_into()
            .context("converting monotonic timer to nanos u64")?)
    }
}

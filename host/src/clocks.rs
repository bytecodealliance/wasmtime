use crate::{wasi_clocks, wasi_default_clocks, WasiCtx};
use anyhow::Context;

pub struct MonotonicClock {
    start: std::time::Instant,
}

impl Default for MonotonicClock {
    fn default() -> MonotonicClock {
        MonotonicClock {
            start: std::time::Instant::now(),
        }
    }
}

impl MonotonicClock {
    fn now(&self) -> std::time::Duration {
        std::time::Instant::now().duration_since(self.start)
    }
    fn resolution(&self) -> std::time::Duration {
        // FIXME bogus value
        std::time::Duration::from_millis(1)
    }
    fn new_timer(&self, initial: std::time::Duration) -> MonotonicTimer {
        MonotonicTimer { initial }
    }
}

pub struct MonotonicTimer {
    initial: std::time::Duration,
}

impl MonotonicTimer {
    fn current(&self) -> std::time::Duration {
        // FIXME totally bogus implementation
        self.initial
    }
}

#[derive(Default)]
pub struct WallClock;

impl WallClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }
    fn resolution(&self) -> std::time::SystemTime {
        todo!()
    }
}

impl TryInto<wasi_clocks::Datetime> for std::time::SystemTime {
    type Error = anyhow::Error;
    fn try_into(self) -> anyhow::Result<wasi_clocks::Datetime> {
        todo!()
    }
}

impl wasi_default_clocks::WasiDefaultClocks for WasiCtx {
    fn default_monotonic_clock(&mut self) -> anyhow::Result<wasi_clocks::MonotonicClock> {
        Ok(self.default_monotonic)
    }
    fn default_wall_clock(&mut self) -> anyhow::Result<wasi_clocks::WallClock> {
        Ok(self.default_wall)
    }
}

impl wasi_clocks::WasiClocks for WasiCtx {
    fn subscribe_wall_clock(
        &mut self,
        when: wasi_clocks::Datetime,
        absolute: bool,
    ) -> anyhow::Result<wasi_clocks::WasiFuture> {
        drop((when, absolute));
        todo!()
    }

    fn subscribe_monotonic_clock(
        &mut self,
        when: wasi_clocks::Instant,
        absolute: bool,
    ) -> anyhow::Result<wasi_clocks::WasiFuture> {
        drop((when, absolute));
        todo!()
    }

    fn monotonic_clock_now(
        &mut self,
        fd: wasi_clocks::MonotonicClock,
    ) -> anyhow::Result<wasi_clocks::Instant> {
        let clock = self.table.get::<MonotonicClock>(fd)?;
        let now = clock.now();
        Ok(now
            .as_nanos()
            .try_into()
            .context("converting monotonic time to nanos u64")?)
    }
    fn monotonic_clock_resolution(
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

    fn monotonic_clock_new_timer(
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

    fn wall_clock_now(
        &mut self,
        fd: wasi_clocks::WallClock,
    ) -> anyhow::Result<wasi_clocks::Datetime> {
        let clock = self.table.get::<WallClock>(fd)?;
        Ok(clock.now().try_into()?)
    }

    fn wall_clock_resolution(
        &mut self,
        fd: wasi_clocks::WallClock,
    ) -> anyhow::Result<wasi_clocks::Datetime> {
        let clock = self.table.get::<WallClock>(fd)?;
        Ok(clock.resolution().try_into()?)
    }

    fn monotonic_timer_current(
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

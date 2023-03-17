#![allow(unused_variables)]

use crate::command::wasi::{
    instance_monotonic_clock, instance_wall_clock,
    monotonic_clock::{self, Instant, MonotonicClock},
    poll::Pollable,
    timezone::{self, Timezone, TimezoneDisplay},
    wall_clock::{self, Datetime, WallClock},
};
use crate::poll::PollableEntry;
use crate::WasiCtx;
use cap_std::time::SystemTime;
use wasi_common::clocks::{TableMonotonicClockExt, TableWallClockExt};

impl TryFrom<SystemTime> for Datetime {
    type Error = anyhow::Error;

    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        let duration =
            time.duration_since(SystemTime::from_std(std::time::SystemTime::UNIX_EPOCH))?;

        Ok(Datetime {
            seconds: duration.as_secs(),
            nanoseconds: duration.subsec_nanos(),
        })
    }
}

#[async_trait::async_trait]
impl instance_wall_clock::Host for WasiCtx {
    async fn instance_wall_clock(&mut self) -> anyhow::Result<WallClock> {
        // Create a new handle to the default wall clock.
        let new = self.clocks.instance_wall_clock.dup();
        Ok(self.table_mut().push(Box::new(new))?)
    }
}

#[async_trait::async_trait]
impl instance_monotonic_clock::Host for WasiCtx {
    async fn instance_monotonic_clock(&mut self) -> anyhow::Result<MonotonicClock> {
        // Create a new handle to the default monotonic clock.
        let new = self.clocks.instance_monotonic_clock.dup();
        Ok(self.table_mut().push(Box::new(new))?)
    }
}

#[async_trait::async_trait]
impl wall_clock::Host for WasiCtx {
    async fn now(&mut self, fd: WallClock) -> anyhow::Result<Datetime> {
        let clock = self.table().get_wall_clock(fd)?;
        let now = clock.now();
        Ok(Datetime {
            seconds: now.as_secs(),
            nanoseconds: now.subsec_nanos(),
        })
    }

    async fn resolution(&mut self, fd: WallClock) -> anyhow::Result<Datetime> {
        let clock = self.table().get_wall_clock(fd)?;
        let res = clock.resolution();
        Ok(Datetime {
            seconds: res.as_secs(),
            nanoseconds: res.subsec_nanos(),
        })
    }

    async fn drop_wall_clock(&mut self, clock: WallClock) -> anyhow::Result<()> {
        Ok(self.table_mut().delete_wall_clock(clock)?)
    }
}

#[async_trait::async_trait]
impl monotonic_clock::Host for WasiCtx {
    async fn now(&mut self, fd: MonotonicClock) -> anyhow::Result<Instant> {
        Ok(self.table().get_monotonic_clock(fd)?.now())
    }

    async fn resolution(&mut self, fd: MonotonicClock) -> anyhow::Result<Instant> {
        Ok(self.table().get_monotonic_clock(fd)?.now())
    }

    async fn drop_monotonic_clock(&mut self, clock: MonotonicClock) -> anyhow::Result<()> {
        Ok(self.table_mut().delete_monotonic_clock(clock)?)
    }

    async fn subscribe(
        &mut self,
        clock: MonotonicClock,
        when: Instant,
        absolute: bool,
    ) -> anyhow::Result<Pollable> {
        Ok(self
            .table_mut()
            .push(Box::new(PollableEntry::MonotonicClock(
                clock, when, absolute,
            )))?)
    }
}

#[async_trait::async_trait]
impl timezone::Host for WasiCtx {
    async fn display(
        &mut self,
        timezone: Timezone,
        when: Datetime,
    ) -> anyhow::Result<TimezoneDisplay> {
        todo!()
    }

    async fn utc_offset(&mut self, timezone: Timezone, when: Datetime) -> anyhow::Result<i32> {
        todo!()
    }

    async fn drop_timezone(&mut self, timezone: Timezone) -> anyhow::Result<()> {
        todo!()
    }
}

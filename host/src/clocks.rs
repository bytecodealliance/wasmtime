#![allow(unused_variables)]

use crate::command::wasi::{
    monotonic_clock::{self, Instant},
    poll::Pollable,
    timezone::{self, Timezone, TimezoneDisplay},
    wall_clock::{self, Datetime},
};
use crate::poll::PollableEntry;
use crate::WasiCtx;
use cap_std::time::SystemTime;

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
impl wall_clock::Host for WasiCtx {
    async fn now(&mut self) -> anyhow::Result<Datetime> {
        let now = self.clocks.wall.now();
        Ok(Datetime {
            seconds: now.as_secs(),
            nanoseconds: now.subsec_nanos(),
        })
    }

    async fn resolution(&mut self) -> anyhow::Result<Datetime> {
        let res = self.clocks.wall.resolution();
        Ok(Datetime {
            seconds: res.as_secs(),
            nanoseconds: res.subsec_nanos(),
        })
    }
}

#[async_trait::async_trait]
impl monotonic_clock::Host for WasiCtx {
    async fn now(&mut self) -> anyhow::Result<Instant> {
        Ok(self.clocks.monotonic.now())
    }

    async fn resolution(&mut self) -> anyhow::Result<Instant> {
        Ok(self.clocks.monotonic.resolution())
    }

    async fn subscribe(&mut self, when: Instant, absolute: bool) -> anyhow::Result<Pollable> {
        Ok(self
            .table_mut()
            .push(Box::new(PollableEntry::MonotonicClock(when, absolute)))?)
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

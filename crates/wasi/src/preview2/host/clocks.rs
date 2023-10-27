#![allow(unused_variables)]

use crate::preview2::bindings::{
    clocks::monotonic_clock::{self, Instant},
    clocks::timezone::{self, TimezoneDisplay},
    clocks::wall_clock::{self, Datetime},
};
use crate::preview2::poll::{subscribe, Subscribe};
use crate::preview2::{Pollable, WasiView};
use cap_std::time::SystemTime;
use std::time::Duration;
use wasmtime::component::Resource;

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

impl<T: WasiView> wall_clock::Host for T {
    fn now(&mut self) -> anyhow::Result<Datetime> {
        let now = self.ctx().wall_clock.now();
        Ok(Datetime {
            seconds: now.as_secs(),
            nanoseconds: now.subsec_nanos(),
        })
    }

    fn resolution(&mut self) -> anyhow::Result<Datetime> {
        let res = self.ctx().wall_clock.resolution();
        Ok(Datetime {
            seconds: res.as_secs(),
            nanoseconds: res.subsec_nanos(),
        })
    }
}

impl<T: WasiView> monotonic_clock::Host for T {
    fn now(&mut self) -> anyhow::Result<Instant> {
        Ok(self.ctx().monotonic_clock.now())
    }

    fn resolution(&mut self) -> anyhow::Result<Instant> {
        Ok(self.ctx().monotonic_clock.resolution())
    }

    fn subscribe(&mut self, when: Instant, absolute: bool) -> anyhow::Result<Resource<Pollable>> {
        let clock_now = self.ctx().monotonic_clock.now();
        let duration = if absolute {
            Duration::from_nanos(when.saturating_sub(clock_now))
        } else {
            Duration::from_nanos(when)
        };
        let deadline = tokio::time::Instant::now()
            .checked_add(duration)
            .ok_or_else(|| anyhow::anyhow!("time overflow: duration {duration:?}"))?;
        // NB: this resource created here is not actually exposed to wasm, it's
        // only an internal implementation detail used to match the signature
        // expected by `subscribe`.
        let sleep = self.table_mut().push(Sleep(deadline))?;
        subscribe(self.table_mut(), sleep)
    }
}

struct Sleep(tokio::time::Instant);

#[async_trait::async_trait]
impl Subscribe for Sleep {
    async fn ready(&mut self) {
        tokio::time::sleep_until(self.0).await;
    }
}

impl<T: WasiView> timezone::Host for T {
    fn display(&mut self, when: Datetime) -> anyhow::Result<TimezoneDisplay> {
        todo!("timezone display is not implemented")
    }

    fn utc_offset(&mut self, when: Datetime) -> anyhow::Result<i32> {
        todo!("timezone utc_offset is not implemented")
    }
}

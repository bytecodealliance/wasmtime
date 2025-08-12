use crate::clocks::WasiClocksCtxView;
use crate::p2::DynPollable;
use crate::p2::bindings::{
    clocks::monotonic_clock::{self, Duration as WasiDuration, Instant},
    clocks::wall_clock::{self, Datetime},
};
use cap_std::time::SystemTime;
use std::time::Duration;
use wasmtime::component::Resource;
use wasmtime_wasi_io::poll::{Pollable, subscribe};

impl From<crate::clocks::Datetime> for Datetime {
    fn from(
        crate::clocks::Datetime {
            seconds,
            nanoseconds,
        }: crate::clocks::Datetime,
    ) -> Self {
        Self {
            seconds,
            nanoseconds,
        }
    }
}

impl From<Datetime> for crate::clocks::Datetime {
    fn from(
        Datetime {
            seconds,
            nanoseconds,
        }: Datetime,
    ) -> Self {
        Self {
            seconds,
            nanoseconds,
        }
    }
}

impl TryFrom<SystemTime> for Datetime {
    type Error = wasmtime::Error;

    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        let time = crate::clocks::Datetime::try_from(time)?;
        Ok(time.into())
    }
}

impl wall_clock::Host for WasiClocksCtxView<'_> {
    fn now(&mut self) -> anyhow::Result<Datetime> {
        let now = self.ctx.wall_clock.now();
        Ok(Datetime {
            seconds: now.as_secs(),
            nanoseconds: now.subsec_nanos(),
        })
    }

    fn resolution(&mut self) -> anyhow::Result<Datetime> {
        let res = self.ctx.wall_clock.resolution();
        Ok(Datetime {
            seconds: res.as_secs(),
            nanoseconds: res.subsec_nanos(),
        })
    }
}

fn subscribe_to_duration(
    table: &mut wasmtime::component::ResourceTable,
    duration: tokio::time::Duration,
) -> anyhow::Result<Resource<DynPollable>> {
    let sleep = if duration.is_zero() {
        table.push(Deadline::Past)?
    } else if let Some(deadline) = tokio::time::Instant::now().checked_add(duration) {
        // NB: this resource created here is not actually exposed to wasm, it's
        // only an internal implementation detail used to match the signature
        // expected by `subscribe`.
        table.push(Deadline::Instant(deadline))?
    } else {
        // If the user specifies a time so far in the future we can't
        // represent it, wait forever rather than trap.
        table.push(Deadline::Never)?
    };
    subscribe(table, sleep)
}

impl monotonic_clock::Host for WasiClocksCtxView<'_> {
    fn now(&mut self) -> anyhow::Result<Instant> {
        Ok(self.ctx.monotonic_clock.now())
    }

    fn resolution(&mut self) -> anyhow::Result<Instant> {
        Ok(self.ctx.monotonic_clock.resolution())
    }

    fn subscribe_instant(&mut self, when: Instant) -> anyhow::Result<Resource<DynPollable>> {
        let clock_now = self.ctx.monotonic_clock.now();
        let duration = if when > clock_now {
            Duration::from_nanos(when - clock_now)
        } else {
            Duration::from_nanos(0)
        };
        subscribe_to_duration(self.table, duration)
    }

    fn subscribe_duration(
        &mut self,
        duration: WasiDuration,
    ) -> anyhow::Result<Resource<DynPollable>> {
        subscribe_to_duration(self.table, Duration::from_nanos(duration))
    }
}

enum Deadline {
    Past,
    Instant(tokio::time::Instant),
    Never,
}

#[async_trait::async_trait]
impl Pollable for Deadline {
    async fn ready(&mut self) {
        match self {
            Deadline::Past => {}
            Deadline::Instant(instant) => tokio::time::sleep_until(*instant).await,
            Deadline::Never => std::future::pending().await,
        }
    }
}

use crate::p3::bindings::{
    clocks::monotonic_clock::{self, Duration as WasiDuration, Instant},
    clocks::wall_clock::{self, Datetime},
};
use crate::{WasiImpl, WasiView};
use cap_std::time::SystemTime;
use std::time::Duration;
use tokio::time::sleep;

mod sync;

impl TryFrom<SystemTime> for Datetime {
    type Error = anyhow::Error;

    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        let duration =
            time.duration_since(SystemTime::from_std(std::time::SystemTime::UNIX_EPOCH))?;

        Ok(Self {
            seconds: duration.as_secs(),
            nanoseconds: duration.subsec_nanos(),
        })
    }
}

impl<T> wall_clock::Host for WasiImpl<T>
where
    T: WasiView,
{
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

impl<T> monotonic_clock::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn now(&mut self) -> anyhow::Result<Instant> {
        Ok(self.ctx().monotonic_clock.now())
    }

    fn resolution(&mut self) -> anyhow::Result<Instant> {
        Ok(self.ctx().monotonic_clock.resolution())
    }

    async fn wait_until(&mut self, when: Instant) -> anyhow::Result<()> {
        let clock_now = self.ctx().monotonic_clock.now();
        if when > clock_now {
            sleep(Duration::from_nanos(when - clock_now)).await;
        };
        Ok(())
    }

    async fn wait_for(&mut self, duration: WasiDuration) -> anyhow::Result<()> {
        if duration > 0 {
            sleep(Duration::from_nanos(duration)).await;
        }
        Ok(())
    }
}

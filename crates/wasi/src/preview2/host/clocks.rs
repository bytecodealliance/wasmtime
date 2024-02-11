use crate::preview2::bindings::{
    clocks::monotonic_clock::{self, Duration as WasiDuration, Instant},
    clocks::wall_clock::{self, Datetime},
};
use crate::preview2::poll;
use crate::preview2::{Pollable, PollableInternal, WasiView};
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

fn make_pollable(duration: tokio::time::Duration) -> Box<dyn PollableInternal> {
    if duration.is_zero() {
        Box::new(poll::ready())
    } else if let Some(deadline) = tokio::time::Instant::now().checked_add(duration) {
        Box::new(poll::once(async move {
            tokio::time::sleep_until(deadline).await
        }))
    } else {
        // If the user specifies a time so far in the future we can't
        // represent it, wait forever rather than trap.
        Box::new(poll::pending())
    }
}

impl<T: WasiView> monotonic_clock::Host for T {
    fn now(&mut self) -> anyhow::Result<Instant> {
        Ok(self.ctx().monotonic_clock.now())
    }

    fn resolution(&mut self) -> anyhow::Result<Instant> {
        Ok(self.ctx().monotonic_clock.resolution())
    }

    fn subscribe_instant(&mut self, when: Instant) -> anyhow::Result<Resource<Pollable>> {
        let clock_now = self.ctx().monotonic_clock.now();
        let duration = if when > clock_now {
            Duration::from_nanos(when - clock_now)
        } else {
            Duration::from_nanos(0)
        };
        let pollable = make_pollable(duration);
        Ok(self.table().push(Pollable::own(pollable))?)
    }

    fn subscribe_duration(&mut self, duration: WasiDuration) -> anyhow::Result<Resource<Pollable>> {
        let pollable = make_pollable(Duration::from_nanos(duration));
        Ok(self.table().push(Pollable::own(pollable))?)
    }
}

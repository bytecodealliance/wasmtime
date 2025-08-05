use crate::clocks::WasiClocksCtxView;
use crate::p3::bindings::clocks::{monotonic_clock, wall_clock};
use crate::p3::clocks::WasiClocks;
use cap_std::time::SystemTime;
use core::time::Duration;
use tokio::time::sleep;
use wasmtime::component::Accessor;

impl TryFrom<SystemTime> for wall_clock::Datetime {
    type Error = wasmtime::Error;

    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        let duration =
            time.duration_since(SystemTime::from_std(std::time::SystemTime::UNIX_EPOCH))?;

        Ok(Self {
            seconds: duration.as_secs(),
            nanoseconds: duration.subsec_nanos(),
        })
    }
}

impl wall_clock::Host for WasiClocksCtxView<'_> {
    fn now(&mut self) -> wasmtime::Result<wall_clock::Datetime> {
        let now = self.ctx.wall_clock.now();
        Ok(wall_clock::Datetime {
            seconds: now.as_secs(),
            nanoseconds: now.subsec_nanos(),
        })
    }

    fn resolution(&mut self) -> wasmtime::Result<wall_clock::Datetime> {
        let res = self.ctx.wall_clock.resolution();
        Ok(wall_clock::Datetime {
            seconds: res.as_secs(),
            nanoseconds: res.subsec_nanos(),
        })
    }
}

impl monotonic_clock::HostWithStore for WasiClocks {
    async fn wait_until<U>(
        store: &Accessor<U, Self>,
        when: monotonic_clock::Instant,
    ) -> wasmtime::Result<()> {
        let clock_now = store.with(|mut view| view.get().ctx.monotonic_clock.now());
        if when > clock_now {
            sleep(Duration::from_nanos(when - clock_now)).await;
        };
        Ok(())
    }

    async fn wait_for<U>(
        _store: &Accessor<U, Self>,
        duration: monotonic_clock::Duration,
    ) -> wasmtime::Result<()> {
        if duration > 0 {
            sleep(Duration::from_nanos(duration)).await;
        }
        Ok(())
    }
}

impl monotonic_clock::Host for WasiClocksCtxView<'_> {
    fn now(&mut self) -> wasmtime::Result<monotonic_clock::Instant> {
        Ok(self.ctx.monotonic_clock.now())
    }

    fn resolution(&mut self) -> wasmtime::Result<monotonic_clock::Instant> {
        Ok(self.ctx.monotonic_clock.resolution())
    }
}

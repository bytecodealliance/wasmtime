use crate::clocks::WasiClocksCtxView;
use crate::p3::bindings::clocks::{monotonic_clock, system_clock};
use crate::p3::clocks::WasiClocks;
use core::time::Duration;
use tokio::time::sleep;
use wasmtime::component::Accessor;

impl system_clock::Host for WasiClocksCtxView<'_> {
    fn now(&mut self) -> wasmtime::Result<system_clock::Instant> {
        let now = self.ctx.wall_clock.now();
        // Convert from u64 seconds (since Unix epoch) to s64 seconds
        // This should always fit since we're dealing with current time
        let seconds = now.as_secs() as i64;
        Ok(system_clock::Instant {
            seconds,
            nanoseconds: now.subsec_nanos(),
        })
    }

    fn get_resolution(&mut self) -> wasmtime::Result<system_clock::Duration> {
        let res = self.ctx.wall_clock.resolution();
        // Resolution is always positive, convert Duration to nanoseconds (u64)
        Ok(res
            .as_nanos()
            .try_into()
            .map_err(|_| wasmtime::Error::msg("resolution too large"))?)
    }
}

impl monotonic_clock::HostWithStore for WasiClocks {
    async fn wait_until<U>(
        store: &Accessor<U, Self>,
        when: monotonic_clock::Mark,
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
    fn now(&mut self) -> wasmtime::Result<monotonic_clock::Mark> {
        Ok(self.ctx.monotonic_clock.now())
    }

    fn get_resolution(&mut self) -> wasmtime::Result<monotonic_clock::Duration> {
        Ok(self.ctx.monotonic_clock.resolution())
    }
}

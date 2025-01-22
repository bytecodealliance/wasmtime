use crate::p3::bindings::clocks as async_clocks;
use crate::p3::bindings::sync::clocks as sync_clocks;
use crate::p3::bindings::sync::clocks::monotonic_clock::{Duration, Instant};
use crate::runtime::in_tokio;
use crate::{WasiImpl, WasiView};

impl<T> sync_clocks::monotonic_clock::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn now(&mut self) -> anyhow::Result<Instant> {
        async_clocks::monotonic_clock::Host::now(self)
    }

    fn resolution(&mut self) -> anyhow::Result<Instant> {
        async_clocks::monotonic_clock::Host::resolution(self)
    }

    fn wait_until(&mut self, when: Instant) -> anyhow::Result<()> {
        in_tokio(async_clocks::monotonic_clock::Host::wait_until(self, when))
    }

    fn wait_for(&mut self, duration: Duration) -> anyhow::Result<()> {
        in_tokio(async_clocks::monotonic_clock::Host::wait_for(
            self, duration,
        ))
    }
}

use crate::clocks::WasiClocksCtxView;
use crate::p3::bindings::clocks::{monotonic_clock, system_clock, types};
use crate::p3::clocks::WasiClocks;
use core::time::Duration;
use tokio::time::sleep;
use wasmtime::component::Accessor;

impl WasiClocksCtxView<'_> {
    fn monotonic_wait_until_duration(&mut self, when: monotonic_clock::Mark) -> Option<Duration> {
        let clock_now = self.ctx.monotonic_clock.now();
        if when > clock_now {
            Some(Duration::from_nanos(when - clock_now))
        } else {
            None
        }
    }
}

impl types::Host for WasiClocksCtxView<'_> {}

impl system_clock::Host for WasiClocksCtxView<'_> {
    fn now(&mut self) -> wasmtime::Result<system_clock::Instant> {
        let now = self.ctx.wall_clock.now();
        Ok(system_clock::Instant {
            seconds: now.as_secs().try_into()?,
            nanoseconds: now.subsec_nanos(),
        })
    }

    fn get_resolution(&mut self) -> wasmtime::Result<types::Duration> {
        let res = self.ctx.wall_clock.resolution();
        Ok(res.as_nanos().try_into()?)
    }
}

impl<U> monotonic_clock::HostWithStore<U> for WasiClocks {
    async fn wait_until(
        store: &Accessor<U, Self>,
        when: monotonic_clock::Mark,
    ) -> wasmtime::Result<()> {
        if let Some(dur) = store.with(|mut view| view.get().monotonic_wait_until_duration(when)) {
            sleep(dur).await;
        }
        Ok(())
    }

    async fn wait_for(
        _store: &Accessor<U, Self>,
        duration: types::Duration,
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

    fn get_resolution(&mut self) -> wasmtime::Result<types::Duration> {
        Ok(self.ctx.monotonic_clock.resolution())
    }
}

mod named {
    use crate::clocks::{WasiClocksNamed, WasiClocksNamedView};
    use crate::p3::bindings::clocks::monotonic_clock::Mark;
    use crate::p3::bindings::clocks::system_clock::Instant;
    use crate::p3::bindings::clocks::types;
    use crate::p3::bindings::named_imports::wasi::clocks::{monotonic_clock, system_clock};
    use crate::{NamedId, WasiCtxNamedView};
    use core::time::Duration;
    use tokio::time::sleep;
    use wasmtime::component::Accessor;

    impl<T> system_clock::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiClocksNamedView,
    {
        fn now(&mut self, id: NamedId) -> wasmtime::Result<Instant> {
            super::system_clock::Host::now(&mut self.0.clocks(id))
        }

        fn get_resolution(&mut self, id: NamedId) -> wasmtime::Result<types::Duration> {
            super::system_clock::Host::get_resolution(&mut self.0.clocks(id))
        }
    }

    impl<T, U> monotonic_clock::HostWithStore<U> for WasiClocksNamed<T>
    where
        T: WasiClocksNamedView,
    {
        async fn wait_until(
            store: &Accessor<U, Self>,
            id: NamedId,
            when: Mark,
        ) -> wasmtime::Result<()> {
            if let Some(dur) =
                store.with(|mut view| view.get().0.clocks(id).monotonic_wait_until_duration(when))
            {
                sleep(dur).await;
            }
            Ok(())
        }

        async fn wait_for(
            _store: &Accessor<U, Self>,
            _id: NamedId,
            duration: types::Duration,
        ) -> wasmtime::Result<()> {
            if duration > 0 {
                sleep(Duration::from_nanos(duration)).await;
            }
            Ok(())
        }
    }

    impl<T> monotonic_clock::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiClocksNamedView,
    {
        fn now(&mut self, id: NamedId) -> wasmtime::Result<Mark> {
            super::monotonic_clock::Host::now(&mut self.0.clocks(id))
        }

        fn get_resolution(&mut self, id: NamedId) -> wasmtime::Result<types::Duration> {
            super::monotonic_clock::Host::get_resolution(&mut self.0.clocks(id))
        }
    }
}

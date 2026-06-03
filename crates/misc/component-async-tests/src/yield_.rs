use super::Ctx;
use wasmtime::component::Accessor;

wasmtime::component::bindgen!({
    path: "wit",
    world: "yield-host",
});

impl<T> local::local::yield_::HostWithStore<T> for Ctx {
    async fn yield_times(_: &Accessor<T, Self>, times: u64) {
        crate::util::yield_times(times.try_into().unwrap()).await;
    }
}

impl local::local::yield_::Host for Ctx {}

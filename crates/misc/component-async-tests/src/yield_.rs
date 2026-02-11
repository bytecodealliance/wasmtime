use super::Ctx;
use wasmtime::component::Accessor;

wasmtime::component::bindgen!({
    path: "wit",
    world: "yield-host",
});

impl local::local::yield_::HostWithStore for Ctx {
    async fn yield_times<T>(_: &Accessor<T, Self>, times: u64) {
        crate::util::yield_times(times.try_into().unwrap()).await;
    }
}

impl local::local::yield_::Host for Ctx {}

use super::Ctx;
use std::time::Duration;
use wasmtime::component::Accessor;

wasmtime::component::bindgen!({
    path: "wit",
    world: "sleep-host",
    concurrent_imports: true,
    concurrent_exports: true,
    async: {
        only_imports: [
            "local:local/sleep#[async]sleep-millis",
        ]
    },
});

impl local::local::sleep::HostConcurrent for Ctx {
    async fn sleep_millis<T>(_: &Accessor<T, Self>, time_in_millis: u64) {
        crate::util::sleep(Duration::from_millis(time_in_millis)).await;
    }
}

impl local::local::sleep::Host for Ctx {}

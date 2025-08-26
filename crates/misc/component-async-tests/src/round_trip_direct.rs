use super::Ctx;
use std::time::Duration;
use wasmtime::component::Accessor;

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "round-trip-direct",
    });
}

impl bindings::RoundTripDirectImportsWithStore for Ctx {
    async fn foo<T>(_: &Accessor<T, Self>, s: String) -> String {
        crate::util::sleep(Duration::from_millis(10)).await;
        format!("{s} - entered host - exited host")
    }
}

impl bindings::RoundTripDirectImports for Ctx {}

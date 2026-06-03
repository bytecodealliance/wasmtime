use super::Ctx;
use crate::util::yield_times;
use wasmtime::component::Accessor;

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "round-trip-direct",
    });
}

impl<T> bindings::RoundTripDirectImportsWithStore<T> for Ctx {
    async fn foo(_: &Accessor<T, Self>, s: String) -> String {
        yield_times(5).await;
        format!("{s} - entered host - exited host")
    }
}

impl bindings::RoundTripDirectImports for Ctx {}

use super::Ctx;
use std::time::Duration;
use wasmtime::component::Accessor;

pub mod bindings {
    wasmtime::component::bindgen!({
        trappable_imports: true,
        path: "wit",
        world: "round-trip-direct",
        concurrent_imports: true,
        concurrent_exports: true,
        async: true,
    });
}

impl bindings::RoundTripDirectImportsConcurrent for Ctx {
    async fn foo<T>(_: &Accessor<T, Self>, s: String) -> wasmtime::Result<String> {
        crate::util::sleep(Duration::from_millis(10)).await;
        Ok(format!("{s} - entered host - exited host"))
    }
}

impl bindings::RoundTripDirectImports for Ctx {}

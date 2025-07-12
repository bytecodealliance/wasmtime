use super::Ctx;
use std::time::Duration;
use wasmtime::component::Accessor;

pub mod bindings {
    wasmtime::component::bindgen!({
        trappable_imports: true,
        path: "wit",
        world: "round-trip",
        concurrent_imports: true,
        concurrent_exports: true,
        async: true,
    });
}

pub mod non_concurrent_export_bindings {
    wasmtime::component::bindgen!({
        trappable_imports: true,
        path: "wit",
        world: "round-trip",
        concurrent_imports: true,
        async: true,
    });
}

impl bindings::local::local::baz::HostConcurrent for Ctx {
    async fn foo<T>(_: &mut Accessor<T, Self>, s: String) -> wasmtime::Result<String> {
        crate::util::sleep(Duration::from_millis(10)).await;
        Ok(format!("{s} - entered host - exited host"))
    }
}

impl bindings::local::local::baz::Host for Ctx {}

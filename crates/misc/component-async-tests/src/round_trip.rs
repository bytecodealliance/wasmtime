use super::Ctx;
use crate::util::yield_times;
use wasmtime::component::Accessor;

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "round-trip",
        imports: { default: trappable },
    });
}

pub mod non_concurrent_export_bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "round-trip",
        exports: { default: ignore_wit | async },
    });
}

impl<T> bindings::local::local::baz::HostWithStore<T> for Ctx {
    async fn foo(_: &Accessor<T, Self>, s: String) -> wasmtime::Result<String> {
        yield_times(10).await;
        Ok(format!("{s} - entered host - exited host"))
    }
}

impl bindings::local::local::baz::Host for Ctx {}

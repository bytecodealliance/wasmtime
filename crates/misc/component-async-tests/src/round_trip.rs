use super::Ctx;
use std::time::Duration;
use wasmtime::component::Accessor;

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "round-trip",
    });
}

pub mod non_concurrent_export_bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "round-trip",
        exports: { default: ignore_wit | async },
    });
}

impl bindings::local::local::baz::HostWithStore for Ctx {
    async fn foo<T>(_: &Accessor<T, Self>, s: String) -> String {
        crate::util::sleep(Duration::from_millis(10)).await;
        format!("{s} - entered host - exited host")
    }
}

impl bindings::local::local::baz::Host for Ctx {}

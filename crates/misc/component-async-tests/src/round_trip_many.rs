use super::Ctx;
use anyhow::Result;
use std::time::Duration;
use wasmtime::component::Accessor;

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "round-trip-many",
        additional_derives: [ Eq, PartialEq ],
    });
}

pub mod non_concurrent_export_bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "round-trip-many",
        additional_derives: [ Eq, PartialEq ],
        exports: { default: ignore_wit | async },
    });
}

use bindings::local::local::many::Stuff;

impl bindings::local::local::many::HostWithStore for Ctx {
    async fn foo<T>(
        _: &Accessor<T, Self>,
        a: String,
        b: u32,
        c: Vec<u8>,
        d: (u64, u64),
        e: Stuff,
        f: Option<Stuff>,
        g: Result<Stuff, ()>,
    ) -> (
        String,
        u32,
        Vec<u8>,
        (u64, u64),
        Stuff,
        Option<Stuff>,
        Result<Stuff, ()>,
    ) {
        crate::util::sleep(Duration::from_millis(10)).await;
        (
            format!("{a} - entered host - exited host"),
            b,
            c,
            d,
            e,
            f,
            g,
        )
    }
}

impl bindings::local::local::many::Host for Ctx {}

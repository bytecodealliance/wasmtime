//! Implementation of the `wasi:http/proxy` world.
//!
//! The implementation at the top of the module for use in async contexts,
//! while the `sync` module provides implementation for use in sync contexts.

use crate::WasiHttpView;

mod bindings {
    wasmtime::component::bindgen!({
        world: "wasi:http/proxy",
        tracing: true,
        async: true,
        with: {
            "wasi:http": crate::bindings::http,
            "wasi": wasmtime_wasi::bindings,
        },
    });
}

/// Raw bindings to the `wasi:http/proxy` exports.
pub use bindings::exports;

/// Bindings to the `wasi:http/proxy` world.
pub use bindings::Proxy;

/// Add support for the `wasi:http/proxy` world to a [`wasmtime::component::Linker`].
///
/// This should be used in async contexts. For sync contexts, use [`sync::add_to_linker`].
pub fn add_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView + wasmtime_wasi::WasiView,
{
    wasmtime_wasi::bindings::clocks::wall_clock::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::clocks::monotonic_clock::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::io::poll::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::io::error::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::io::streams::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::cli::stdin::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::cli::stdout::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::cli::stderr::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::random::random::add_to_linker(l, |t| t)?;

    add_only_http_to_linker(l)
}

#[doc(hidden)]
pub fn add_only_http_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView + wasmtime_wasi::WasiView + crate::bindings::http::types::Host,
{
    crate::bindings::http::outgoing_handler::add_to_linker(l, |t| t)?;
    crate::bindings::http::types::add_to_linker(l, |t| t)?;

    Ok(())
}

/// Sync implementation of the `wasi:http/proxy` world.
pub mod sync {
    use crate::WasiHttpView;

    wasmtime::component::bindgen!({
        world: "wasi:http/proxy",
        tracing: true,
        async: false,
        with: {
            "wasi:http": crate::bindings::http, // http is in this crate
            "wasi:io": wasmtime_wasi::bindings::sync, // io is sync
            "wasi": wasmtime_wasi::bindings, // everything else
        },
    });

    /// Add support for the `wasi:http/proxy` world to a [`wasmtime::component::Linker`].
    ///
    /// This should be used in sync contexts. For async contexts, use [`super::add_to_linker`].
    pub fn add_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
    where
        T: WasiHttpView + wasmtime_wasi::WasiView,
    {
        wasmtime_wasi::bindings::clocks::wall_clock::add_to_linker(l, |t| t)?;
        wasmtime_wasi::bindings::clocks::monotonic_clock::add_to_linker(l, |t| t)?;
        wasmtime_wasi::bindings::sync::io::poll::add_to_linker(l, |t| t)?;
        wasmtime_wasi::bindings::sync::io::streams::add_to_linker(l, |t| t)?;
        wasmtime_wasi::bindings::io::error::add_to_linker(l, |t| t)?;
        wasmtime_wasi::bindings::cli::stdin::add_to_linker(l, |t| t)?;
        wasmtime_wasi::bindings::cli::stdout::add_to_linker(l, |t| t)?;
        wasmtime_wasi::bindings::cli::stderr::add_to_linker(l, |t| t)?;
        wasmtime_wasi::bindings::random::random::add_to_linker(l, |t| t)?;

        add_only_http_to_linker(l)?;

        Ok(())
    }

    #[doc(hidden)]
    // TODO: This is temporary solution until the wasmtime_wasi command functions can be removed
    pub fn add_only_http_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
    where
        T: WasiHttpView + wasmtime_wasi::WasiView + crate::bindings::http::types::Host,
    {
        crate::bindings::http::outgoing_handler::add_to_linker(l, |t| t)?;
        crate::bindings::http::types::add_to_linker(l, |t| t)?;

        Ok(())
    }
}

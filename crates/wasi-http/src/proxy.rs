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

/// Add all of the `wasi:http/proxy` world's interfaces to a [`wasmtime::component::Linker`].
///
/// This function will add the `async` variant of all interfaces into the
/// `Linker` provided. By `async` this means that this function is only
/// compatible with [`Config::async_support(true)`][async]. For embeddings with
/// async support disabled see [`sync::add_to_linker`] instead.
///
/// [async]: wasmtime::Config::async_support
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{ResourceTable, Linker};
/// use wasmtime_wasi::{WasiCtx, WasiView, WasiCtxBuilder};
/// use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.async_support(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi_http::proxy::add_to_linker(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     let mut store = Store::new(
///         &engine,
///         MyState {
///             ctx: WasiCtxBuilder::new().build(),
///             http_ctx: WasiHttpCtx::new(),
///             table: ResourceTable::new(),
///         },
///     );
///
///     // use `linker.instantiate_async` to instantiate within `store`
///
///     Ok(())
/// }
///
/// struct MyState {
///     ctx: WasiCtx,
///     http_ctx: WasiHttpCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiHttpView for MyState {
///     fn ctx(&mut self) -> &mut WasiHttpCtx { &mut self.http_ctx }
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
/// ```
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

    mod bindings {
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
    }

    /// Raw bindings to the `wasi:http/proxy` exports.
    pub use bindings::exports;

    /// Bindings to the `wasi:http/proxy` world.
    pub use bindings::Proxy;

    /// Add all of the `wasi:http/proxy` world's interfaces to a [`wasmtime::component::Linker`].
    ///
    /// This function will add the `sync` variant of all interfaces into the
    /// `Linker` provided. For embeddings with async support see [`super::add_to_linker`] instead.
    ///
    /// # Example
    ///
    /// ```
    /// use wasmtime::{Engine, Result, Store, Config};
    /// use wasmtime::component::{ResourceTable, Linker};
    /// use wasmtime_wasi::{WasiCtx, WasiView, WasiCtxBuilder};
    /// use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};
    ///
    /// fn main() -> Result<()> {
    ///     let config = Config::default();
    ///     let engine = Engine::new(&config)?;
    ///
    ///     let mut linker = Linker::<MyState>::new(&engine);
    ///     wasmtime_wasi_http::proxy::sync::add_to_linker(&mut linker)?;
    ///     // ... add any further functionality to `linker` if desired ...
    ///
    ///     let mut store = Store::new(
    ///         &engine,
    ///         MyState {
    ///             ctx: WasiCtxBuilder::new().build(),
    ///             http_ctx: WasiHttpCtx::new(),
    ///             table: ResourceTable::new(),
    ///         },
    ///     );
    ///
    ///     // use `linker.instantiate` to instantiate within `store`
    ///
    ///     Ok(())
    /// }
    ///
    /// struct MyState {
    ///     ctx: WasiCtx,
    ///     http_ctx: WasiHttpCtx,
    ///     table: ResourceTable,
    /// }
    ///
    /// impl WasiHttpView for MyState {
    ///     fn ctx(&mut self) -> &mut WasiHttpCtx { &mut self.http_ctx }
    ///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
    /// }
    /// impl WasiView for MyState {
    ///     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
    ///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
    /// }
    /// ```
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

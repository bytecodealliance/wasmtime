//! Implementation of wasip2 version of `wasi:http` package
//!
//! # WASI HTTP Interfaces
//!
//! This module contains implementations of the following interfaces:
//!
//! * [`wasi:http/incoming-handler`]
//! * [`wasi:http/outgoing-handler`]
//! * [`wasi:http/types`]
//!
//! The module also contains an implementation of the [`wasi:http/proxy`] world.
//!
//! [`wasi:http/proxy`]: crate::p2::bindings::Proxy
//! [`wasi:http/outgoing-handler`]: crate::p2::bindings::http::outgoing_handler::Host
//! [`wasi:http/types`]: crate::p2::bindings::http::types::Host
//! [`wasi:http/incoming-handler`]: crate::p2::bindings::exports::wasi::http::incoming_handler::Guest
//!
//! This module is very similar to [`wasmtime-wasi`] in the it uses the
//! `bindgen!` macro in Wasmtime to generate bindings to interfaces. Bindings
//! are located in the [`bindings`] submodule.

mod http_impl;
mod types_impl;

pub mod bindings;

use crate::types::{WasiHttpImpl, WasiHttpView};
use crate::{type_annotate_http, type_annotate_io, type_annotate_wasi};
use wasmtime_wasi::IoImpl;
/// Add all of the `wasi:http/proxy` world's interfaces to a [`wasmtime::component::Linker`].
///
/// This function will add the `async` variant of all interfaces into the
/// `Linker` provided. By `async` this means that this function is only
/// compatible with [`Config::async_support(true)`][async]. For embeddings with
/// async support disabled see [`add_to_linker_sync`] instead.
///
/// [async]: wasmtime::Config::async_support
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Config};
/// use wasmtime::component::{ResourceTable, Linker};
/// use wasmtime_wasi::{IoView, WasiCtx, WasiView};
/// use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.async_support(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi_http::add_to_linker_async(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
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
/// impl IoView for MyState {
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
/// impl WasiHttpView for MyState {
///     fn ctx(&mut self) -> &mut WasiHttpCtx { &mut self.http_ctx }
/// }
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
/// }
/// ```
pub fn add_to_linker_async<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView + wasmtime_wasi::WasiView,
{
    let io_closure = type_annotate_io::<T, _>(|t| wasmtime_wasi::IoImpl(t));
    let closure = type_annotate_wasi::<T, _>(|t| wasmtime_wasi::WasiImpl(wasmtime_wasi::IoImpl(t)));
    wasmtime_wasi::bindings::clocks::wall_clock::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::clocks::monotonic_clock::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::io::poll::add_to_linker_get_host(l, io_closure)?;
    wasmtime_wasi::bindings::io::error::add_to_linker_get_host(l, io_closure)?;
    wasmtime_wasi::bindings::io::streams::add_to_linker_get_host(l, io_closure)?;
    wasmtime_wasi::bindings::cli::stdin::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stdout::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stderr::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::random::random::add_to_linker_get_host(l, closure)?;

    add_only_http_to_linker_async(l)
}

/// A slimmed down version of [`add_to_linker_async`] which only adds
/// `wasi:http` interfaces to the linker.
///
/// This is useful when using [`wasmtime_wasi::add_to_linker_async`] for
/// example to avoid re-adding the same interfaces twice.
pub fn add_only_http_to_linker_async<T>(
    l: &mut wasmtime::component::Linker<T>,
) -> anyhow::Result<()>
where
    T: WasiHttpView,
{
    let closure = type_annotate_http::<T, _>(|t| WasiHttpImpl(IoImpl(t)));
    self::bindings::http::outgoing_handler::add_to_linker_get_host(l, closure)?;
    self::bindings::http::types::add_to_linker_get_host(l, closure)?;

    Ok(())
}

/// Add all of the `wasi:http/proxy` world's interfaces to a [`wasmtime::component::Linker`].
///
/// This function will add the `sync` variant of all interfaces into the
/// `Linker` provided. For embeddings with async support see
/// [`add_to_linker_async`] instead.
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Config};
/// use wasmtime::component::{ResourceTable, Linker};
/// use wasmtime_wasi::{IoView, WasiCtx, WasiView};
/// use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};
///
/// fn main() -> Result<()> {
///     let config = Config::default();
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi_http::add_to_linker_sync(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     Ok(())
/// }
///
/// struct MyState {
///     ctx: WasiCtx,
///     http_ctx: WasiHttpCtx,
///     table: ResourceTable,
/// }
/// impl IoView for MyState {
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
/// impl WasiHttpView for MyState {
///     fn ctx(&mut self) -> &mut WasiHttpCtx { &mut self.http_ctx }
/// }
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
/// }
/// ```
pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView + wasmtime_wasi::WasiView,
{
    let io_closure = type_annotate_io::<T, _>(|t| wasmtime_wasi::IoImpl(t));
    let closure = type_annotate_wasi::<T, _>(|t| wasmtime_wasi::WasiImpl(wasmtime_wasi::IoImpl(t)));

    wasmtime_wasi::bindings::clocks::wall_clock::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::clocks::monotonic_clock::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::sync::io::poll::add_to_linker_get_host(l, io_closure)?;
    wasmtime_wasi::bindings::sync::io::streams::add_to_linker_get_host(l, io_closure)?;
    wasmtime_wasi::bindings::io::error::add_to_linker_get_host(l, io_closure)?;
    wasmtime_wasi::bindings::cli::stdin::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stdout::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stderr::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::random::random::add_to_linker_get_host(l, closure)?;

    add_only_http_to_linker_sync(l)?;

    Ok(())
}

/// A slimmed down version of [`add_to_linker_sync`] which only adds
/// `wasi:http` interfaces to the linker.
///
/// This is useful when using [`wasmtime_wasi::add_to_linker_sync`] for
/// example to avoid re-adding the same interfaces twice.
pub fn add_only_http_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView,
{
    let closure = type_annotate_http::<T, _>(|t| WasiHttpImpl(IoImpl(t)));

    self::bindings::http::outgoing_handler::add_to_linker_get_host(l, closure)?;
    self::bindings::http::types::add_to_linker_get_host(l, closure)?;

    Ok(())
}

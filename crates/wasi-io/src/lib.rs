//! # Wasmtime's wasi-io Implementation
//!
//! This crate provides a Wasmtime host implementation of the WASI 0.2 (aka
//! WASIp2 aka Preview 2) wasi-io package. The host implementation is
//! abstract: it is exposed as a set of traits which other crates provide
//! impls of.
//!
//! The wasi-io package is the foundation which defines how WASI programs
//! interact with the scheduler. It provides the `pollable`, `input-stream`,
//! and `output-stream` Component Model resources, which other packages
//! (including wasi-filesystem, wasi-sockets, wasi-cli, and wasi-http)
//! expose as the standard way to wait for readiness, and asynchronously read
//! and write to streams.
//!
//! This crate is designed to have no unnecessary dependencies and, in
//! particular, compile without `std`.

pub mod bindings;
mod impls;
pub mod poll;
pub mod streams;
mod view;

pub use view::{IoImpl, IoView};

#[doc(no_inline)]
pub use async_trait::async_trait;

/// Add the wasi-io host implementation from this crate into the `linker`
/// provided.
///
/// This function will add the `async` variant of all interfaces into the
/// [`Linker`] provided. By `async` this means that this function is only
/// compatible with [`Config::async_support(true)`][async]. For embeddings
/// with async support disabled, you'll need to use other crates, such as the
/// [`wasmtime-wasi`] crate, which provides an [`add_to_linker_sync`] that
/// includes an appropriate wasi-io implementation based on this crate's.
///
/// This function will add all interfaces implemented by this crate to the
/// [`Linker`], which corresponds to the `wasi:io/imports` world supported by
/// this crate.
///
/// [async]: wasmtime::Config::async_support
/// [`wasmtime-wasi`]: https://crates.io/crates/wasmtime-wasi
/// [`add_to_linker_sync`]: https://docs.rs/wasmtime-wasi/latest/wasmtime_wasi/fn.add_to_linker_sync.html
///
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{ResourceTable, Linker};
/// use wasmtime_wasi_io::{IoView;
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.async_support(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi_io::add_to_linker_async(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     let mut builder = WasiCtxBuilder::new();
///
///     // ... configure `builder` more to add env vars, args, etc ...
///
///     let mut store = Store::new(
///         &engine,
///         MyState {
///             table: ResourceTable::new(),
///         },
///     );
///
///     // ... use `linker` to instantiate within `store` ...
///
///     Ok(())
/// }
///
/// struct MyState {
///     table: ResourceTable,
/// }
///
/// impl IoView for MyState {
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
/// ```
pub fn add_to_linker_async<T: IoView>(
    l: &mut wasmtime::component::Linker<T>,
) -> wasmtime::Result<()> {
    let closure = io_type_annotate::<T, _>(|t| IoImpl(t));
    crate::bindings::wasi::io::error::add_to_linker_get_host(l, closure)?;
    crate::bindings::wasi::io::poll::add_to_linker_get_host(l, closure)?;
    crate::bindings::wasi::io::streams::add_to_linker_get_host(l, closure)?;
    Ok(())
}

// NB: workaround some rustc inference - a future refactoring may make this
// obsolete.
fn io_type_annotate<T: IoView, F>(val: F) -> F
where
    F: Fn(&mut T) -> IoImpl<&mut T>,
{
    val
}

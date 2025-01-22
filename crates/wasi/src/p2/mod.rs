//! Implementation of wasip2 version of WASI

use wasmtime::component::Linker;

use crate::{io_type_annotate, type_annotate, IoImpl, WasiImpl, WasiView};

pub mod bindings;
pub(crate) mod host;

/// Add all WASI interfaces from this module into the `linker` provided.
///
/// This function will add the `async` variant of all interfaces into the
/// [`Linker`] provided. By `async` this means that this function is only
/// compatible with [`Config::async_support(true)`][async]. For embeddings with
/// async support disabled see [`add_to_linker_sync`] instead.
///
/// This function will add all interfaces implemented by this crate to the
/// [`Linker`], which corresponds to the `wasi:cli/imports` world supported by
/// this crate.
///
/// [async]: wasmtime::Config::async_support
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{ResourceTable, Linker};
/// use wasmtime_wasi::{IoView, WasiCtx, WasiView, WasiCtxBuilder};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.async_support(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::add_to_linker_async(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     let mut builder = WasiCtxBuilder::new();
///
///     // ... configure `builder` more to add env vars, args, etc ...
///
///     let mut store = Store::new(
///         &engine,
///         MyState {
///             ctx: builder.build(),
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
///     ctx: WasiCtx,
///     table: ResourceTable,
/// }
///
/// impl IoView for MyState {
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
/// }
/// ```
pub fn add_to_linker_async<T: WasiView>(linker: &mut Linker<T>) -> anyhow::Result<()> {
    let options = crate::p2::bindings::LinkOptions::default();
    add_to_linker_with_options_async(linker, &options)
}

/// Similar to [`add_to_linker_async`], but with the ability to enable unstable features.
pub fn add_to_linker_with_options_async<T: WasiView>(
    linker: &mut Linker<T>,
    options: &crate::p2::bindings::LinkOptions,
) -> anyhow::Result<()> {
    let l = linker;
    let io_closure = io_type_annotate::<T, _>(|t| IoImpl(t));
    let closure = type_annotate::<T, _>(|t| WasiImpl(IoImpl(t)));

    crate::p2::bindings::clocks::wall_clock::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::clocks::monotonic_clock::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::filesystem::types::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::filesystem::preopens::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::io::error::add_to_linker_get_host(l, io_closure)?;
    crate::p2::bindings::io::poll::add_to_linker_get_host(l, io_closure)?;
    crate::p2::bindings::io::streams::add_to_linker_get_host(l, io_closure)?;
    crate::p2::bindings::random::random::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::random::insecure::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::random::insecure_seed::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::exit::add_to_linker_get_host(l, &options.into(), closure)?;
    crate::p2::bindings::cli::environment::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::stdin::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::stdout::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::stderr::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::terminal_input::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::terminal_output::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::terminal_stdin::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::terminal_stdout::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::terminal_stderr::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sockets::tcp::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sockets::tcp_create_socket::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sockets::udp::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sockets::udp_create_socket::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sockets::instance_network::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sockets::network::add_to_linker_get_host(l, &options.into(), closure)?;
    crate::p2::bindings::sockets::ip_name_lookup::add_to_linker_get_host(l, closure)?;
    Ok(())
}

/// Add all WASI interfaces from this crate into the `linker` provided.
///
/// This function will add the synchronous variant of all interfaces into the
/// [`Linker`] provided. By synchronous this means that this function is only
/// compatible with [`Config::async_support(false)`][async]. For embeddings
/// with async support enabled see [`add_to_linker_async`] instead.
///
/// This function will add all interfaces implemented by this crate to the
/// [`Linker`], which corresponds to the `wasi:cli/imports` world supported by
/// this crate.
///
/// [async]: wasmtime::Config::async_support
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{ResourceTable, Linker};
/// use wasmtime_wasi::{IoView, WasiCtx, WasiView, WasiCtxBuilder};
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::add_to_linker_sync(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     let mut builder = WasiCtxBuilder::new();
///
///     // ... configure `builder` more to add env vars, args, etc ...
///
///     let mut store = Store::new(
///         &engine,
///         MyState {
///             ctx: builder.build(),
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
///     ctx: WasiCtx,
///     table: ResourceTable,
/// }
/// impl IoView for MyState {
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
/// }
/// ```
pub fn add_to_linker_sync<T: WasiView>(
    linker: &mut wasmtime::component::Linker<T>,
) -> anyhow::Result<()> {
    let options = crate::p2::bindings::sync::LinkOptions::default();
    add_to_linker_with_options_sync(linker, &options)
}

/// Similar to [`add_to_linker_sync`], but with the ability to enable unstable features.
pub fn add_to_linker_with_options_sync<T: WasiView>(
    linker: &mut wasmtime::component::Linker<T>,
    options: &crate::p2::bindings::sync::LinkOptions,
) -> anyhow::Result<()> {
    let l = linker;
    let io_closure = io_type_annotate::<T, _>(|t| IoImpl(t));
    let closure = type_annotate::<T, _>(|t| WasiImpl(IoImpl(t)));

    crate::p2::bindings::clocks::wall_clock::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::clocks::monotonic_clock::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sync::filesystem::types::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::filesystem::preopens::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::io::error::add_to_linker_get_host(l, io_closure)?;
    crate::p2::bindings::sync::io::poll::add_to_linker_get_host(l, io_closure)?;
    crate::p2::bindings::sync::io::streams::add_to_linker_get_host(l, io_closure)?;
    crate::p2::bindings::random::random::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::random::insecure::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::random::insecure_seed::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::exit::add_to_linker_get_host(l, &options.into(), closure)?;
    crate::p2::bindings::cli::environment::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::stdin::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::stdout::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::stderr::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::terminal_input::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::terminal_output::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::terminal_stdin::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::terminal_stdout::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::cli::terminal_stderr::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sync::sockets::tcp::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sockets::tcp_create_socket::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sync::sockets::udp::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sockets::udp_create_socket::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sockets::instance_network::add_to_linker_get_host(l, closure)?;
    crate::p2::bindings::sockets::network::add_to_linker_get_host(l, &options.into(), closure)?;
    crate::p2::bindings::sockets::ip_name_lookup::add_to_linker_get_host(l, closure)?;
    Ok(())
}

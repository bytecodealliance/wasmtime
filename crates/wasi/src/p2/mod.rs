//! # Wasmtime's WASIp2 Implementation
//!
//!
//! This module provides a Wasmtime host implementation of WASI 0.2 (aka WASIp2
//! aka Preview 2) and WASI 0.1 (aka WASIp1 aka Preview 1). WASI is implemented
//! with the Rust crates [`tokio`] and [`cap-primitives`] primarily, meaning that
//! operations are implemented in terms of their native platform equivalents by
//! default.
//!
//! # WASIp2 interfaces
//!
//! This module contains implementations of the following interfaces:
//!
//! * [`wasi:cli/environment`]
//! * [`wasi:cli/exit`]
//! * [`wasi:cli/stderr`]
//! * [`wasi:cli/stdin`]
//! * [`wasi:cli/stdout`]
//! * [`wasi:cli/terminal-input`]
//! * [`wasi:cli/terminal-output`]
//! * [`wasi:cli/terminal-stderr`]
//! * [`wasi:cli/terminal-stdin`]
//! * [`wasi:cli/terminal-stdout`]
//! * [`wasi:clocks/monotonic-clock`]
//! * [`wasi:clocks/wall-clock`]
//! * [`wasi:filesystem/preopens`]
//! * [`wasi:filesystem/types`]
//! * [`wasi:random/insecure-seed`]
//! * [`wasi:random/insecure`]
//! * [`wasi:random/random`]
//! * [`wasi:sockets/instance-network`]
//! * [`wasi:sockets/ip-name-lookup`]
//! * [`wasi:sockets/network`]
//! * [`wasi:sockets/tcp-create-socket`]
//! * [`wasi:sockets/tcp`]
//! * [`wasi:sockets/udp-create-socket`]
//! * [`wasi:sockets/udp`]
//!
//! Most traits are implemented for [`WasiCtxView`] trait which provides
//! access to [`WasiCtx`] and [`ResourceTable`], which defines the configuration
//! for WASI and handle state. The [`WasiView`] trait is used to acquire and
//! construct a [`WasiCtxView`].
//!
//! The [`wasmtime-wasi-io`] crate contains implementations of the
//! following interfaces, and this module reuses those implementations:
//!
//! * [`wasi:io/error`]
//! * [`wasi:io/poll`]
//! * [`wasi:io/streams`]
//!
//! These traits are implemented directly for [`ResourceTable`]. All aspects of
//! `wasmtime-wasi-io` that are used by this module are re-exported. Unless you
//! are implementing other host functionality that needs to interact with the
//! WASI scheduler and don't want to use other functionality provided by
//! `wasmtime-wasi`, you don't need to take a direct dependency on
//! `wasmtime-wasi-io`.
//!
//! # Generated Bindings
//!
//! This module uses [`wasmtime::component::bindgen!`] to generate bindings for
//! all WASI interfaces. Raw bindings are available in the [`bindings`] submodule
//! of this module. Downstream users can either implement these traits themselves
//! or you can use the built-in implementations in this module for
//! `WasiImpl<T: WasiView>`.
//!
//! # The `WasiView` trait
//!
//! This module's implementation of WASI is done in terms of an implementation of
//! [`WasiView`]. This trait provides a "view" into WASI-related state that is
//! contained within a [`Store<T>`](wasmtime::Store).
//!
//! For all of the generated bindings in this module (Host traits),
//! implementations are provided looking like:
//!
//! ```
//! # use wasmtime_wasi::WasiCtxView;
//! # trait WasiView {}
//! # mod bindings { pub mod wasi { pub trait Host {} } }
//! impl bindings::wasi::Host for WasiCtxView<'_> {
//!     // ...
//! }
//! ```
//!
//! where the [`WasiCtxView`] type comes from [`WasiView::ctx`] for the type
//! contained within the `Store<T>`. The [`add_to_linker_sync`] and
//! [`add_to_linker_async`] function then require that `T: WasiView` with
//! [`Linker<T>`](wasmtime::component::Linker).
//!
//! To implement the [`WasiView`] trait you will first select a
//! `T` to put in `Store<T>` (typically, by defining your own struct).
//! Somewhere within `T` you'll store:
//!
//! * [`ResourceTable`] - created through default constructors.
//! * [`WasiCtx`] - created through [`WasiCtxBuilder`].
//!
//! You'll then write an implementation of the [`WasiView`]
//! trait to access those items in your `T`. For example:
//! ```
//! use wasmtime::component::ResourceTable;
//! use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
//!
//! struct MyCtx {
//!     table: ResourceTable,
//!     wasi: WasiCtx,
//! }
//!
//! impl WasiView for MyCtx {
//!     fn ctx(&mut self) -> WasiCtxView<'_> {
//!         WasiCtxView { ctx: &mut self.wasi, table: &mut self.table }
//!     }
//! }
//! ```
//!
//! # Async and Sync
//!
//! All WASIp2 functions are blocking from WebAssembly's point of view: a
//! WebAssembly call into these functions returns only when they are complete.
//!
//! This module provides an implementation of those functions in the host, where
//! for some functions, it is appropriate to implement them using async Rust and
//! the Tokio executor. The host implementation still blocks WebAssembly, but it
//! does not block the host's thread. Synchronous wrappers are also provided for
//! all async implementations, which create a private Tokio executor.
//!
//! Users can choose between these modes of implementation using variants
//! of the add_to_linker functions:
//!
//! * For non-async users, use [`add_to_linker_sync`].
//! * For async users, use [`add_to_linker_async`].
//!
//! Note that bindings are generated once for async and once for sync. Most
//! interfaces do not change, however, so only interfaces with blocking
//! functions have bindings generated twice. Bindings are organized as:
//!
//! * [`bindings`] - default location of all bindings, blocking functions are
//!   `async`
//! * [`bindings::sync`] - blocking interfaces have synchronous versions here.
//!
//! # Module-specific traits
//!
//! This module's default implementation of WASI bindings to native primitives
//! for the platform that it is compiled for. For example opening a TCP socket
//! uses the native platform to open a TCP socket (so long as [`WasiCtxBuilder`]
//! allows it). There are a few important traits, however, that are specific to
//! this module.
//!
//! * [`InputStream`] and [`OutputStream`] - these are the host traits
//!   behind the WASI `input-stream` and `output-stream` types in the
//!   `wasi:io/streams` interface. These enable embedders to build their own
//!   custom stream and insert them into a [`ResourceTable`] (as a boxed trait
//!   object, see [`DynInputStream`] and [`DynOutputStream`]) to be used from
//!   wasm.
//!
//! * [`Pollable`] - this trait enables building arbitrary logic to get hooked
//!   into a `pollable` resource from `wasi:io/poll`. A pollable resource is
//!   created through the [`subscribe`] function.
//!
//! * [`HostWallClock`](crate::HostWallClock) and [`HostMonotonicClock`](crate::HostMonotonicClock) are used in conjunction with
//!   [`WasiCtxBuilder::wall_clock`] and [`WasiCtxBuilder::monotonic_clock`] if
//!   the defaults host's clock should not be used.
//!
//! * [`StdinStream`] and [`StdoutStream`] are used to provide custom
//!   stdin/stdout streams if they're not inherited (or null, which is the
//!   default).
//!
//! These traits enable embedders to customize small portions of WASI interfaces
//! provided while still providing all other interfaces.
//!
//! # Examples
//!
//! Usage of this module is done through a few steps to get everything hooked up:
//!
//! 1. First implement [`WasiView`] for your type which is the
//!    `T` in `Store<T>`.
//! 2. Add WASI interfaces to a `wasmtime::component::Linker<T>`. This is either
//!    done through top-level functions like [`add_to_linker_sync`] or through
//!    individual `add_to_linker` functions in generated bindings throughout
//!    this module.
//! 3. Create a [`WasiCtx`] for each `Store<T>` through [`WasiCtxBuilder`]. Each
//!    WASI context is "null" or "empty" by default, so items must be explicitly
//!    added to get accessed by wasm (such as env vars or program arguments).
//! 4. Use the previous `Linker<T>` to instantiate a `Component` within a
//!    `Store<T>`.
//!
//! For examples see each of [`WasiView`], [`WasiCtx`], [`WasiCtxBuilder`],
//! [`add_to_linker_sync`], and [`bindings::Command`].
//!
//! [`wasmtime::component::bindgen!`]: https://docs.rs/wasmtime/latest/wasmtime/component/macro.bindgen.html
//! [`tokio`]: https://crates.io/crates/tokio
//! [`cap-primitives`]: https://crates.io/crates/cap-primitives
//! [`wasmtime-wasi-io`]: https://crates.io/crates/wasmtime-wasi-io
//! [`wasi:cli/environment`]: bindings::cli::environment::Host
//! [`wasi:cli/exit`]: bindings::cli::exit::Host
//! [`wasi:cli/stderr`]: bindings::cli::stderr::Host
//! [`wasi:cli/stdin`]: bindings::cli::stdin::Host
//! [`wasi:cli/stdout`]: bindings::cli::stdout::Host
//! [`wasi:cli/terminal-input`]: bindings::cli::terminal_input::Host
//! [`wasi:cli/terminal-output`]: bindings::cli::terminal_output::Host
//! [`wasi:cli/terminal-stdin`]: bindings::cli::terminal_stdin::Host
//! [`wasi:cli/terminal-stdout`]: bindings::cli::terminal_stdout::Host
//! [`wasi:cli/terminal-stderr`]: bindings::cli::terminal_stderr::Host
//! [`wasi:clocks/monotonic-clock`]: bindings::clocks::monotonic_clock::Host
//! [`wasi:clocks/wall-clock`]: bindings::clocks::wall_clock::Host
//! [`wasi:filesystem/preopens`]: bindings::filesystem::preopens::Host
//! [`wasi:filesystem/types`]: bindings::filesystem::types::Host
//! [`wasi:io/error`]: wasmtime_wasi_io::bindings::wasi::io::error::Host
//! [`wasi:io/poll`]: wasmtime_wasi_io::bindings::wasi::io::poll::Host
//! [`wasi:io/streams`]: wasmtime_wasi_io::bindings::wasi::io::streams::Host
//! [`wasi:random/insecure-seed`]: bindings::random::insecure_seed::Host
//! [`wasi:random/insecure`]: bindings::random::insecure::Host
//! [`wasi:random/random`]: bindings::random::random::Host
//! [`wasi:sockets/instance-network`]: bindings::sockets::instance_network::Host
//! [`wasi:sockets/ip-name-lookup`]: bindings::sockets::ip_name_lookup::Host
//! [`wasi:sockets/network`]: bindings::sockets::network::Host
//! [`wasi:sockets/tcp-create-socket`]: bindings::sockets::tcp_create_socket::Host
//! [`wasi:sockets/tcp`]: bindings::sockets::tcp::Host
//! [`wasi:sockets/udp-create-socket`]: bindings::sockets::udp_create_socket::Host
//! [`wasi:sockets/udp`]: bindings::sockets::udp::Host
//! [`ResourceTable`]: wasmtime::component::ResourceTable
//! [`WasiCtx`]: crate::WasiCtx
//! [`WasiCtxView`]: crate::WasiCtxView
//! [`WasiCtxBuilder`]: crate::WasiCtxBuilder
//! [`WasiCtxBuilder::wall_clock`]: crate::WasiCtxBuilder::wall_clock
//! [`WasiCtxBuilder::monotonic_clock`]: crate::WasiCtxBuilder::monotonic_clock
//! [`StdinStream`]: crate::cli::StdinStream
//! [`StdoutStream`]: crate::cli::StdoutStream

use crate::cli::{WasiCli, WasiCliNamed, WasiCliView as _};
use crate::clocks::{WasiClocks, WasiClocksNamed, WasiClocksView as _};
use crate::filesystem::{WasiFilesystem, WasiFilesystemNamed, WasiFilesystemView as _};
use crate::random::{WasiRandom, WasiRandomNamed};
use crate::sockets::{WasiSockets, WasiSocketsNamed, WasiSocketsView as _};
use crate::{NamedId, WasiCtxNamedView, WasiNamedView, WasiView};
use wasmtime::component::{Component, HasData, Linker, ResourceTable};

pub mod bindings;
pub(crate) mod filesystem;
mod host;
mod ip_name_lookup;
mod network;
pub mod pipe;
mod poll;
mod stdio;
mod tcp;
mod udp;
mod write_stream;

pub use self::filesystem::{FsError, FsResult, ReaddirIterator};
pub use self::network::{Network, SocketError, SocketResult};
pub use self::stdio::IsATTY;
pub use tcp::TcpSocket;
pub use udp::UdpSocket;
// These contents of wasmtime-wasi-io are re-exported by this module for compatibility:
// they were originally defined in this module before being factored out, and many
// users of this module depend on them at these names.
pub use wasmtime_wasi_io::poll::{DynFuture, DynPollable, MakeFuture, Pollable, subscribe};
pub use wasmtime_wasi_io::streams::{
    DynInputStream, DynOutputStream, Error as IoError, InputStream, OutputStream, StreamError,
    StreamResult,
};

/// Add all WASI interfaces from this crate into the `linker` provided.
///
/// This function will add the `async` variant of all interfaces into the
/// [`Linker`] provided. For embeddings with async support disabled see
/// [`add_to_linker_sync`] instead.
///
/// This function will add all interfaces implemented by this crate to the
/// [`Linker`], which corresponds to the `wasi:cli/imports` world supported by
/// this crate.
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{ResourceTable, Linker};
/// use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     let mut builder = WasiCtx::builder();
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
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> WasiCtxView<'_> {
///         WasiCtxView { ctx: &mut self.ctx, table: &mut self.table }
///     }
/// }
/// ```
pub fn add_to_linker_async<T: WasiView>(linker: &mut Linker<T>) -> wasmtime::Result<()> {
    let options = bindings::LinkOptions::default();
    add_to_linker_with_options_async(linker, &options)
}

/// Similar to [`add_to_linker_async`], but with the ability to enable unstable features.
pub fn add_to_linker_with_options_async<T: WasiView>(
    linker: &mut Linker<T>,
    options: &bindings::LinkOptions,
) -> wasmtime::Result<()> {
    add_async_io_to_linker(linker)?;
    add_nonblocking_to_linker(linker, options)?;

    let l = linker;
    bindings::filesystem::types::add_to_linker::<T, WasiFilesystem>(l, T::filesystem)?;
    bindings::sockets::tcp::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
    bindings::sockets::udp::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
    bindings::sockets::udp_create_socket::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
    bindings::sockets::ip_name_lookup::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
    Ok(())
}

/// Shared functionality for [`add_to_linker_async`] and [`add_to_linker_sync`].
fn add_nonblocking_to_linker<'a, T: WasiView, O>(
    linker: &mut Linker<T>,
    options: &'a O,
) -> wasmtime::Result<()>
where
    bindings::sockets::network::LinkOptions: From<&'a O>,
{
    use crate::p2::bindings::{cli, clocks, filesystem, random, sockets};

    let l = linker;
    clocks::wall_clock::add_to_linker::<T, WasiClocks>(l, T::clocks)?;
    clocks::monotonic_clock::add_to_linker::<T, WasiClocks>(l, T::clocks)?;
    filesystem::preopens::add_to_linker::<T, WasiFilesystem>(l, T::filesystem)?;
    random::random::add_to_linker::<T, WasiRandom>(l, |t| &mut t.ctx().ctx.random)?;
    random::insecure::add_to_linker::<T, WasiRandom>(l, |t| &mut t.ctx().ctx.random)?;
    random::insecure_seed::add_to_linker::<T, WasiRandom>(l, |t| &mut t.ctx().ctx.random)?;
    cli::exit::add_to_linker::<T, WasiCli>(l, T::cli)?;
    cli::environment::add_to_linker::<T, WasiCli>(l, T::cli)?;
    cli::stdin::add_to_linker::<T, WasiCli>(l, T::cli)?;
    cli::stdout::add_to_linker::<T, WasiCli>(l, T::cli)?;
    cli::stderr::add_to_linker::<T, WasiCli>(l, T::cli)?;
    cli::terminal_input::add_to_linker::<T, WasiCli>(l, T::cli)?;
    cli::terminal_output::add_to_linker::<T, WasiCli>(l, T::cli)?;
    cli::terminal_stdin::add_to_linker::<T, WasiCli>(l, T::cli)?;
    cli::terminal_stdout::add_to_linker::<T, WasiCli>(l, T::cli)?;
    cli::terminal_stderr::add_to_linker::<T, WasiCli>(l, T::cli)?;
    sockets::tcp_create_socket::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
    sockets::instance_network::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
    sockets::network::add_to_linker::<T, WasiSockets>(l, &options.into(), T::sockets)?;
    Ok(())
}

/// Same as [`add_to_linker_async`] except that this only adds interfaces
/// present in the `wasi:http/proxy` world.
pub fn add_to_linker_proxy_interfaces_async<T: WasiView>(
    linker: &mut Linker<T>,
) -> wasmtime::Result<()> {
    add_async_io_to_linker(linker)?;
    add_proxy_interfaces_nonblocking(linker)
}

/// Same as [`add_to_linker_sync`] except that this only adds interfaces
/// present in the `wasi:http/proxy` world.
#[doc(hidden)]
pub fn add_to_linker_proxy_interfaces_sync<T: WasiView>(
    linker: &mut Linker<T>,
) -> wasmtime::Result<()> {
    add_sync_wasi_io(linker)?;
    add_proxy_interfaces_nonblocking(linker)
}

fn add_proxy_interfaces_nonblocking<T: WasiView>(linker: &mut Linker<T>) -> wasmtime::Result<()> {
    use crate::p2::bindings::{cli, clocks, random};

    let l = linker;
    clocks::wall_clock::add_to_linker::<T, WasiClocks>(l, T::clocks)?;
    clocks::monotonic_clock::add_to_linker::<T, WasiClocks>(l, T::clocks)?;
    random::random::add_to_linker::<T, WasiRandom>(l, |t| &mut t.ctx().ctx.random)?;
    cli::stdin::add_to_linker::<T, WasiCli>(l, T::cli)?;
    cli::stdout::add_to_linker::<T, WasiCli>(l, T::cli)?;
    cli::stderr::add_to_linker::<T, WasiCli>(l, T::cli)?;
    Ok(())
}

/// Add all WASI interfaces from this crate into the `linker` provided.
///
/// This function will add the synchronous variant of all interfaces into the
/// [`Linker`] provided. For embeddings with async support enabled see
/// [`add_to_linker_async`] instead.
///
/// This function will add all interfaces implemented by this crate to the
/// [`Linker`], which corresponds to the `wasi:cli/imports` world supported by
/// this crate.
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{ResourceTable, Linker};
/// use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     let mut builder = WasiCtx::builder();
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
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> WasiCtxView<'_> {
///         WasiCtxView { ctx: &mut self.ctx, table: &mut self.table }
///     }
/// }
/// ```
pub fn add_to_linker_sync<T: WasiView>(
    linker: &mut wasmtime::component::Linker<T>,
) -> wasmtime::Result<()> {
    let options = bindings::sync::LinkOptions::default();
    add_to_linker_with_options_sync(linker, &options)
}

/// Similar to [`add_to_linker_sync`], but with the ability to enable unstable features.
pub fn add_to_linker_with_options_sync<T: WasiView>(
    linker: &mut wasmtime::component::Linker<T>,
    options: &bindings::sync::LinkOptions,
) -> wasmtime::Result<()> {
    add_nonblocking_to_linker(linker, options)?;
    add_sync_wasi_io(linker)?;

    let l = linker;
    bindings::sync::filesystem::types::add_to_linker::<T, WasiFilesystem>(l, T::filesystem)?;
    bindings::sync::sockets::tcp::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
    bindings::sync::sockets::udp::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
    bindings::sync::sockets::udp_create_socket::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
    bindings::sync::sockets::ip_name_lookup::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
    Ok(())
}

/// Shared functionality of [`add_to_linker_sync`]` and
/// [`add_to_linker_proxy_interfaces_sync`].
fn add_sync_wasi_io<T: WasiView>(
    linker: &mut wasmtime::component::Linker<T>,
) -> wasmtime::Result<()> {
    let l = linker;
    wasmtime_wasi_io::bindings::wasi::io::error::add_to_linker::<T, HasIo>(l, |t| t.ctx().table)?;
    bindings::sync::io::poll::add_to_linker::<T, HasIo>(l, |t| t.ctx().table)?;
    bindings::sync::io::streams::add_to_linker::<T, HasIo>(l, |t| t.ctx().table)?;
    Ok(())
}

struct HasIo;

impl HasData for HasIo {
    type Data<'a> = &'a mut ResourceTable;
}

// FIXME: it's a bit unfortunate that this can't use
// `wasmtime_wasi_io::add_to_linker` and that's because `T: WasiView`, here,
// not `T: IoView`. Ideally we'd have `impl<T: WasiView> IoView for T` but
// that's not possible with these two traits in separate crates. For now this
// is some small duplication but if this gets worse over time then we'll want
// to massage this.
fn add_async_io_to_linker<T: WasiView>(l: &mut Linker<T>) -> wasmtime::Result<()> {
    wasmtime_wasi_io::bindings::wasi::io::error::add_to_linker::<T, HasIo>(l, |t| t.ctx().table)?;
    wasmtime_wasi_io::bindings::wasi::io::poll::add_to_linker::<T, HasIo>(l, |t| t.ctx().table)?;
    wasmtime_wasi_io::bindings::wasi::io::streams::add_to_linker::<T, HasIo>(l, |t| t.ctx().table)?;
    Ok(())
}

/// Interfaces that are added via [`add_named_to_linker_async`].
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Interface {
    /// `wasi:clocks/monotonic-clock`
    ClocksMonotonicClock,
    /// `wasi:clocks/wall-clock`
    ClocksWallClock,
    /// `wasi:random/random`
    RandomRandom,
    /// `wasi:random/insecure`
    RandomInsecure,
    /// `wasi:random/insecure-seed`
    RandomInsecureSeed,
    /// `wasi:cli/exit`
    CliExit,
    /// `wasi:cli/environment`
    CliEnvironment,
    /// `wasi:cli/stdin`
    CliStdin,
    /// `wasi:cli/stdout`
    CliStdout,
    /// `wasi:cli/stderr`
    CliStderr,
    /// `wasi:cli/terminal-input`
    CliTerminalInput,
    /// `wasi:cli/terminal-output`
    CliTerminalOutput,
    /// `wasi:cli/terminal-stdin`
    CliTerminalStdin,
    /// `wasi:cli/terminal-stdout`
    CliTerminalStdout,
    /// `wasi:cli/terminal-stderr`
    CliTerminalStderr,
    /// `wasi:filesystem/types`
    FilesystemTypes,
    /// `wasi:filesystem/preopens`
    FilesystemPreopens,
    /// `wasi:sockets/instance-network`
    SocketsInstanceNetwork,
    /// `wasi:sockets/network`
    SocketsNetwork,
    /// `wasi:sockets/ip-name-lookup`
    SocketsIpNameLookup,
    /// `wasi:sockets/tcp-create-socket`
    SocketsTcpCreateSocket,
    /// `wasi:sockets/tcp`
    SocketsTcp,
    /// `wasi:sockets/udp-create-socket`
    SocketsUdpCreateSocket,
    /// `wasi:sockets/udp`
    SocketsUdp,
}

/// Add all WASI interfaces from this crate into the `linker` provided for any
/// named imports that a component has.
///
/// This function is similar to [`add_to_linker_async`] except that it's specifically
/// designed to work with named imports of WASI interfaces that components may
/// have. This requires a [`Component`] parameter to be passed in when
/// populating the [`Linker`] provided to see what the [`Component`] actually
/// imports.
///
/// Like [`add_to_linker_async`] this adds the `async` variant of all
/// interfaces. If this isn't low level enough you can invoke the
/// bindgen-generated `add_to_linker` functions within the [`named_imports`]
/// module directly instead.
///
/// [`named_imports`]: crate::p2::bindings::named_imports
///
/// The `lookup` function provided here is invoked for every named import found
/// for a particular interface. The [`Interface`] given is what's being bound,
/// and the `&str` argument is the name that the component imports it as. The
/// embedder can then decide how it would like to allocate a [`NamedId`] for
/// this import. If `Ok` is returned then the linker is populated with this
/// name, and imported functions will pass the [`NamedId`] later to the
/// implementation of [`WasiNamedView`] on `T` when invoked. If `Err` is
/// returned then the error will cause this entire function to fail and this
/// function call will return the same error.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use wasmtime::component::{Component, Linker, ResourceTable};
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime_wasi::{NamedId, WasiCtx, WasiCtxView, WasiNamedView};
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let component = Component::new(&engine, "(component)")?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///
///     // ... add default functionality to `linker` as needed ...
///
///     // and then additionally fill in any specific named imports `component`
///     // might have for WASI interfaces.
///     let mut name_map = HashMap::new();
///     wasmtime_wasi::p2::add_named_to_linker_async(&mut linker, &component, |_i, name| {
///         let len = name_map.len();
///         Ok(NamedId(*name_map.entry(name.to_string()).or_insert(len)))
///     })?;
///
///     // Here a `WasiCtx` is allocated per-named-import and will then be
///     // referred to internally by the [`NamedId`] allocated above. You could
///     // also use `name_map` to configure each context differently.
///     let mut my_state = MyState::default();
///     for _ in 0..name_map.len() {
///         my_state.contexts.push(WasiCtx::default());
///     }
///     let mut store = Store::new(&engine, my_state);
///
///     // ... use `linker` to instantiate within `store` ...
///
///     Ok(())
/// }
///
/// #[derive(Default)]
/// struct MyState {
///     table: ResourceTable,
///     contexts: Vec<WasiCtx>,
/// }
///
/// impl WasiNamedView for MyState {
///     fn ctx(&mut self, id: NamedId) -> WasiCtxView<'_> {
///         WasiCtxView {
///             ctx: &mut self.contexts[id.0],
///             table: &mut self.table,
///         }
///     }
/// }
/// ```
pub fn add_named_to_linker_async<T>(
    linker: &mut Linker<T>,
    component: &Component,
    lookup: impl FnMut(Interface, &str) -> wasmtime::Result<NamedId>,
) -> wasmtime::Result<()>
where
    T: WasiNamedView,
{
    let options = bindings::LinkOptions::default();
    add_named_to_linker_with_options_async(linker, &options, component, lookup)
}

/// Same as [`add_named_to_linker_async`] except [`bindings::LinkOptions`]
/// can be specified to configure interfaces that are added.
pub fn add_named_to_linker_with_options_async<T>(
    linker: &mut Linker<T>,
    options: &bindings::LinkOptions,
    component: &Component,
    mut lookup: impl FnMut(Interface, &str) -> wasmtime::Result<NamedId>,
) -> wasmtime::Result<()>
where
    T: WasiNamedView,
{
    use crate::p2::bindings::named_imports::wasi::{cli, clocks, filesystem, random, sockets};

    let l = linker;
    clocks::wall_clock::add_to_linker::<T, WasiClocksNamed<T>>(
        l,
        component,
        |name| lookup(Interface::ClocksWallClock, name),
        |x| WasiCtxNamedView(x),
    )?;
    clocks::monotonic_clock::add_to_linker::<T, WasiClocksNamed<T>>(
        l,
        component,
        |name| lookup(Interface::ClocksMonotonicClock, name),
        |x| WasiCtxNamedView(x),
    )?;
    filesystem::types::add_to_linker::<T, WasiFilesystemNamed<T>>(
        l,
        component,
        |name| lookup(Interface::FilesystemTypes, name),
        |x| WasiCtxNamedView(x),
    )?;
    filesystem::preopens::add_to_linker::<T, WasiFilesystemNamed<T>>(
        l,
        component,
        |name| lookup(Interface::FilesystemPreopens, name),
        |x| WasiCtxNamedView(x),
    )?;
    random::random::add_to_linker::<T, WasiRandomNamed<T>>(
        l,
        component,
        |name| lookup(Interface::RandomRandom, name),
        |x| WasiCtxNamedView(x),
    )?;
    random::insecure::add_to_linker::<T, WasiRandomNamed<T>>(
        l,
        component,
        |name| lookup(Interface::RandomInsecure, name),
        |x| WasiCtxNamedView(x),
    )?;
    random::insecure_seed::add_to_linker::<T, WasiRandomNamed<T>>(
        l,
        component,
        |name| lookup(Interface::RandomInsecureSeed, name),
        |x| WasiCtxNamedView(x),
    )?;
    cli::exit::add_to_linker::<T, WasiCliNamed<T>>(
        l,
        component,
        |name| lookup(Interface::CliExit, name),
        |x| WasiCtxNamedView(x),
    )?;
    cli::environment::add_to_linker::<T, WasiCliNamed<T>>(
        l,
        component,
        |name| lookup(Interface::CliEnvironment, name),
        |x| WasiCtxNamedView(x),
    )?;
    cli::stdin::add_to_linker::<T, WasiCliNamed<T>>(
        l,
        component,
        |name| lookup(Interface::CliStdin, name),
        |x| WasiCtxNamedView(x),
    )?;
    cli::stdout::add_to_linker::<T, WasiCliNamed<T>>(
        l,
        component,
        |name| lookup(Interface::CliStdout, name),
        |x| WasiCtxNamedView(x),
    )?;
    cli::stderr::add_to_linker::<T, WasiCliNamed<T>>(
        l,
        component,
        |name| lookup(Interface::CliStderr, name),
        |x| WasiCtxNamedView(x),
    )?;
    cli::terminal_input::add_to_linker::<T, WasiCliNamed<T>>(
        l,
        component,
        |name| lookup(Interface::CliTerminalInput, name),
        |x| WasiCtxNamedView(x),
    )?;
    cli::terminal_output::add_to_linker::<T, WasiCliNamed<T>>(
        l,
        component,
        |name| lookup(Interface::CliTerminalOutput, name),
        |x| WasiCtxNamedView(x),
    )?;
    cli::terminal_stdin::add_to_linker::<T, WasiCliNamed<T>>(
        l,
        component,
        |name| lookup(Interface::CliTerminalStdin, name),
        |x| WasiCtxNamedView(x),
    )?;
    cli::terminal_stdout::add_to_linker::<T, WasiCliNamed<T>>(
        l,
        component,
        |name| lookup(Interface::CliTerminalStdout, name),
        |x| WasiCtxNamedView(x),
    )?;
    cli::terminal_stderr::add_to_linker::<T, WasiCliNamed<T>>(
        l,
        component,
        |name| lookup(Interface::CliTerminalStderr, name),
        |x| WasiCtxNamedView(x),
    )?;
    sockets::instance_network::add_to_linker::<T, WasiSocketsNamed<T>>(
        l,
        component,
        |name| lookup(Interface::SocketsInstanceNetwork, name),
        |x| WasiCtxNamedView(x),
    )?;
    sockets::network::add_to_linker::<T, WasiSocketsNamed<T>>(
        l,
        component,
        |name| lookup(Interface::SocketsNetwork, name),
        &options.into(),
        |x| WasiCtxNamedView(x),
    )?;
    sockets::ip_name_lookup::add_to_linker::<T, WasiSocketsNamed<T>>(
        l,
        component,
        |name| lookup(Interface::SocketsIpNameLookup, name),
        |x| WasiCtxNamedView(x),
    )?;
    sockets::tcp_create_socket::add_to_linker::<T, WasiSocketsNamed<T>>(
        l,
        component,
        |name| lookup(Interface::SocketsTcpCreateSocket, name),
        |x| WasiCtxNamedView(x),
    )?;
    sockets::tcp::add_to_linker::<T, WasiSocketsNamed<T>>(
        l,
        component,
        |name| lookup(Interface::SocketsTcp, name),
        |x| WasiCtxNamedView(x),
    )?;
    sockets::udp_create_socket::add_to_linker::<T, WasiSocketsNamed<T>>(
        l,
        component,
        |name| lookup(Interface::SocketsUdpCreateSocket, name),
        |x| WasiCtxNamedView(x),
    )?;
    sockets::udp::add_to_linker::<T, WasiSocketsNamed<T>>(
        l,
        component,
        |name| lookup(Interface::SocketsUdp, name),
        |x| WasiCtxNamedView(x),
    )?;
    Ok(())
}

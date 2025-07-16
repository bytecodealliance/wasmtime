//! Auto-generated bindings for WASI interfaces.
//!
//! This module contains the output of the [`bindgen!`] macro when run over
//! the `wasi:cli/imports` world.
//!
//! [`bindgen!`]: https://docs.rs/wasmtime/latest/wasmtime/component/macro.bindgen.html
//!
//! # Examples
//!
//! If you have a WIT world which refers to WASI interfaces you probably want to
//! use this modules's bindings rather than generate fresh bindings. That can be
//! done using the `with` option to [`bindgen!`]:
//!
//! ```rust
//! use wasmtime_wasi::p3::{WasiCtx, WasiView};
//! use wasmtime::{Result, Engine, Config};
//! use wasmtime::component::{Linker, HasSelf};
//!
//! wasmtime::component::bindgen!({
//!     inline: "
//!         package example:wasi;
//!
//!         // An example of extending the `wasi:cli/command` world with a
//!         // custom host interface.
//!         world my-world {
//!             include wasi:cli/command@0.3.0;
//!
//!             import custom-host;
//!         }
//!
//!         interface custom-host {
//!             my-custom-function: func();
//!         }
//!     ",
//!     path: "src/p3/wit",
//!     with: {
//!         "wasi": wasmtime_wasi::p3::bindings,
//!     },
//!     concurrent_exports: true,
//!     concurrent_imports: true,
//!     async: {
//!         only_imports: [
//!             "wasi:cli/stdin@0.3.0#get-stdin",
//!             "wasi:cli/stdout@0.3.0#set-stdout",
//!             "wasi:cli/stderr@0.3.0#set-stderr",
//!             "wasi:clocks/monotonic-clock@0.3.0#[async]wait-for",
//!             "wasi:clocks/monotonic-clock@0.3.0#[async]wait-until",
//!             "wasi:filesystem/types@0.3.0#[method]descriptor.read-via-stream",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.write-via-stream",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.append-via-stream",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.advise",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.sync-data",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.get-flags",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.get-type",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.set-size",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.set-times",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.read-directory",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.sync",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.create-directory-at",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.stat",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.stat-at",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.set-times-at",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.link-at",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.open-at",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.readlink-at",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.remove-directory-at",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.rename-at",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.symlink-at",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.unlink-file-at",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.is-same-object",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.metadata-hash",
//!             "wasi:filesystem/types@0.3.0#[async method]descriptor.metadata-hash-at",
//!             "wasi:sockets/ip-name-lookup@0.3.0#[async]resolve-addresses",
//!             "wasi:sockets/types@0.3.0#[async method]tcp-socket.connect",
//!             "wasi:sockets/types@0.3.0#[async method]tcp-socket.send",
//!             "wasi:sockets/types@0.3.0#[async method]udp-socket.receive",
//!             "wasi:sockets/types@0.3.0#[async method]udp-socket.send",
//!             "wasi:sockets/types@0.3.0#[method]tcp-socket.bind",
//!             "wasi:sockets/types@0.3.0#[method]tcp-socket.listen",
//!             "wasi:sockets/types@0.3.0#[method]tcp-socket.receive",
//!             "wasi:sockets/types@0.3.0#[method]udp-socket.bind",
//!             "wasi:sockets/types@0.3.0#[method]udp-socket.connect",
//!         ],
//!     },
//! });
//!
//! struct MyState {
//!     ctx: WasiCtx,
//! }
//!
//! impl example::wasi::custom_host::Host for MyState {
//!     fn my_custom_function(&mut self) {
//!         // ..
//!     }
//! }
//!
//! impl WasiView for MyState {
//!     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
//! }
//!
//! fn main() -> Result<()> {
//!     let mut config = Config::default();
//!     config.async_support(true);
//!     config.wasm_component_model_async(true);
//!     let engine = Engine::new(&config)?;
//!     let mut linker: Linker<MyState> = Linker::new(&engine);
//!     wasmtime_wasi::p3::add_to_linker(&mut linker)?;
//!     example::wasi::custom_host::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;
//!
//!     // .. use `Linker` to instantiate component ...
//!
//!     Ok(())
//! }
//! ```

mod generated {
    wasmtime::component::bindgen!({
        path: "src/p3/wit",
        world: "wasi:cli/command",
        tracing: true,
        trappable_imports: true,
        concurrent_exports: true,
        concurrent_imports: true,
        async: {
            only_imports: [
                "wasi:cli/stdin@0.3.0#get-stdin",
                "wasi:cli/stdout@0.3.0#set-stdout",
                "wasi:cli/stderr@0.3.0#set-stderr",
                "wasi:clocks/monotonic-clock@0.3.0#[async]wait-for",
                "wasi:clocks/monotonic-clock@0.3.0#[async]wait-until",
                "wasi:filesystem/types@0.3.0#[method]descriptor.read-via-stream",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.write-via-stream",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.append-via-stream",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.advise",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.sync-data",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.get-flags",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.get-type",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.set-size",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.set-times",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.read-directory",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.sync",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.create-directory-at",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.stat",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.stat-at",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.set-times-at",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.link-at",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.open-at",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.readlink-at",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.remove-directory-at",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.rename-at",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.symlink-at",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.unlink-file-at",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.is-same-object",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.metadata-hash",
                "wasi:filesystem/types@0.3.0#[async method]descriptor.metadata-hash-at",
                "wasi:sockets/ip-name-lookup@0.3.0#[async]resolve-addresses",
                "wasi:sockets/types@0.3.0#[async method]tcp-socket.connect",
                "wasi:sockets/types@0.3.0#[async method]tcp-socket.send",
                "wasi:sockets/types@0.3.0#[async method]udp-socket.receive",
                "wasi:sockets/types@0.3.0#[async method]udp-socket.send",
                "wasi:sockets/types@0.3.0#[method]tcp-socket.bind",
                "wasi:sockets/types@0.3.0#[method]tcp-socket.listen",
                "wasi:sockets/types@0.3.0#[method]tcp-socket.receive",
                "wasi:sockets/types@0.3.0#[method]udp-socket.bind",
                "wasi:sockets/types@0.3.0#[method]udp-socket.connect",
            ],
        },
    });
}
pub use self::generated::LinkOptions;
pub use self::generated::exports;
pub use self::generated::wasi::*;

/// Bindings to execute and run a `wasi:cli/command`.
///
/// This structure is automatically generated by `bindgen!`.
///
/// This can be used for a more "typed" view of executing a command component
/// through the [`Command::wasi_cli_run`] method plus
/// [`Guest::call_run`](exports::wasi::cli::run::Guest::call_run).
///
/// # Examples
///
/// ```no_run
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{Component, Linker};
/// use wasmtime_wasi::p3::{WasiCtx, WasiView, WasiCtxBuilder};
/// use wasmtime_wasi::p3::bindings::Command;
///
/// // This example is an example shim of executing a component based on the
/// // command line arguments provided to this program.
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let args = std::env::args().skip(1).collect::<Vec<_>>();
///
///     // Configure and create `Engine`
///     let mut config = Config::new();
///     config.async_support(true);
///     config.wasm_component_model_async(true);
///     let engine = Engine::new(&config)?;
///
///     // Configure a `Linker` with WASI, compile a component based on
///     // command line arguments, and then pre-instantiate it.
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::p3::add_to_linker(&mut linker)?;
///     let component = Component::from_file(&engine, &args[0])?;
///
///
///     // Configure a `WasiCtx` based on this program's environment. Then
///     // build a `Store` to instantiate into.
///     let mut builder = WasiCtxBuilder::new();
///     builder.inherit_stdio().inherit_env().args(&args);
///     let mut store = Store::new(
///         &engine,
///         MyState {
///             ctx: builder.build(),
///         },
///     );
///
///     // Instantiate the component and we're off to the races.
///     let instance = linker.instantiate_async(&mut store, &component).await?;
///     let command = Command::new(&mut store, &instance)?;
///     let program_result = instance.run_with(&mut store, async move |store| {
///         command.wasi_cli_run().call_run(store).await
///     }).await??;
///     match program_result {
///         Ok(()) => Ok(()),
///         Err(()) => std::process::exit(1),
///     }
/// }
///
/// struct MyState {
///     ctx: WasiCtx,
/// }
///
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
/// }
/// ```
///
/// ---
pub use self::generated::Command;

/// Pre-instantiated analog of [`Command`]
///
/// This can be used to front-load work such as export lookup before
/// instantiation.
///
/// # Examples
///
/// ```no_run
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{Linker, Component};
/// use wasmtime_wasi::p3::{WasiCtx, WasiView, WasiCtxBuilder};
/// use wasmtime_wasi::p3::bindings::CommandPre;
///
/// // This example is an example shim of executing a component based on the
/// // command line arguments provided to this program.
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let args = std::env::args().skip(1).collect::<Vec<_>>();
///
///     // Configure and create `Engine`
///     let mut config = Config::new();
///     config.async_support(true);
///     config.wasm_component_model_async(true);
///     let engine = Engine::new(&config)?;
///
///     // Configure a `Linker` with WASI, compile a component based on
///     // command line arguments, and then pre-instantiate it.
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::p3::add_to_linker(&mut linker)?;
///     let component = Component::from_file(&engine, &args[0])?;
///     let pre = CommandPre::new(linker.instantiate_pre(&component)?)?;
///
///
///     // Configure a `WasiCtx` based on this program's environment. Then
///     // build a `Store` to instantiate into.
///     let mut builder = WasiCtxBuilder::new();
///     builder.inherit_stdio().inherit_env().args(&args);
///     let mut store = Store::new(
///         &engine,
///         MyState {
///             ctx: builder.build(),
///         },
///     );
///
///     // Instantiate the component and we're off to the races.
///     let command = pre.instantiate_async(&mut store).await?;
///     // TODO: Construct an accessor from `store` to call `run`
///     // https://github.com/bytecodealliance/wasmtime/issues/11249
///     //let program_result = command.wasi_cli_run().call_run(&mut store).await?;
///     let program_result = todo!();
///     match program_result {
///         Ok(()) => Ok(()),
///         Err(()) => std::process::exit(1),
///     }
/// }
///
/// struct MyState {
///     ctx: WasiCtx,
/// }
///
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
/// }
/// ```
///
/// ---
// TODO: Make this public, once `CommandPre` can be used for
// calling exports
// https://github.com/bytecodealliance/wasmtime/issues/11249
#[doc(hidden)]
pub use self::generated::CommandPre;

pub use self::generated::CommandIndices;

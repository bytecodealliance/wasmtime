//! Auto-generated bindings for WASI interfaces.
//!
//! This module contains the output of the [`bindgen!`] macro when run over
//! the `wasi:cli/command` world. That means this module has all the generated
//! types for WASI for all of its base interfaces used by the CLI world. This
//! module itself by default contains bindings for `async`-related traits. The
//! [`sync`] module contains bindings for a non-`async` version of types.
//!
//! [`bindgen!`]: https://docs.rs/wasmtime/latest/wasmtime/component/macro.bindgen.html
//!
//! # Examples
//!
//! If you have a WIT world which refers to WASI interfaces you probably want to
//! use this crate's bindings rather than generate fresh bindings. That can be
//! done using the `with` option to [`bindgen!`]:
//!
//! ```rust
//! use wasmtime_wasi::{IoView, WasiCtx, ResourceTable, WasiView};
//! use wasmtime::{Result, Engine, Config};
//! use wasmtime::component::Linker;
//!
//! wasmtime::component::bindgen!({
//!     world: "example:wasi/my-world",
//!     inline: "
//!         package example:wasi;
//!
//!         // An example of extending the `wasi:cli/command` world with a
//!         // custom host interface.
//!         world my-world {
//!             include wasi:clocks/imports@0.3.0;
//!             include wasi:random/imports@0.3.0;
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
//!     async: true,
//! });
//!
//! struct MyState {
//!     table: ResourceTable,
//!     ctx: WasiCtx,
//! }
//!
//! impl example::wasi::custom_host::Host for MyState {
//!     async fn my_custom_function(&mut self) {
//!         // ..
//!     }
//! }
//!
//! impl IoView for MyState {
//!     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
//! }
//! impl WasiView for MyState {
//!     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
//! }
//!
//! fn main() -> Result<()> {
//!     let mut config = Config::default();
//!     config.async_support(true);
//!     let engine = Engine::new(&config)?;
//!     let mut linker: Linker<MyState> = Linker::new(&engine);
//!     wasmtime_wasi::add_to_linker_async(&mut linker)?;
//!     example::wasi::custom_host::add_to_linker(&mut linker, |state| state)?;
//!
//!     // .. use `Linker` to instantiate component ...
//!
//!     Ok(())
//! }
//! ```

/// Synchronous-generated bindings for WASI interfaces.
///
/// This is the same as the top-level [`bindings`](crate::p3::bindings) module of
/// this crate except that it's for synchronous calls.
///
/// # Examples
///
/// If you have a WIT world which refers to WASI interfaces you probably want to
/// use this crate's bindings rather than generate fresh bindings. That can be
/// done using the `with` option to `bindgen!`:
///
/// ```rust
/// use wasmtime_wasi::{IoView, WasiCtx, ResourceTable, WasiView};
/// use wasmtime::{Result, Engine};
/// use wasmtime::component::Linker;
///
/// wasmtime::component::bindgen!({
///     world: "example:wasi/my-world",
///     inline: "
///         package example:wasi;
///
///         // An example of extending the `wasi:cli/command` world with a
///         // custom host interface.
///         world my-world {
///             include wasi:clocks/imports@0.3.0;
///             include wasi:random/imports@0.3.0;
///
///             import custom-host;
///         }
///
///         interface custom-host {
///             my-custom-function: func();
///         }
///     ",
///     path: "src/p3/wit",
///     with: {
///         "wasi": wasmtime_wasi::p3::bindings::sync,
///     },
///     // This is required for bindings using `wasmtime-wasi` and it otherwise
///     // isn't the default for non-async bindings.
///     require_store_data_send: true,
/// });
///
/// struct MyState {
///     table: ResourceTable,
///     ctx: WasiCtx,
/// }
///
/// impl example::wasi::custom_host::Host for MyState {
///     fn my_custom_function(&mut self) {
///         // ..
///     }
/// }
///
/// impl IoView for MyState {
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let mut linker: Linker<MyState> = Linker::new(&engine);
///     wasmtime_wasi::add_to_linker_sync(&mut linker)?;
///     example::wasi::custom_host::add_to_linker(&mut linker, |state| state)?;
///
///     // .. use `Linker` to instantiate component ...
///
///     Ok(())
/// }
/// ```
pub mod sync {
    mod generated {
        wasmtime::component::bindgen!({
            path: "src/p3/wit",
            // TODO: Use `command` once 0.3.0 released
            //world: "wasi:cli/command",
            world: "inline:wasi/command",
            inline: "
                package inline:wasi;

                world command {
                    include wasi:clocks/imports@0.3.0;
                    include wasi:random/imports@0.3.0;
                }
            ",
            tracing: true,
            trappable_imports: true,
            with: {
                // These interfaces come from the outer module, as it's
                // sync/async agnostic.
                "wasi:random": crate::p3::bindings::random,
                "wasi:clocks/wall-clock": crate::p3::bindings::clocks::wall_clock,
            },
            require_store_data_send: true,
        });
    }
    pub use self::generated::wasi::*;

    /// Synchronous bindings to execute and run a `wasi:cli/command`.
    ///
    /// This structure is automatically generated by `bindgen!` and is intended
    /// to be used with [`Config::async_support(false)`][async]. For the
    /// asynchronous version see [`bindings::Command`](super::Command).
    ///
    /// This can be used for a more "typed" view of executing a command
    /// component through the [`Command::wasi_cli_run`] method plus
    /// [`Guest::call_run`](exports::wasi::cli::run::Guest::call_run).
    ///
    /// [async]: wasmtime::Config::async_support
    /// [`wasmtime_wasi::add_to_linker_sync`]: crate::add_to_linker_sync
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasmtime::{Engine, Result, Store, Config};
    /// use wasmtime::component::{ResourceTable, Linker, Component};
    /// use wasmtime_wasi::{IoView, WasiCtx, WasiView, WasiCtxBuilder};
    /// use wasmtime_wasi::bindings::sync::Command;
    ///
    /// // This example is an example shim of executing a component based on the
    /// // command line arguments provided to this program.
    /// fn main() -> Result<()> {
    ///     let args = std::env::args().skip(1).collect::<Vec<_>>();
    ///
    ///     // Configure and create `Engine`
    ///     let engine = Engine::default();
    ///
    ///     // Configure a `Linker` with WASI, compile a component based on
    ///     // command line arguments.
    ///     let mut linker = Linker::<MyState>::new(&engine);
    ///     wasmtime_wasi::add_to_linker_sync(&mut linker)?;
    ///     let component = Component::from_file(&engine, &args[0])?;
    ///
    ///
    ///     // Configure a `WasiCtx` based on this program's environment. Then
    ///     // build a `Store` to instantiate into.
    ///     let mut builder = WasiCtxBuilder::new();
    ///     builder.inherit_stdio().inherit_env().args(&args[2..]);
    ///     let mut store = Store::new(
    ///         &engine,
    ///         MyState {
    ///             ctx: builder.build(),
    ///             table: ResourceTable::new(),
    ///         },
    ///     );
    ///
    ///     // Instantiate the component and we're off to the races.
    ///     let command = Command::instantiate(&mut store, &component, &linker)?;
    ///     let program_result = command.wasi_cli_run().call_run(&mut store)?;
    ///     match program_result {
    ///         Ok(()) => Ok(()),
    ///         Err(()) => std::process::exit(1),
    ///     }
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
    ///
    /// ---
    pub use self::generated::Command;

    /// Pre-instantiated analogue of [`Command`].
    ///
    /// This works the same as [`Command`] but enables front-loading work such
    /// as export lookup to before instantiation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasmtime::{Engine, Result, Store, Config};
    /// use wasmtime::component::{ResourceTable, Linker, Component};
    /// use wasmtime_wasi::{IoView, WasiCtx, WasiView, WasiCtxBuilder};
    /// use wasmtime_wasi::bindings::sync::CommandPre;
    ///
    /// // This example is an example shim of executing a component based on the
    /// // command line arguments provided to this program.
    /// fn main() -> Result<()> {
    ///     let args = std::env::args().skip(1).collect::<Vec<_>>();
    ///
    ///     // Configure and create `Engine`
    ///     let engine = Engine::default();
    ///
    ///     // Configure a `Linker` with WASI, compile a component based on
    ///     // command line arguments, and then pre-instantiate it.
    ///     let mut linker = Linker::<MyState>::new(&engine);
    ///     wasmtime_wasi::add_to_linker_sync(&mut linker)?;
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
    ///             table: ResourceTable::new(),
    ///         },
    ///     );
    ///
    ///     // Instantiate the component and we're off to the races.
    ///     let command = pre.instantiate(&mut store)?;
    ///     let program_result = command.wasi_cli_run().call_run(&mut store)?;
    ///     match program_result {
    ///         Ok(()) => Ok(()),
    ///         Err(()) => std::process::exit(1),
    ///     }
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
    ///
    /// ---
    pub use self::generated::CommandPre;

    pub use self::generated::CommandIndices;
}

mod async_io {
    wasmtime::component::bindgen!({
        path: "src/p3/wit",
        // TODO: Use `command` once 0.3.0 released
        //world: "wasi:cli/command",
        world: "inline:wasi/command",
        inline: "
            package inline:wasi;

            world command {
                include wasi:clocks/imports@0.3.0;
                include wasi:random/imports@0.3.0;
            }
        ",
        tracing: true,
        trappable_imports: true,
        async: {
            // Only these functions are `async` and everything else is sync
            // meaning that it basically doesn't need to block. These functions
            // are the only ones that need to block.
            //
            // Note that at this time `only_imports` works on function names
            // which in theory can be shared across interfaces, so this may
            // need fancier syntax in the future.
            only_imports: [
                "wait-for",
                "wait-until",
            ],
        },
    });
}

pub use self::async_io::wasi::*;

/// Asynchronous bindings to execute and run a `wasi:cli/command`.
///
/// This structure is automatically generated by `bindgen!` and is intended to
/// be used with [`Config::async_support(true)`][async]. For the synchronous
/// version see [`bindings::sync::Command`](sync::Command).
///
/// This can be used for a more "typed" view of executing a command component
/// through the [`Command::wasi_cli_run`] method plus
/// [`Guest::call_run`](exports::wasi::cli::run::Guest::call_run).
///
/// [async]: wasmtime::Config::async_support
/// [`wasmtime_wasi::add_to_linker_async`]: crate::add_to_linker_async
///
/// # Examples
///
/// ```no_run
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{ResourceTable, Linker, Component};
/// use wasmtime_wasi::{IoView, WasiCtx, WasiView, WasiCtxBuilder};
/// use wasmtime_wasi::bindings::Command;
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
///     let engine = Engine::new(&config)?;
///
///     // Configure a `Linker` with WASI, compile a component based on
///     // command line arguments, and then pre-instantiate it.
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::add_to_linker_async(&mut linker)?;
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
///             table: ResourceTable::new(),
///         },
///     );
///
///     // Instantiate the component and we're off to the races.
///     let command = Command::instantiate_async(&mut store, &component, &linker).await?;
///     let program_result = command.wasi_cli_run().call_run(&mut store).await?;
///     match program_result {
///         Ok(()) => Ok(()),
///         Err(()) => std::process::exit(1),
///     }
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
///
/// ---
pub use self::async_io::Command;

/// Pre-instantiated analog of [`Command`]
///
/// This can be used to front-load work such as export lookup before
/// instantiation.
///
/// # Examples
///
/// ```no_run
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{ResourceTable, Linker, Component};
/// use wasmtime_wasi::{IoView, WasiCtx, WasiView, WasiCtxBuilder};
/// use wasmtime_wasi::bindings::CommandPre;
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
///     let engine = Engine::new(&config)?;
///
///     // Configure a `Linker` with WASI, compile a component based on
///     // command line arguments, and then pre-instantiate it.
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::add_to_linker_async(&mut linker)?;
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
///             table: ResourceTable::new(),
///         },
///     );
///
///     // Instantiate the component and we're off to the races.
///     let command = pre.instantiate_async(&mut store).await?;
///     let program_result = command.wasi_cli_run().call_run(&mut store).await?;
///     match program_result {
///         Ok(()) => Ok(()),
///         Err(()) => std::process::exit(1),
///     }
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
///
/// ---
pub use self::async_io::CommandPre;

pub use self::async_io::CommandIndices;

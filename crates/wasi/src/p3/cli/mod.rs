mod host;

use crate::cli::{WasiCli, WasiCliNamed, WasiCliNamedView, WasiCliView};
use crate::p3::Interface;
use crate::p3::bindings::cli::{
    environment, exit, stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr,
    terminal_stdin, terminal_stdout,
};
use crate::{NamedId, WasiCtxNamedView};
use wasmtime::component::{Component, Linker};

/// Add all WASI interfaces from this module into the `linker` provided.
///
/// This function will add all interfaces implemented by this module to the
/// [`Linker`], which corresponds to the `wasi:cli/imports` world supported by
/// this module.
///
/// This is low-level API for advanced use cases,
/// [`wasmtime_wasi::p3::add_to_linker`](crate::p3::add_to_linker) can be used instead
/// to add *all* wasip3 interfaces (including the ones from this module) to the `linker`.
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{Linker, ResourceTable};
/// use wasmtime_wasi::cli::{WasiCliCtx, WasiCliView, WasiCliCtxView};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.wasm_component_model_async(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::p3::cli::add_to_linker(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     let mut store = Store::new(
///         &engine,
///         MyState::default(),
///     );
///
///     // ... use `linker` to instantiate within `store` ...
///
///     Ok(())
/// }
///
/// #[derive(Default)]
/// struct MyState {
///     cli: WasiCliCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiCliView for MyState {
///     fn cli(&mut self) -> WasiCliCtxView<'_> {
///         WasiCliCtxView {
///             ctx: &mut self.cli,
///             table: &mut self.table,
///         }
///     }
/// }
/// ```
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiCliView + 'static,
{
    exit::add_to_linker::<_, WasiCli>(linker, T::cli)?;
    environment::add_to_linker::<_, WasiCli>(linker, T::cli)?;
    stdin::add_to_linker::<_, WasiCli>(linker, T::cli)?;
    stdout::add_to_linker::<_, WasiCli>(linker, T::cli)?;
    stderr::add_to_linker::<_, WasiCli>(linker, T::cli)?;
    terminal_input::add_to_linker::<_, WasiCli>(linker, T::cli)?;
    terminal_output::add_to_linker::<_, WasiCli>(linker, T::cli)?;
    terminal_stdin::add_to_linker::<_, WasiCli>(linker, T::cli)?;
    terminal_stdout::add_to_linker::<_, WasiCli>(linker, T::cli)?;
    terminal_stderr::add_to_linker::<_, WasiCli>(linker, T::cli)?;
    Ok(())
}

/// Convenience function to add `wasi:cli` interfaces into `linker` for any
/// named imports of `wasi:cli` interfaces.
///
/// This function is similar to [`add_to_linker`] except that it's specifically
/// designed to work with named imports of `wasi:cli` interfaces that components
/// may have. This requires a [`Component`] parameter to be passed in when
/// populating the [`Linker`] provided to see what the [`Component`] actually
/// imports.
///
/// Like [`add_to_linker`] this is a bit low level and you may want to possibly
/// invoke [`wasmtime_wasi::p3::add_named_to_linker`] instead. Alternatively if
/// this isn't low level enough you can additionally invoke bindgen-generated
/// `add_to_linker` functions directly from within the
/// [`named_imports::wasi::cli`] module.
///
/// [`wasmtime_wasi::p3::add_named_to_linker`]: crate::p3::add_named_to_linker
/// [`named_imports::wasi::cli`]: crate::p3::bindings::named_imports::wasi::cli
///
/// The `lookup` function provided here is invoked for every named import found
/// for a particular interface. The [`Interface`] given is what's being bound,
/// and the `&str` argument is the name that the component imports it as. The
/// embedder can then decide how it would like to allocate a [`NamedId`] for
/// this import. If `Ok` is returned then the linker is populated with this
/// name, and imported functions will pass the [`NamedId`] later to the
/// implementation of [`WasiCliNamedView`] on `T` when invoked. If `Err` is
/// returned then the error will cause this entire function to fail and this
/// function call will return the same error.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use wasmtime::component::{Component, Linker, ResourceTable};
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime_wasi::NamedId;
/// use wasmtime_wasi::cli::{WasiCliCtx, WasiCliNamedView, WasiCliCtxView};
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
///     // might have for `wasi:cli` interfaces.
///     let mut name_map = HashMap::new();
///     wasmtime_wasi::p3::cli::add_named_to_linker(&mut linker, &component, |_i, name| {
///         let len = name_map.len();
///         Ok(NamedId(*name_map.entry(name.to_string()).or_insert(len)))
///     })?;
///
///     // Here a `WasiCliCtx` is allocated per-named-import and will then be
///     // referred to internally by the [`NamedId`] allocated above. You could
///     // also use `name_map` to configure each context differently.
///     let mut my_state = MyState::default();
///     for _ in 0..name_map.len() {
///         my_state.contexts.push(WasiCliCtx::default());
///     }
///     let mut store = Store::new(&engine, my_state);
///     let instance = linker.instantiate(&mut store, &component)?;
///
///     // ... work with `instance` ...
///
///     Ok(())
/// }
///
/// #[derive(Default)]
/// struct MyState {
///     table: ResourceTable,
///     contexts: Vec<WasiCliCtx>,
/// }
///
/// impl WasiCliNamedView for MyState {
///     fn cli(&mut self, id: NamedId) -> WasiCliCtxView<'_> {
///         WasiCliCtxView {
///             ctx: &mut self.contexts[id.0],
///             table: &mut self.table,
///         }
///     }
/// }
/// ```
pub fn add_named_to_linker<T>(
    linker: &mut Linker<T>,
    component: &Component,
    mut lookup: impl FnMut(Interface, &str) -> wasmtime::Result<NamedId>,
) -> wasmtime::Result<()>
where
    T: WasiCliNamedView + 'static,
{
    use crate::p3::bindings::named_imports::wasi::cli::{
        environment, exit, stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr,
        terminal_stdin, terminal_stdout,
    };
    exit::add_to_linker::<_, WasiCliNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::CliExit, name),
        |x| WasiCtxNamedView(x),
    )?;
    environment::add_to_linker::<_, WasiCliNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::CliEnvironment, name),
        |x| WasiCtxNamedView(x),
    )?;
    stdin::add_to_linker::<_, WasiCliNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::CliStdin, name),
        |x| WasiCtxNamedView(x),
    )?;
    stdout::add_to_linker::<_, WasiCliNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::CliStdout, name),
        |x| WasiCtxNamedView(x),
    )?;
    stderr::add_to_linker::<_, WasiCliNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::CliStderr, name),
        |x| WasiCtxNamedView(x),
    )?;
    terminal_input::add_to_linker::<_, WasiCliNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::CliTerminalInput, name),
        |x| WasiCtxNamedView(x),
    )?;
    terminal_output::add_to_linker::<_, WasiCliNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::CliTerminalOutput, name),
        |x| WasiCtxNamedView(x),
    )?;
    terminal_stdin::add_to_linker::<_, WasiCliNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::CliTerminalStdin, name),
        |x| WasiCtxNamedView(x),
    )?;
    terminal_stdout::add_to_linker::<_, WasiCliNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::CliTerminalStdout, name),
        |x| WasiCtxNamedView(x),
    )?;
    terminal_stderr::add_to_linker::<_, WasiCliNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::CliTerminalStderr, name),
        |x| WasiCtxNamedView(x),
    )?;
    Ok(())
}

pub struct TerminalInput;
pub struct TerminalOutput;

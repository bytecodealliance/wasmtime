mod host;

use crate::clocks::{WasiClocks, WasiClocksNamed, WasiClocksNamedView, WasiClocksView};
use crate::p3::Interface;
use crate::p3::bindings::clocks::{monotonic_clock, system_clock, types};
use crate::{NamedId, WasiCtxNamedView};
use wasmtime::component::{Component, Linker};

/// Add all WASI interfaces from this module into the `linker` provided.
///
/// This function will add all interfaces implemented by this module to the
/// [`Linker`], which corresponds to the `wasi:clocks/imports` world supported by
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
/// use wasmtime_wasi::clocks::{WasiClocksView, WasiClocksCtxView, WasiClocksCtx};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.wasm_component_model_async(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::p3::clocks::add_to_linker(&mut linker)?;
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
///     clocks: WasiClocksCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiClocksView for MyState {
///     fn clocks(&mut self) -> WasiClocksCtxView {
///         WasiClocksCtxView { ctx: &mut self.clocks, table: &mut self.table }
///     }
/// }
/// ```
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiClocksView + 'static,
{
    types::add_to_linker::<_, WasiClocks>(linker, T::clocks)?;
    monotonic_clock::add_to_linker::<_, WasiClocks>(linker, T::clocks)?;
    system_clock::add_to_linker::<_, WasiClocks>(linker, T::clocks)?;
    Ok(())
}

/// Convenience function to add `wasi:clocks` interfaces into `linker` for any
/// named imports of `wasi:clocks` interfaces.
///
/// This function is similar to [`add_to_linker`] except that it's specifically
/// designed to work with named imports of `wasi:clocks` interfaces that
/// components may have. This requires a [`Component`] parameter to be passed in
/// when populating the [`Linker`] provided to see what the [`Component`]
/// actually imports.
///
/// Like [`add_to_linker`] this is a bit low level and you may want to possibly
/// invoke [`wasmtime_wasi::p3::add_named_to_linker`] instead. Alternatively if
/// this isn't low level enough you can additionally invoke bindgen-generated
/// `add_to_linker` functions directly from within the
/// [`named_imports::wasi::clocks`] module.
///
/// [`wasmtime_wasi::p3::add_named_to_linker`]: crate::p3::add_named_to_linker
/// [`named_imports::wasi::clocks`]: crate::p3::bindings::named_imports::wasi::clocks
///
/// The `lookup` function provided here is invoked for every named import found
/// for a particular interface. The [`Interface`] given is what's being bound,
/// and the `&str` argument is the name that the component imports it as. The
/// embedder can then decide how it would like to allocate a [`NamedId`] for
/// this import. If `Ok` is returned then the linker is populated with this
/// name, and imported functions will pass the [`NamedId`] later to the
/// implementation of [`WasiClocksNamedView`] on `T` when invoked. If `Err` is
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
/// use wasmtime_wasi::clocks::{WasiClocksCtx, WasiClocksCtxView, WasiClocksNamedView};
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
///     // might have for `wasi:clocks` interfaces.
///     let mut name_map = HashMap::new();
///     wasmtime_wasi::p3::clocks::add_named_to_linker(&mut linker, &component, |_i, name| {
///         let len = name_map.len();
///         Ok(NamedId(*name_map.entry(name.to_string()).or_insert(len)))
///     })?;
///
///     // Here a `WasiClocksCtx` is allocated per-named-import and will then be
///     // referred to internally by the [`NamedId`] allocated above. You could
///     // also use `name_map` to configure each context differently.
///     let mut my_state = MyState::default();
///     for _ in 0..name_map.len() {
///         my_state.contexts.push(WasiClocksCtx::default());
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
///     contexts: Vec<WasiClocksCtx>,
/// }
///
/// impl WasiClocksNamedView for MyState {
///     fn clocks(&mut self, id: NamedId) -> WasiClocksCtxView<'_> {
///         WasiClocksCtxView {
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
    T: WasiClocksNamedView + 'static,
{
    use crate::p3::bindings::named_imports::wasi::clocks::{monotonic_clock, system_clock};
    monotonic_clock::add_to_linker::<_, WasiClocksNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::ClocksMonotonicClock, name),
        |x| WasiCtxNamedView(x),
    )?;
    system_clock::add_to_linker::<_, WasiClocksNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::ClocksSystemClock, name),
        |x| WasiCtxNamedView(x),
    )?;
    Ok(())
}

impl From<crate::clocks::Datetime> for system_clock::Instant {
    fn from(
        crate::clocks::Datetime {
            seconds,
            nanoseconds,
        }: crate::clocks::Datetime,
    ) -> Self {
        Self {
            seconds,
            nanoseconds,
        }
    }
}

impl From<system_clock::Instant> for crate::clocks::Datetime {
    fn from(
        system_clock::Instant {
            seconds,
            nanoseconds,
        }: system_clock::Instant,
    ) -> Self {
        Self {
            seconds,
            nanoseconds,
        }
    }
}

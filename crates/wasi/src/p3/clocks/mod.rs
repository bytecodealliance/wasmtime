mod host;

use crate::clocks::{WasiClocks, WasiClocksView};
use crate::p3::bindings::clocks::{monotonic_clock, wall_clock};
use cap_std::time::SystemTime;
use wasmtime::component::Linker;

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
///     config.async_support(true);
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
    monotonic_clock::add_to_linker::<_, WasiClocks>(linker, T::clocks)?;
    wall_clock::add_to_linker::<_, WasiClocks>(linker, T::clocks)?;
    Ok(())
}

impl From<crate::clocks::Datetime> for wall_clock::Datetime {
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

impl From<wall_clock::Datetime> for crate::clocks::Datetime {
    fn from(
        wall_clock::Datetime {
            seconds,
            nanoseconds,
        }: wall_clock::Datetime,
    ) -> Self {
        Self {
            seconds,
            nanoseconds,
        }
    }
}

impl TryFrom<SystemTime> for wall_clock::Datetime {
    type Error = wasmtime::Error;

    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        let time = crate::clocks::Datetime::try_from(time)?;
        Ok(time.into())
    }
}

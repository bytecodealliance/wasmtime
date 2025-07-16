mod host;

use crate::clocks::{WasiClocksImpl, WasiClocksView};
use crate::p3::bindings::clocks;
use wasmtime::component::{HasData, Linker};

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
/// use wasmtime::component::Linker;
/// use wasmtime_wasi::clocks::{WasiClocksView, WasiClocksCtx};
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
///         MyState {
///             clocks: WasiClocksCtx::default(),
///         },
///     );
///
///     // ... use `linker` to instantiate within `store` ...
///
///     Ok(())
/// }
///
/// struct MyState {
///     clocks: WasiClocksCtx,
/// }
///
/// impl WasiClocksView for MyState {
///     fn clocks(&mut self) -> &WasiClocksCtx { &self.clocks }
/// }
/// ```
pub fn add_to_linker<T: WasiClocksView + 'static>(linker: &mut Linker<T>) -> wasmtime::Result<()> {
    add_to_linker_impl(linker, |x| WasiClocksImpl(x))
}

pub(crate) fn add_to_linker_impl<T, U>(
    linker: &mut Linker<T>,
    host_getter: fn(&mut T) -> WasiClocksImpl<&mut U>,
) -> wasmtime::Result<()>
where
    T: Send,
    U: WasiClocksView + 'static,
{
    clocks::monotonic_clock::add_to_linker::<_, WasiClocks<U>>(linker, host_getter)?;
    clocks::wall_clock::add_to_linker::<_, WasiClocks<U>>(linker, host_getter)?;
    Ok(())
}

struct WasiClocks<T>(T);

impl<T: 'static> HasData for WasiClocks<T> {
    type Data<'a> = WasiClocksImpl<&'a mut T>;
}

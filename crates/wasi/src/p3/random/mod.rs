mod host;

use crate::p3::bindings::random;
use crate::random::{WasiRandomImpl, WasiRandomView};
use wasmtime::component::{HasData, Linker};

/// Add all WASI interfaces from this module into the `linker` provided.
///
/// This function will add all interfaces implemented by this module to the
/// [`Linker`], which corresponds to the `wasi:random/imports` world supported by
/// this crate.
///
/// This is low-level API for advanced use cases,
/// [`wasmtime_wasi::p3::add_to_linker`](crate::p3::add_to_linker) can be used instead
/// to add *all* wasip3 interfaces (including the ones from this module) to the `linker`.
///
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{ResourceTable, Linker};
/// use wasmtime_wasi::random::{WasiRandomView, WasiRandomCtx};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.async_support(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::p3::random::add_to_linker(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     let mut store = Store::new(
///         &engine,
///         MyState {
///             random: WasiRandomCtx::default(),
///         },
///     );
///
///     // ... use `linker` to instantiate within `store` ...
///
///     Ok(())
/// }
///
/// struct MyState {
///     random: WasiRandomCtx,
/// }
///
/// impl WasiRandomView for MyState {
///     fn random(&mut self) -> &mut WasiRandomCtx { &mut self.random }
/// }
/// ```
pub fn add_to_linker<T: WasiRandomView + 'static>(linker: &mut Linker<T>) -> wasmtime::Result<()> {
    add_to_linker_impl(linker, |x| WasiRandomImpl(x))
}

pub(crate) fn add_to_linker_impl<T, U>(
    linker: &mut Linker<T>,
    host_getter: fn(&mut T) -> WasiRandomImpl<&mut U>,
) -> wasmtime::Result<()>
where
    T: Send,
    U: WasiRandomView + 'static,
{
    random::random::add_to_linker::<_, WasiRandom<U>>(linker, host_getter)?;
    random::insecure::add_to_linker::<_, WasiRandom<U>>(linker, host_getter)?;
    random::insecure_seed::add_to_linker::<_, WasiRandom<U>>(linker, host_getter)?;
    Ok(())
}

struct WasiRandom<T>(T);

impl<T: 'static> HasData for WasiRandom<T> {
    type Data<'a> = WasiRandomImpl<&'a mut T>;
}

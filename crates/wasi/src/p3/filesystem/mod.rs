mod host;

use crate::filesystem::{WasiFilesystem, WasiFilesystemView};
use crate::p3::bindings::filesystem;
use wasmtime::component::Linker;

/// Add all WASI interfaces from this module into the `linker` provided.
///
/// This function will add all interfaces implemented by this module to the
/// [`Linker`], which corresponds to the `wasi:sockets/imports` world supported by
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
/// use wasmtime_wasi::filesystem::{WasiFilesystemCtx, WasiFilesystemCtxView, WasiFilesystemView};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.async_support(true);
///     config.wasm_component_model_async(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::p3::filesystem::add_to_linker(&mut linker)?;
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
///     filesystem: WasiFilesystemCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiFilesystemView for MyState {
///     fn filesystem(&mut self) -> WasiFilesystemCtxView<'_> {
///         WasiFilesystemCtxView {
///             ctx: &mut self.filesystem,
///             table: &mut self.table,
///         }
///     }
/// }
/// ```
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiFilesystemView + 'static,
{
    filesystem::types::add_to_linker::<_, WasiFilesystem>(linker, T::filesystem)?;
    filesystem::preopens::add_to_linker::<_, WasiFilesystem>(linker, T::filesystem)?;
    Ok(())
}

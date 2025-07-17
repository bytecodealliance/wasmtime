//! Experimental, unstable and incomplete implementation of wasip3 version of WASI.
//!
//! This module is under heavy development.
//! It is not compliant with semver and is not ready
//! for production use.
//!
//! Bug and security fixes limited to wasip3 will not be given patch releases.
//!
//! Documentation of this module may be incorrect or out-of-sync with the implementation.

pub mod bindings;
pub mod cli;
pub mod clocks;
mod ctx;
pub mod filesystem;
pub mod random;
mod view;

use wasmtime::component::Linker;

use crate::clocks::WasiClocksImpl;
use crate::p3::bindings::LinkOptions;
use crate::p3::cli::WasiCliCtxView;
use crate::random::WasiRandomImpl;

pub use self::ctx::{WasiCtx, WasiCtxBuilder};
pub use self::view::{WasiCtxView, WasiView};

/// Add all WASI interfaces from this module into the `linker` provided.
///
/// This function will add all interfaces implemented by this module to the
/// [`Linker`], which corresponds to the `wasi:cli/imports` world supported by
/// this module.
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{Linker, ResourceTable};
/// use wasmtime_wasi::p3::{WasiCtx, WasiCtxView, WasiView};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.async_support(true);
///     config.wasm_component_model_async(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::p3::add_to_linker(&mut linker)?;
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
///     ctx: WasiCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> WasiCtxView<'_> {
///         WasiCtxView{
///             ctx: &mut self.ctx,
///             table: &mut self.table,
///         }
///     }
/// }
/// ```
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiView + 'static,
{
    let options = LinkOptions::default();
    add_to_linker_with_options(linker, &options)
}

/// Similar to [`add_to_linker`], but with the ability to enable unstable features.
pub fn add_to_linker_with_options<T>(
    linker: &mut Linker<T>,
    options: &LinkOptions,
) -> anyhow::Result<()>
where
    T: WasiView + 'static,
{
    clocks::add_to_linker_impl(linker, |x| WasiClocksImpl(&mut x.ctx().ctx.clocks))?;
    random::add_to_linker_impl(linker, |x| WasiRandomImpl(&mut x.ctx().ctx.random))?;
    cli::add_to_linker_impl(linker, &options.into(), |x| {
        let WasiCtxView { ctx, table } = x.ctx();
        WasiCliCtxView {
            ctx: &mut ctx.cli,
            table,
        }
    })?;
    Ok(())
}

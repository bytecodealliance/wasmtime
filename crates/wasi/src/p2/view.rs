use crate::p2::ctx::WasiCtx;
use wasmtime::component::ResourceTable;

/// A trait which provides access to the [`WasiCtx`] inside the embedder's `T`
/// of [`Store<T>`][`Store`].
///
/// This crate's WASI Host implementations depend on the contents of
/// [`WasiCtx`]. The `T` type [`Store<T>`][`Store`] is defined in each
/// embedding of Wasmtime. These implementations are connected to the
/// [`Linker<T>`][`Linker`] by the
/// [`add_to_linker_sync`](crate::p2::add_to_linker_sync) and
/// [`add_to_linker_async`](crate::p2::add_to_linker_async) functions.
///
/// # Example
///
/// ```
/// use wasmtime_wasi::ResourceTable;
/// use wasmtime_wasi::p2::{WasiCtx, WasiCtxView, WasiView, WasiCtxBuilder};
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
/// [`Store`]: wasmtime::Store
/// [`Linker`]: wasmtime::component::Linker
/// [`ResourceTable`]: wasmtime::component::ResourceTable
///
pub trait WasiView: Send + 'static {
    /// Yields mutable access to the [`WasiCtx`] configuration used for this
    /// context.
    fn ctx(&mut self) -> WasiCtxView<'_>;
}

/// "View" type into WASI state returned by [`WasiView::ctx`].
///
/// This type is used to implement most WASI traits in this crate and contains
/// the fields necessary to implement WASI functionality.
pub struct WasiCtxView<'a> {
    /// Generic WASI state such as preopened files, randomness configuration,
    /// socket configuration, etc.
    pub ctx: &'a mut WasiCtx,

    /// Handle state used to manipulate WASI resources.
    pub table: &'a mut ResourceTable,
}

use wasmtime::component::ResourceTable;

use crate::p3::ctx::WasiCtx;

/// A trait which provides access to the [`WasiCtx`] inside the embedder's `T`
/// of [`Store<T>`][`Store`].
///
/// This crate's WASI Host implementations depend on the contents of
/// [`WasiCtx`]. The `T` type [`Store<T>`][`Store`] is defined in each
/// embedding of Wasmtime. These implementations are connected to the
/// [`Linker<T>`][`Linker`] by the
/// [`add_to_linker`](crate::p3::add_to_linker) function.
///
/// # Example
///
/// ```
/// use wasmtime_wasi::p3::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
/// use wasmtime::component::ResourceTable;
///
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
/// [`Store`]: wasmtime::Store
/// [`Linker`]: wasmtime::component::Linker
///
pub trait WasiView: Send {
    /// Yields mutable access to the [`WasiCtx`] configuration used for this
    /// context.
    fn ctx(&mut self) -> WasiCtxView<'_>;
}

impl<T: ?Sized + WasiView> WasiView for &mut T {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        T::ctx(self)
    }
}

impl<T: ?Sized + WasiView> WasiView for Box<T> {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        T::ctx(self)
    }
}

pub struct WasiCtxView<'a> {
    pub ctx: &'a mut WasiCtx,
    pub table: &'a mut ResourceTable,
}

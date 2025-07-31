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

pub struct WasiCtxView<'a> {
    pub ctx: &'a mut WasiCtx,
    pub table: &'a mut ResourceTable,
}

impl<T: WasiView> crate::sockets::WasiSocketsView for T {
    fn sockets(&mut self) -> crate::sockets::WasiSocketsCtxView<'_> {
        let WasiCtxView { ctx, table } = self.ctx();
        crate::sockets::WasiSocketsCtxView {
            ctx: &mut ctx.sockets,
            table,
        }
    }
}

impl<T: WasiView> crate::clocks::WasiClocksView for T {
    fn clocks(&mut self) -> &mut crate::clocks::WasiClocksCtx {
        &mut self.ctx().ctx.clocks
    }
}

impl<T: WasiView> crate::random::WasiRandomView for T {
    fn random(&mut self) -> &mut crate::random::WasiRandomCtx {
        &mut self.ctx().ctx.random
    }
}

impl<T: WasiView> crate::p3::cli::WasiCliView for T {
    fn cli(&mut self) -> crate::p3::cli::WasiCliCtxView<'_> {
        let WasiCtxView { ctx, table } = self.ctx();
        crate::p3::cli::WasiCliCtxView {
            ctx: &mut ctx.cli,
            table,
        }
    }
}

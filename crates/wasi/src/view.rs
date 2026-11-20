use crate::WasiCtx;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::Arc;
use wasmtime::component::ResourceTable;

/// A trait which provides access to the [`WasiCtx`] inside the embedder's `T`
/// of [`Store<T>`][`Store`].
///
/// This crate's WASI Host implementations depend on the contents of
/// [`WasiCtx`]. The `T` type [`Store<T>`][`Store`] is defined in each
/// embedding of Wasmtime. These implementations are connected to the
/// [`Linker<T>`][`Linker`] by [`add_to_linker_async`](crate::p2::add_to_linker_async)
/// functions.
///
/// # Example
///
/// ```
/// use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
/// use wasmtime::component::ResourceTable;
///
/// struct MyState {
///     ctx: WasiCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiView for MyState {
///     fn ctx(&mut self) -> WasiCtxView<'_> {
///         WasiCtxView {
///             ctx: &mut self.ctx,
///             table: &mut self.table,
///             hooks: Default::default(),
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

/// Structure returned from [`WasiView::ctx`] which provides access to WASI
/// state for host functions to be implemented with.
pub struct WasiCtxView<'a> {
    /// The [`WasiCtx`], or configuration, of the guest.
    pub ctx: &'a mut WasiCtx,
    /// Resources, such as files/streams, that the guest is using.
    pub table: &'a mut ResourceTable,
    /// Mutable reference to the WASI hooks.
    ///
    /// Note that [`default_hooks`] or [`Default::default()`] can be used if
    /// you don't want or need to customize this.
    pub hooks: &'a mut dyn WasiHooks,
}

impl<T: WasiView> crate::cli::WasiCliView for T {
    fn cli(&mut self) -> crate::cli::WasiCliCtxView<'_> {
        let WasiCtxView { ctx, table, .. } = self.ctx();
        crate::cli::WasiCliCtxView {
            ctx: &mut ctx.cli,
            table,
        }
    }
}

impl<T: WasiView> crate::clocks::WasiClocksView for T {
    fn clocks(&mut self) -> crate::clocks::WasiClocksCtxView<'_> {
        let WasiCtxView { ctx, table, .. } = self.ctx();
        crate::clocks::WasiClocksCtxView {
            ctx: &mut ctx.clocks,
            table,
        }
    }
}

impl<T: WasiView> crate::filesystem::WasiFilesystemView for T {
    fn filesystem(&mut self) -> crate::filesystem::WasiFilesystemCtxView<'_> {
        let WasiCtxView { ctx, table, hooks } = self.ctx();
        crate::filesystem::WasiFilesystemCtxView {
            ctx: &mut ctx.filesystem,
            table,
            hooks,
        }
    }
}

impl<T: WasiView> crate::random::WasiRandomView for T {
    fn random(&mut self) -> &mut crate::random::WasiRandomCtx {
        &mut self.ctx().ctx.random
    }
}

impl<T: WasiView> crate::sockets::WasiSocketsView for T {
    fn sockets(&mut self) -> crate::sockets::WasiSocketsCtxView<'_> {
        let WasiCtxView { ctx, table, hooks } = self.ctx();
        crate::sockets::WasiSocketsCtxView {
            ctx: &mut ctx.sockets,
            table,
            hooks,
        }
    }
}

/// Identifier used to correlate a named import to a host-defined value.
///
/// This type is a small newtype wrapper around a `usize`, its only field which
/// is also public. Embedders define the meaning of this value and are in control
/// of both creating this and interpreting it. Creation of [`NamedId`] happens in
/// functions like [`wasmtime_wasi::p3::add_named_to_linker`] where embedders
/// will allocate a [`NamedId`] for all recognized named imports. Interpreting
/// a [`NamedId`] happens later in traits such as [`WasiNamedView`] where the id
/// is passed back to the embedder and a corresponding context must be returned.
///
/// Internally the `wasmtime-wasi` crate does not inspect the internal field
/// here nor ever create one. Embedders are solely responsible for managing this
/// id and its contents.
///
/// [`wasmtime_wasi::p3::add_named_to_linker`]: crate::p3::add_named_to_linker
#[derive(Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Debug)]
pub struct NamedId(pub usize);

/// A helper structure by which to implement generated-`Host` traits for
/// bindings corresponding to named imports.
///
/// This is an implementation detail of how named imports are organized.
/// Embedders invoking generated `add_to_linker` functions directly will be
/// required to create this structure directly. Otherwise you probably won't
/// need to interact with this.
pub struct WasiCtxNamedView<'a, T>(pub &'a mut T);

/// A trait which provides access to a specific [`WasiCtx`] as scoped by a
/// [`NamedId`] parameter.
///
/// This trait is used as part of convenience
/// [`wasmtime_wasi::p3::add_named_to_linker`] functions for example. This is
/// used to scope access of a WASI context within a specific `id` or named
/// import.
///
/// [`wasmtime_wasi::p3::add_named_to_linker`]: crate::p3::add_named_to_linker
pub trait WasiNamedView: Send + 'static {
    /// Yields mutable access to the [`WasiCtx`] configuration used for this
    /// context.
    ///
    /// It's strongly recommended, though not always required, that the
    /// [`ResourceTable`] in the returned [`WasiCtxView`] is the same table that
    /// [`WasiView::ctx`] returns. Resources created through a named import can
    /// be used through interfaces bound to the default context (and vice
    /// versa), for example streams returned by a named `wasi:cli/stdout` are
    /// used through `wasi:io/streams`, and with separate tables those lookups
    /// fail or find another context's resources.
    fn ctx(&mut self, id: NamedId) -> WasiCtxView<'_>;
}

impl<T: WasiNamedView> crate::cli::WasiCliNamedView for T {
    fn cli(&mut self, id: NamedId) -> crate::cli::WasiCliCtxView<'_> {
        let WasiCtxView { ctx, table, .. } = self.ctx(id);
        crate::cli::WasiCliCtxView {
            ctx: &mut ctx.cli,
            table,
        }
    }
}

impl<T: WasiNamedView> crate::clocks::WasiClocksNamedView for T {
    fn clocks(&mut self, id: NamedId) -> crate::clocks::WasiClocksCtxView<'_> {
        let WasiCtxView { ctx, table, .. } = self.ctx(id);
        crate::clocks::WasiClocksCtxView {
            ctx: &mut ctx.clocks,
            table,
        }
    }
}

impl<T: WasiNamedView> crate::filesystem::WasiFilesystemNamedView for T {
    fn filesystem(&mut self, id: NamedId) -> crate::filesystem::WasiFilesystemCtxView<'_> {
        let WasiCtxView { ctx, table, hooks } = self.ctx(id);
        crate::filesystem::WasiFilesystemCtxView {
            ctx: &mut ctx.filesystem,
            table,
            hooks,
        }
    }
}

impl<T: WasiNamedView> crate::random::WasiRandomNamedView for T {
    fn random(&mut self, id: NamedId) -> &mut crate::random::WasiRandomCtx {
        self.ctx(id).ctx.random()
    }
}

impl<T: WasiNamedView> crate::sockets::WasiSocketsNamedView for T {
    fn sockets(&mut self, id: NamedId) -> crate::sockets::WasiSocketsCtxView<'_> {
        let WasiCtxView { ctx, table, hooks } = self.ctx(id);
        crate::sockets::WasiSocketsCtxView {
            ctx: &mut ctx.sockets,
            table,
            hooks,
        }
    }
}

/// Trait representing a permit returned by `WasiHooks::acquire`.
pub trait WasiPermit: Send + Sync {}

/// Type of permit to request via `WasiHooks::acquire`.
#[derive(Clone, Copy)]
pub enum WasiPermitType {
    /// Request a permit to open a `wasi:filesystem` file or directory.
    FileOrDirectory,
    /// Request a permit to create a `wasi:sockets` TCP or UDP socket.
    Socket,
}

/// A trait which provides hooks into internal WASI operations.
pub trait WasiHooks: Send + Sync {
    /// Acquire a permit to create a host file descriptor to be wrapped in a
    /// resource and returned to the guest.
    ///
    /// The `ty` parameter indicates which type of file descriptor is to be
    /// created.  The result, if successful, represents a semaphore-style permit
    /// which will be dropped immediately _after_ closing the file descriptor.
    /// Therefore, the implementation may, in the `Drop` impl, e.g. return the
    /// permit into a pool and unblock other requests, as appropriate.
    fn acquire(
        &self,
        ty: WasiPermitType,
    ) -> Pin<Box<dyn Future<Output = wasmtime::Result<Box<dyn WasiPermit>>> + Send>> {
        _ = ty;
        Box::pin(async { Ok(Box::new(DummyPermit) as Box<dyn WasiPermit>) })
    }

    /// Returns a "clone" of `self` for use in `'static` contexts.
    fn owned(&self) -> Arc<dyn WasiHooks> {
        Arc::new([(); 0])
    }
}

/// Returns a value suitable for the `WasiCtxView::hooks` field which has
/// the default behavior.
pub fn default_hooks() -> &'static mut dyn WasiHooks {
    Default::default()
}

impl<'a> Default for &'a mut dyn WasiHooks {
    fn default() -> Self {
        let x: &mut [(); 0] = &mut [];
        x
    }
}

#[doc(hidden)]
impl WasiHooks for [(); 0] {}

pub(crate) struct DummyPermit;

impl WasiPermit for DummyPermit {}

/// Bundles an inner value with an associated `WasiPermit`, ensuring that the
/// former is dropped before the latter.
pub struct Permitted<T> {
    inner: T,
    // Note that we declare `permit` last to ensure that it is dropped last per
    // Rust's field drop order.
    permit: Box<dyn WasiPermit>,
}

impl<T> Permitted<T> {
    pub(crate) fn new(inner: T, permit: Box<dyn WasiPermit>) -> Self {
        Self { inner, permit }
    }

    pub(crate) fn try_map<U, E>(
        self,
        fun: impl FnOnce(T) -> Result<U, E>,
    ) -> Result<Permitted<U>, E> {
        Ok(Permitted {
            inner: fun(self.inner)?,
            permit: self.permit,
        })
    }

    pub(crate) async fn try_map_async<U, E>(
        self,
        fun: impl AsyncFnOnce(T) -> Result<U, E>,
    ) -> Result<Permitted<U>, E> {
        Ok(Permitted {
            inner: fun(self.inner).await?,
            permit: self.permit,
        })
    }
}

impl<T> Deref for Permitted<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Permitted<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

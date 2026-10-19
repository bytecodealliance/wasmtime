use crate::{WasiTls, WasiTlsNamed, WasiTlsNamedView, WasiTlsView};
use wasmtime::component::{Component, Linker};
use wasmtime_wasi::{NamedId, WasiCtxNamedView};

pub mod bindings;
mod host;
mod io;

pub use bindings::types::LinkOptions;
pub use host::{HostClientConnection, HostClientHandshake, HostFutureClientStreams};

/// Add the `wasi-tls` world's types to a [`wasmtime::component::Linker`].
pub fn add_to_linker<T>(
    l: &mut wasmtime::component::Linker<T>,
    opts: &LinkOptions,
) -> wasmtime::Result<()>
where
    T: WasiTlsView + 'static,
{
    bindings::types::add_to_linker::<_, WasiTls>(l, &opts, T::tls)?;
    Ok(())
}

/// Interfaces that are added via [`add_named_to_linker`].
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Interface {
    /// `wasi:tls/types`
    TlsTypes,
}

/// Add the `wasi-tls` world's types to `linker` for any named imports of them
/// that `component` has.
///
/// This is the named-import counterpart of [`add_to_linker`]: `component`'s
/// imports are inspected and, for each import that implements a `wasi:tls`
/// interface under a different name, `lookup` is invoked with the
/// [`Interface`] being bound and the name of the import. The [`NamedId`]
/// returned is later passed to [`WasiTlsNamedView::tls`] when that import is
/// used. If `lookup` returns an error then this function returns that error.
pub fn add_named_to_linker<T>(
    l: &mut Linker<T>,
    opts: &LinkOptions,
    component: &Component,
    mut lookup: impl FnMut(Interface, &str) -> wasmtime::Result<NamedId>,
) -> wasmtime::Result<()>
where
    T: WasiTlsNamedView,
{
    bindings::named_imports::wasi::tls::types::add_to_linker::<_, WasiTlsNamed<T>>(
        l,
        component,
        |name| lookup(Interface::TlsTypes, name),
        opts,
        |x| WasiCtxNamedView(x),
    )
}

//! Experimental, unstable and incomplete implementation of wasip3 version of `wasi:tls`.
//!
//! This module is under heavy development.
//! It is not compliant with semver and is not ready
//! for production use.
//!
//! Bug and security fixes limited to wasip3 will not be given patch releases.
//!
//! Documentation of this module may be incorrect or out-of-sync with the implementation.

pub mod bindings;
pub mod host;
pub(crate) mod util;

use crate::{WasiTls, WasiTlsNamed, WasiTlsNamedView, WasiTlsView};
use bindings::tls::{client, types};
use wasmtime::component::{Component, Linker};
use wasmtime_wasi::{NamedId, WasiCtxNamedView};

/// Add all interfaces from this module into the `linker` provided.
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiTlsView + 'static,
{
    client::add_to_linker::<_, WasiTls>(linker, T::tls)?;
    types::add_to_linker::<_, WasiTls>(linker, T::tls)?;
    Ok(())
}

/// Interfaces that are added via [`add_named_to_linker`].
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Interface {
    /// `wasi:tls/client`
    TlsClient,
    /// `wasi:tls/types`
    TlsTypes,
}

/// Add all interfaces from this module into `linker` for any named imports of
/// them that `component` has.
///
/// This is the named-import counterpart of [`add_to_linker`]: `component`'s
/// imports are inspected and, for each import that implements a `wasi:tls`
/// interface under a different name, `lookup` is invoked with the
/// [`Interface`] being bound and the name of the import. The [`NamedId`]
/// returned is later passed to [`WasiTlsNamedView::tls`] when that import is
/// used. If `lookup` returns an error then this function returns that error.
pub fn add_named_to_linker<T>(
    linker: &mut Linker<T>,
    component: &Component,
    mut lookup: impl FnMut(Interface, &str) -> wasmtime::Result<NamedId>,
) -> wasmtime::Result<()>
where
    T: WasiTlsNamedView,
{
    use bindings::named_imports::wasi::tls::{client, types};

    client::add_to_linker::<_, WasiTlsNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::TlsClient, name),
        |x| WasiCtxNamedView(x),
    )?;
    types::add_to_linker::<_, WasiTlsNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::TlsTypes, name),
        |x| WasiCtxNamedView(x),
    )?;
    Ok(())
}

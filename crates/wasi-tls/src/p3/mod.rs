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

use crate::WasiTlsCtx;
use bindings::tls::{client, types};
use wasmtime::component::{HasData, Linker, ResourceTable};

/// The type for which this crate implements the `wasi:tls` interfaces.
pub struct WasiTls;

impl HasData for WasiTls {
    type Data<'a> = WasiTlsCtxView<'a>;
}

/// View into [`WasiTlsCtx`] implementation and [`ResourceTable`].
pub struct WasiTlsCtxView<'a> {
    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,

    /// Mutable reference to the WASI TLS context.
    pub ctx: &'a mut WasiTlsCtx,
}

/// A trait which provides internal WASI TLS state.
pub trait WasiTlsView: Send {
    /// Return a [`WasiTlsCtxView`] from mutable reference to self.
    fn tls(&mut self) -> WasiTlsCtxView<'_>;
}

/// Add all interfaces from this module into the `linker` provided.
///
/// This function will eventually add all interfaces implemented by this module
/// to the [`Linker`], corresponding to the `wasi:tls/imports` world.
///
/// At this stage the p3 host implementation is intentionally scaffolded only.
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiTlsView + 'static,
{
    client::add_to_linker::<_, WasiTls>(linker, T::tls)?;
    types::add_to_linker::<_, WasiTls>(linker, T::tls)?;
    Ok(())
}

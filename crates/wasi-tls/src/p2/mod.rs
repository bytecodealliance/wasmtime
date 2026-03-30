use wasmtime::component::{HasData, ResourceTable};

use crate::WasiTlsCtx;

pub mod bindings;
mod host;
mod io;

pub use bindings::types::LinkOptions;
pub use host::{HostClientConnection, HostClientHandshake, HostFutureClientStreams};

/// Capture the state necessary for use in the `wasi-tls` API implementation.
pub struct WasiTls<'a> {
    pub(crate) ctx: &'a WasiTlsCtx,
    pub(crate) table: &'a mut ResourceTable,
}

impl<'a> WasiTls<'a> {
    /// Create a new Wasi TLS context.
    pub fn new(ctx: &'a WasiTlsCtx, table: &'a mut ResourceTable) -> Self {
        Self { ctx, table }
    }
}

/// Add the `wasi-tls` world's types to a [`wasmtime::component::Linker`].
pub fn add_to_linker<T: Send + 'static>(
    l: &mut wasmtime::component::Linker<T>,
    opts: &mut LinkOptions,
    f: fn(&mut T) -> WasiTls<'_>,
) -> wasmtime::Result<()> {
    bindings::types::add_to_linker::<_, HasWasiTls>(l, &opts, f)?;
    Ok(())
}

struct HasWasiTls;
impl HasData for HasWasiTls {
    type Data<'a> = WasiTls<'a>;
}

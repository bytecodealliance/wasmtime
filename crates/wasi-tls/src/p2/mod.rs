use crate::{WasiTls, WasiTlsView};

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

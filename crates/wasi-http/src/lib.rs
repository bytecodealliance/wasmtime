use crate::component_impl::add_component_to_linker;
pub use crate::r#struct::WasiHttp;

wasmtime::component::bindgen!("proxy");

pub mod component_impl;
pub mod http_impl;
pub mod streams_impl;
pub mod r#struct;
pub mod types_impl;

pub fn add_to_component_linker<T>(
    linker: &mut wasmtime::component::Linker<T>,
    get_cx: impl Fn(&mut T) -> &mut WasiHttp + Send + Sync + Copy + 'static,
) -> anyhow::Result<()> {
    default_outgoing_http::add_to_linker(linker, get_cx)?;
    types::add_to_linker(linker, get_cx)?;
    streams::add_to_linker(linker, get_cx)?;
    Ok(())
}

pub fn add_to_linker<T>(
    linker: &mut wasmtime::Linker<T>,
    get_cx: impl Fn(&mut T) -> &mut WasiHttp + Send + Sync + Copy + 'static,
) -> anyhow::Result<()> {
    add_component_to_linker(linker, get_cx)
}

use crate::component_impl::add_component_to_linker;
pub use crate::r#struct::WasiHttp;

wasmtime::component::bindgen!({ path: "wasi-http/wit", world: "proxy"});

pub mod component_impl;
pub mod http_impl;
pub mod streams_impl;
pub mod r#struct;
pub mod types_impl;

pub fn add_to_component_linker<T>(
    linker: &mut wasmtime::component::Linker<T>,
    get_cx: impl Fn(&mut T) -> &mut WasiHttp + Send + Sync + Copy + 'static,
) -> anyhow::Result<()> {
    crate::wasi::http::outgoing_handler::add_to_linker(linker, get_cx)?;
    crate::wasi::http::types::add_to_linker(linker, get_cx)?;
    crate::wasi::io::streams::add_to_linker(linker, get_cx)?;
    Ok(())
}

pub fn add_to_linker<T>(
    linker: &mut wasmtime::Linker<T>,
    get_cx: impl Fn(&mut T) -> &mut WasiHttp + Send + Sync + Copy + 'static,
) -> anyhow::Result<()> {
    add_component_to_linker(linker, get_cx)
}

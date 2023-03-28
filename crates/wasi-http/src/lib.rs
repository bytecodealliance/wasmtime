use crate::component_impl::add_component_to_linker;
use crate::http_server::async_http_server;
use crate::http_server::spawn_http_server;
pub use crate::r#struct::WasiHttp;
use wasmtime::Store;

wasmtime::component::bindgen!({ path: "wasi-http/wit", world: "proxy"});

pub mod component_impl;
pub mod http_impl;
pub mod http_server;
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

pub fn run_http<T>(
    linker: &mut wasmtime::Linker<T>,
    wasi_http: &mut Store<T>,
    get_cx: impl Fn(&mut T) -> &mut WasiHttp + Send + Sync + Copy + 'static,
) {
    spawn_http_server(linker, wasi_http, get_cx);
}

pub async fn async_run_http<T>(
    linker: &mut wasmtime::Linker<T>,
    wasi_http: &mut Store<T>,
    get_cx: impl Fn(&mut T) -> &mut WasiHttp + Send + Sync + Copy + 'static,
) {
    async_http_server(linker, wasi_http, get_cx).await
}

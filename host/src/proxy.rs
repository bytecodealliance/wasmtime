use crate::WasiCtx;

pub mod wasi {
    wasmtime::component::bindgen!({
        path: "../wit",
        world: "proxy",
        tracing: true,
        async: true,
        trappable_error_type: {
            "filesystem"::"error-code": Error,
            "streams"::"stream-error": Error,
        }
    });
}

pub fn add_to_linker<T: Send>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl (Fn(&mut T) -> &mut WasiCtx) + Copy + Send + Sync + 'static,
) -> anyhow::Result<()> {
    wasi::random::add_to_linker(l, f)?;
    wasi::console::add_to_linker(l, f)?;
    wasi::types::add_to_linker(l, f)?;
    wasi::poll::add_to_linker(l, f)?;
    wasi::streams::add_to_linker(l, f)?;
    wasi::default_outgoing_http::add_to_linker(l, f)?;
    Ok(())
}

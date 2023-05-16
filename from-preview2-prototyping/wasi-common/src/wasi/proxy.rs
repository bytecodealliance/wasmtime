use crate::WasiView;

wasmtime::component::bindgen!({
    path: "../wit",
    world: "proxy",
    tracing: true,
    async: true,
    trappable_error_type: {
        "filesystem"::"error-code": Error,
        "streams"::"stream-error": Error,
    },
    with: {
       "filesystem": crate::wasi::filesystem,
       "monotonic_clock": crate::wasi::monotonic_clock,
       "poll": crate::wasi::poll,
       "streams": crate::wasi::streams,
       "timezone": crate::wasi::timezone,
       "wall_clock": crate::wasi::wall_clock,
       "random": crate::wasi::random,
       "environment": crate::wasi::environment,
       "exit": crate::wasi::exit,
       "preopens": crate::wasi::preopens,
    },
});

pub fn add_to_linker<T: WasiView>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()> {
    crate::wasi::random::add_to_linker(l, |t| t)?;
    crate::wasi::console::add_to_linker(l, |t| t)?;
    crate::wasi::types::add_to_linker(l, |t| t)?;
    crate::wasi::poll::add_to_linker(l, |t| t)?;
    crate::wasi::streams::add_to_linker(l, |t| t)?;
    crate::wasi::default_outgoing_http::add_to_linker(l, |t| t)?;
    Ok(())
}

use crate::WasiCtx;

wasmtime::component::bindgen!({
    path: "../wit",
    world: "command",
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

pub fn add_to_linker<T: Send>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl (Fn(&mut T) -> &mut WasiCtx) + Copy + Send + Sync + 'static,
) -> anyhow::Result<()> {
    crate::wasi::wall_clock::add_to_linker(l, f)?;
    crate::wasi::monotonic_clock::add_to_linker(l, f)?;
    crate::wasi::timezone::add_to_linker(l, f)?;
    crate::wasi::filesystem::add_to_linker(l, f)?;
    crate::wasi::poll::add_to_linker(l, f)?;
    crate::wasi::streams::add_to_linker(l, f)?;
    crate::wasi::random::add_to_linker(l, f)?;
    crate::wasi::exit::add_to_linker(l, f)?;
    crate::wasi::environment::add_to_linker(l, f)?;
    crate::wasi::preopens::add_to_linker(l, f)?;
    Ok(())
}

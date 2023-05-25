use crate::preview2::WasiView;

wasmtime::component::bindgen!({
    path: "wit",
    world: "command",
    tracing: true,
    async: true,
    trappable_error_type: {
        "filesystem"::"error-code": Error,
        "streams"::"stream-error": Error,
    },
    with: {
       "filesystem": crate::preview2::wasi::filesystem,
       "monotonic_clock": crate::preview2::wasi::monotonic_clock,
       "poll": crate::preview2::wasi::poll,
       "streams": crate::preview2::wasi::streams,
       "timezone": crate::preview2::wasi::timezone,
       "wall_clock": crate::preview2::wasi::wall_clock,
       "random": crate::preview2::wasi::random,
       "environment": crate::preview2::wasi::environment,
       "exit": crate::preview2::wasi::exit,
       "preopens": crate::preview2::wasi::preopens,
       "stdin": crate::preview2::wasi::stdin,
       "stdout": crate::preview2::wasi::stdout,
       "stderr": crate::preview2::wasi::stderr,
    },
});

pub fn add_to_linker<T: WasiView>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()> {
    crate::preview2::wasi::wall_clock::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::monotonic_clock::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::timezone::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::filesystem::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::poll::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::streams::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::random::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::exit::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::environment::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::preopens::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::stdin::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::stdout::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::stderr::add_to_linker(l, |t| t)?;
    Ok(())
}

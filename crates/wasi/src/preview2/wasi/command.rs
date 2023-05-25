use crate::preview2::WasiView;

wasmtime::component::bindgen!({
    world: "wasi:preview/command",
    tracing: true,
    async: true,
    trappable_error_type: {
        "filesystem"::"error-code": Error,
        "streams"::"stream-error": Error,
    },
    with: {
       "wasi:filesystem/filesystem": crate::preview2::wasi::filesystem::filesystem,
       "wasi:clocks/monotonic_clock": crate::preview2::wasi::clocks::monotonic_clock,
       "wasi:poll/poll": crate::preview2::wasi::poll::poll,
       "wasi:io/streams": crate::preview2::wasi::io::streams,
       "wasi:clocks/timezone": crate::preview2::wasi::clocks::timezone,
       "wasi:clocks/wall_clock": crate::preview2::wasi::clocks::wall_clock,
       "wasi:random/random": crate::preview2::wasi::random::random,
       "wasi:cli_base/environment": crate::preview2::wasi::cli_base::environment,
       "wasi:cli_base/exit": crate::preview2::wasi::cli_base::exit,
       "wasi:cli_base/preopens": crate::preview2::wasi::cli_base::preopens,
       "wasi:cli_base/stdin": crate::preview2::wasi::cli_base::stdin,
       "wasi:cli_base/stdout": crate::preview2::wasi::cli_base::stdout,
       "wasi:cli_base/stderr": crate::preview2::wasi::cli_base::stderr,
    },
});

pub fn add_to_linker<T: WasiView>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()> {
    crate::preview2::wasi::clocks::wall_clock::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::clocks::monotonic_clock::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::clocks::timezone::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::filesystem::filesystem::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::poll::poll::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::io::streams::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::random::random::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::cli_base::exit::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::cli_base::environment::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::cli_base::preopens::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::cli_base::stdin::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::cli_base::stdout::add_to_linker(l, |t| t)?;
    crate::preview2::wasi::cli_base::stderr::add_to_linker(l, |t| t)?;
    Ok(())
}

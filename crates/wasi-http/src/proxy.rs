use crate::{bindings, WasiHttpView};
use wasmtime_wasi::preview2;

wasmtime::component::bindgen!({
    world: "wasi:http/proxy",
    tracing: true,
    async: true,
    with: {
        "wasi:cli/stderr": preview2::bindings::cli::stderr,
        "wasi:cli/stdin": preview2::bindings::cli::stdin,
        "wasi:cli/stdout": preview2::bindings::cli::stdout,
        "wasi:clocks/monotonic-clock": preview2::bindings::clocks::monotonic_clock,
        "wasi:clocks/timezone": preview2::bindings::clocks::timezone,
        "wasi:clocks/wall-clock": preview2::bindings::clocks::wall_clock,
        "wasi:http/incoming-handler": bindings::http::incoming_handler,
        "wasi:http/outgoing-handler": bindings::http::outgoing_handler,
        "wasi:http/types": bindings::http::types,
        "wasi:io/streams": preview2::bindings::io::streams,
        "wasi:io/poll": preview2::bindings::io::poll,
        "wasi:random/random": preview2::bindings::random::random,
    },
});

pub fn add_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView + preview2::WasiView + bindings::http::types::Host,
{
    preview2::bindings::clocks::wall_clock::add_to_linker(l, |t| t)?;
    preview2::bindings::clocks::monotonic_clock::add_to_linker(l, |t| t)?;
    preview2::bindings::io::poll::add_to_linker(l, |t| t)?;
    preview2::bindings::io::error::add_to_linker(l, |t| t)?;
    preview2::bindings::io::streams::add_to_linker(l, |t| t)?;
    preview2::bindings::cli::stdin::add_to_linker(l, |t| t)?;
    preview2::bindings::cli::stdout::add_to_linker(l, |t| t)?;
    preview2::bindings::cli::stderr::add_to_linker(l, |t| t)?;
    preview2::bindings::random::random::add_to_linker(l, |t| t)?;

    add_only_http_to_linker(l)
}

#[doc(hidden)]
pub fn add_only_http_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView + preview2::WasiView + bindings::http::types::Host,
{
    bindings::http::outgoing_handler::add_to_linker(l, |t| t)?;
    bindings::http::types::add_to_linker(l, |t| t)?;

    Ok(())
}

pub mod sync {
    use crate::{bindings, WasiHttpView};
    use wasmtime_wasi::preview2;

    wasmtime::component::bindgen!({
        world: "wasi:http/proxy",
        tracing: true,
        async: false,
        with: {
            "wasi:cli/stderr": preview2::bindings::cli::stderr,
            "wasi:cli/stdin": preview2::bindings::cli::stdin,
            "wasi:cli/stdout": preview2::bindings::cli::stdout,
            "wasi:clocks/monotonic-clock": preview2::bindings::clocks::monotonic_clock,
            "wasi:clocks/timezone": preview2::bindings::clocks::timezone,
            "wasi:clocks/wall-clock": preview2::bindings::clocks::wall_clock,
            "wasi:http/incoming-handler": bindings::http::incoming_handler,
            "wasi:http/outgoing-handler": bindings::http::outgoing_handler,
            "wasi:http/types": bindings::http::types,
            "wasi:io/streams": preview2::bindings::io::streams,
            "wasi:poll/poll": preview2::bindings::poll::poll,
            "wasi:random/random": preview2::bindings::random::random,
        },
    });

    pub fn add_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
    where
        T: WasiHttpView + preview2::WasiView + bindings::http::types::Host,
    {
        // TODO: this shouldn't be required, but the adapter unconditionally pulls in all of these
        // dependencies.
        preview2::command::sync::add_to_linker(l)?;

        add_only_http_to_linker(l)?;

        Ok(())
    }

    #[doc(hidden)]
    // TODO: This is temporary solution until the preview2 command functions can be removed
    pub fn add_only_http_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
    where
        T: WasiHttpView + preview2::WasiView + bindings::http::types::Host,
    {
        bindings::http::outgoing_handler::add_to_linker(l, |t| t)?;
        bindings::http::types::add_to_linker(l, |t| t)?;

        Ok(())
    }
}

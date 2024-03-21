use crate::{bindings, WasiHttpView};

wasmtime::component::bindgen!({
    world: "wasi:http/proxy",
    tracing: true,
    async: true,
    with: {
        "wasi:http": bindings::http,
        "wasi": wasmtime_wasi::bindings,
    },
});

pub fn add_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView + wasmtime_wasi::WasiView + bindings::http::types::Host,
{
    wasmtime_wasi::bindings::clocks::wall_clock::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::clocks::monotonic_clock::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::io::poll::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::io::error::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::io::streams::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::cli::stdin::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::cli::stdout::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::cli::stderr::add_to_linker(l, |t| t)?;
    wasmtime_wasi::bindings::random::random::add_to_linker(l, |t| t)?;

    add_only_http_to_linker(l)
}

#[doc(hidden)]
pub fn add_only_http_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView + wasmtime_wasi::WasiView + bindings::http::types::Host,
{
    bindings::http::outgoing_handler::add_to_linker(l, |t| t)?;
    bindings::http::types::add_to_linker(l, |t| t)?;

    Ok(())
}

pub mod sync {
    use crate::{bindings, WasiHttpView};

    wasmtime::component::bindgen!({
        world: "wasi:http/proxy",
        tracing: true,
        async: false,
        with: {
            "wasi:http": bindings::http, // http is in this crate
            "wasi:io": wasmtime_wasi::bindings::sync_io, // io is sync
            "wasi": wasmtime_wasi::bindings, // everything else
        },
    });

    pub fn add_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
    where
        T: WasiHttpView + wasmtime_wasi::WasiView + bindings::http::types::Host,
    {
        // TODO: this shouldn't be required, but the adapter unconditionally pulls in all of these
        // dependencies.
        wasmtime_wasi::command::sync::add_to_linker(l)?;

        add_only_http_to_linker(l)?;

        Ok(())
    }

    #[doc(hidden)]
    // TODO: This is temporary solution until the wasmtime_wasi command functions can be removed
    pub fn add_only_http_to_linker<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
    where
        T: WasiHttpView + wasmtime_wasi::WasiView + bindings::http::types::Host,
    {
        bindings::http::outgoing_handler::add_to_linker(l, |t| t)?;
        bindings::http::types::add_to_linker(l, |t| t)?;

        Ok(())
    }
}

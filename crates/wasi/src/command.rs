use crate::WasiView;

wasmtime::component::bindgen!({
    world: "wasi:cli/command",
    tracing: true,
    async: true,
    with: { "wasi": crate::bindings },
});

pub fn add_to_linker<T: WasiView>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()> {
    crate::bindings::clocks::wall_clock::add_to_linker(l, |t| t)?;
    crate::bindings::clocks::monotonic_clock::add_to_linker(l, |t| t)?;
    crate::bindings::filesystem::types::add_to_linker(l, |t| t)?;
    crate::bindings::filesystem::preopens::add_to_linker(l, |t| t)?;
    crate::bindings::io::error::add_to_linker(l, |t| t)?;
    crate::bindings::io::poll::add_to_linker(l, |t| t)?;
    crate::bindings::io::streams::add_to_linker(l, |t| t)?;
    crate::bindings::random::random::add_to_linker(l, |t| t)?;
    crate::bindings::random::insecure::add_to_linker(l, |t| t)?;
    crate::bindings::random::insecure_seed::add_to_linker(l, |t| t)?;
    crate::bindings::cli::exit::add_to_linker(l, |t| t)?;
    crate::bindings::cli::environment::add_to_linker(l, |t| t)?;
    crate::bindings::cli::stdin::add_to_linker(l, |t| t)?;
    crate::bindings::cli::stdout::add_to_linker(l, |t| t)?;
    crate::bindings::cli::stderr::add_to_linker(l, |t| t)?;
    crate::bindings::cli::terminal_input::add_to_linker(l, |t| t)?;
    crate::bindings::cli::terminal_output::add_to_linker(l, |t| t)?;
    crate::bindings::cli::terminal_stdin::add_to_linker(l, |t| t)?;
    crate::bindings::cli::terminal_stdout::add_to_linker(l, |t| t)?;
    crate::bindings::cli::terminal_stderr::add_to_linker(l, |t| t)?;
    crate::bindings::sockets::tcp::add_to_linker(l, |t| t)?;
    crate::bindings::sockets::tcp_create_socket::add_to_linker(l, |t| t)?;
    crate::bindings::sockets::udp::add_to_linker(l, |t| t)?;
    crate::bindings::sockets::udp_create_socket::add_to_linker(l, |t| t)?;
    crate::bindings::sockets::instance_network::add_to_linker(l, |t| t)?;
    crate::bindings::sockets::network::add_to_linker(l, |t| t)?;
    crate::bindings::sockets::ip_name_lookup::add_to_linker(l, |t| t)?;
    Ok(())
}

pub mod sync {
    use crate::WasiView;

    wasmtime::component::bindgen!({
        world: "wasi:cli/command",
        tracing: true,
        async: false,
        with: {
            // Map interfaces with synchronous funtions to their synchronous
            // counterparts...
            "wasi:filesystem": crate::bindings::sync_io::filesystem,
            "wasi:io": crate::bindings::sync_io::io,

            // ... and everything else is not-async and so goes through the
            // top-level bindings.
            "wasi": crate::bindings
        },
    });

    pub fn add_to_linker<T: WasiView>(
        l: &mut wasmtime::component::Linker<T>,
    ) -> anyhow::Result<()> {
        crate::bindings::clocks::wall_clock::add_to_linker(l, |t| t)?;
        crate::bindings::clocks::monotonic_clock::add_to_linker(l, |t| t)?;
        crate::bindings::sync_io::filesystem::types::add_to_linker(l, |t| t)?;
        crate::bindings::filesystem::preopens::add_to_linker(l, |t| t)?;
        crate::bindings::io::error::add_to_linker(l, |t| t)?;
        crate::bindings::sync_io::io::poll::add_to_linker(l, |t| t)?;
        crate::bindings::sync_io::io::streams::add_to_linker(l, |t| t)?;
        crate::bindings::random::random::add_to_linker(l, |t| t)?;
        crate::bindings::random::insecure::add_to_linker(l, |t| t)?;
        crate::bindings::random::insecure_seed::add_to_linker(l, |t| t)?;
        crate::bindings::cli::exit::add_to_linker(l, |t| t)?;
        crate::bindings::cli::environment::add_to_linker(l, |t| t)?;
        crate::bindings::cli::stdin::add_to_linker(l, |t| t)?;
        crate::bindings::cli::stdout::add_to_linker(l, |t| t)?;
        crate::bindings::cli::stderr::add_to_linker(l, |t| t)?;
        crate::bindings::cli::terminal_input::add_to_linker(l, |t| t)?;
        crate::bindings::cli::terminal_output::add_to_linker(l, |t| t)?;
        crate::bindings::cli::terminal_stdin::add_to_linker(l, |t| t)?;
        crate::bindings::cli::terminal_stdout::add_to_linker(l, |t| t)?;
        crate::bindings::cli::terminal_stderr::add_to_linker(l, |t| t)?;
        crate::bindings::sockets::tcp::add_to_linker(l, |t| t)?;
        crate::bindings::sockets::tcp_create_socket::add_to_linker(l, |t| t)?;
        crate::bindings::sockets::udp::add_to_linker(l, |t| t)?;
        crate::bindings::sockets::udp_create_socket::add_to_linker(l, |t| t)?;
        crate::bindings::sockets::instance_network::add_to_linker(l, |t| t)?;
        crate::bindings::sockets::network::add_to_linker(l, |t| t)?;
        crate::bindings::sockets::ip_name_lookup::add_to_linker(l, |t| t)?;
        Ok(())
    }
}

use crate::WasiView;

wasmtime::component::bindgen!({
    world: "wasi:cli/command",
    tracing: true,
    async: true,
    with: {
       "wasi:filesystem/types": crate::bindings::filesystem::types,
       "wasi:filesystem/preopens": crate::bindings::filesystem::preopens,
       "wasi:sockets/tcp": crate::bindings::sockets::tcp,
       "wasi:clocks/monotonic_clock": crate::bindings::clocks::monotonic_clock,
       "wasi:io/poll": crate::bindings::io::poll,
       "wasi:io/streams": crate::bindings::io::streams,
       "wasi:clocks/wall_clock": crate::bindings::clocks::wall_clock,
       "wasi:random/random": crate::bindings::random::random,
       "wasi:cli/environment": crate::bindings::cli::environment,
       "wasi:cli/exit": crate::bindings::cli::exit,
       "wasi:cli/stdin": crate::bindings::cli::stdin,
       "wasi:cli/stdout": crate::bindings::cli::stdout,
       "wasi:cli/stderr": crate::bindings::cli::stderr,
       "wasi:cli/terminal-input": crate::bindings::cli::terminal_input,
       "wasi:cli/terminal-output": crate::bindings::cli::terminal_output,
       "wasi:cli/terminal-stdin": crate::bindings::cli::terminal_stdin,
       "wasi:cli/terminal-stdout": crate::bindings::cli::terminal_stdout,
       "wasi:cli/terminal-stderr": crate::bindings::cli::terminal_stderr,
    },
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
           "wasi:filesystem/types": crate::bindings::sync_io::filesystem::types,
           "wasi:filesystem/preopens": crate::bindings::filesystem::preopens,
           "wasi:sockets/tcp": crate::bindings::sockets::tcp,
           "wasi:sockets/udp": crate::bindings::sockets::udp,
           "wasi:clocks/monotonic_clock": crate::bindings::clocks::monotonic_clock,
           "wasi:io/poll": crate::bindings::sync_io::io::poll,
           "wasi:io/streams": crate::bindings::sync_io::io::streams,
           "wasi:clocks/wall_clock": crate::bindings::clocks::wall_clock,
           "wasi:random/random": crate::bindings::random::random,
           "wasi:cli/environment": crate::bindings::cli::environment,
           "wasi:cli/exit": crate::bindings::cli::exit,
           "wasi:cli/stdin": crate::bindings::cli::stdin,
           "wasi:cli/stdout": crate::bindings::cli::stdout,
           "wasi:cli/stderr": crate::bindings::cli::stderr,
           "wasi:cli/terminal-input": crate::bindings::cli::terminal_input,
           "wasi:cli/terminal-output": crate::bindings::cli::terminal_output,
           "wasi:cli/terminal-stdin": crate::bindings::cli::terminal_stdin,
           "wasi:cli/terminal-stdout": crate::bindings::cli::terminal_stdout,
           "wasi:cli/terminal-stderr": crate::bindings::cli::terminal_stderr,
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

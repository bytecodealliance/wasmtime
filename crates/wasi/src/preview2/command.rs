use crate::preview2::WasiView;

wasmtime::component::bindgen!({
    world: "wasi:cli/command",
    tracing: true,
    async: true,
    with: {
       "wasi:filesystem/types": crate::preview2::bindings::filesystem::types,
       "wasi:filesystem/preopens": crate::preview2::bindings::filesystem::preopens,
       "wasi:sockets/tcp": crate::preview2::bindings::sockets::tcp,
       "wasi:clocks/monotonic_clock": crate::preview2::bindings::clocks::monotonic_clock,
       "wasi:io/poll": crate::preview2::bindings::io::poll,
       "wasi:io/streams": crate::preview2::bindings::io::streams,
       "wasi:clocks/wall_clock": crate::preview2::bindings::clocks::wall_clock,
       "wasi:random/random": crate::preview2::bindings::random::random,
       "wasi:cli/environment": crate::preview2::bindings::cli::environment,
       "wasi:cli/exit": crate::preview2::bindings::cli::exit,
       "wasi:cli/stdin": crate::preview2::bindings::cli::stdin,
       "wasi:cli/stdout": crate::preview2::bindings::cli::stdout,
       "wasi:cli/stderr": crate::preview2::bindings::cli::stderr,
       "wasi:cli/terminal-input": crate::preview2::bindings::cli::terminal_input,
       "wasi:cli/terminal-output": crate::preview2::bindings::cli::terminal_output,
       "wasi:cli/terminal-stdin": crate::preview2::bindings::cli::terminal_stdin,
       "wasi:cli/terminal-stdout": crate::preview2::bindings::cli::terminal_stdout,
       "wasi:cli/terminal-stderr": crate::preview2::bindings::cli::terminal_stderr,
    },
});

pub fn add_to_linker<T: WasiView>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()> {
    crate::preview2::bindings::clocks::wall_clock::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::clocks::monotonic_clock::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::filesystem::types::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::filesystem::preopens::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::io::error::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::io::poll::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::io::streams::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::random::random::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::random::insecure::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::random::insecure_seed::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::cli::exit::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::cli::environment::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::cli::stdin::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::cli::stdout::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::cli::stderr::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::cli::terminal_input::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::cli::terminal_output::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::cli::terminal_stdin::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::cli::terminal_stdout::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::cli::terminal_stderr::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::sockets::tcp::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::sockets::tcp_create_socket::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::sockets::udp::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::sockets::udp_create_socket::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::sockets::instance_network::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::sockets::network::add_to_linker(l, |t| t)?;
    crate::preview2::bindings::sockets::ip_name_lookup::add_to_linker(l, |t| t)?;
    Ok(())
}

pub mod sync {
    use crate::preview2::WasiView;

    wasmtime::component::bindgen!({
        world: "wasi:cli/command",
        tracing: true,
        async: false,
        with: {
           "wasi:filesystem/types": crate::preview2::bindings::sync_io::filesystem::types,
           "wasi:filesystem/preopens": crate::preview2::bindings::filesystem::preopens,
           "wasi:sockets/tcp": crate::preview2::bindings::sockets::tcp,
           "wasi:sockets/udp": crate::preview2::bindings::sockets::udp,
           "wasi:clocks/monotonic_clock": crate::preview2::bindings::clocks::monotonic_clock,
           "wasi:io/poll": crate::preview2::bindings::sync_io::io::poll,
           "wasi:io/streams": crate::preview2::bindings::sync_io::io::streams,
           "wasi:clocks/wall_clock": crate::preview2::bindings::clocks::wall_clock,
           "wasi:random/random": crate::preview2::bindings::random::random,
           "wasi:cli/environment": crate::preview2::bindings::cli::environment,
           "wasi:cli/exit": crate::preview2::bindings::cli::exit,
           "wasi:cli/stdin": crate::preview2::bindings::cli::stdin,
           "wasi:cli/stdout": crate::preview2::bindings::cli::stdout,
           "wasi:cli/stderr": crate::preview2::bindings::cli::stderr,
           "wasi:cli/terminal-input": crate::preview2::bindings::cli::terminal_input,
           "wasi:cli/terminal-output": crate::preview2::bindings::cli::terminal_output,
           "wasi:cli/terminal-stdin": crate::preview2::bindings::cli::terminal_stdin,
           "wasi:cli/terminal-stdout": crate::preview2::bindings::cli::terminal_stdout,
           "wasi:cli/terminal-stderr": crate::preview2::bindings::cli::terminal_stderr,
        },
    });

    pub fn add_to_linker<T: WasiView>(
        l: &mut wasmtime::component::Linker<T>,
    ) -> anyhow::Result<()> {
        crate::preview2::bindings::clocks::wall_clock::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::clocks::monotonic_clock::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::sync_io::filesystem::types::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::filesystem::preopens::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::io::error::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::sync_io::io::poll::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::sync_io::io::streams::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::random::random::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::random::insecure::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::random::insecure_seed::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::cli::exit::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::cli::environment::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::cli::stdin::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::cli::stdout::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::cli::stderr::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::cli::terminal_input::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::cli::terminal_output::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::cli::terminal_stdin::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::cli::terminal_stdout::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::cli::terminal_stderr::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::sockets::tcp::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::sockets::tcp_create_socket::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::sockets::udp::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::sockets::udp_create_socket::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::sockets::instance_network::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::sockets::network::add_to_linker(l, |t| t)?;
        crate::preview2::bindings::sockets::ip_name_lookup::add_to_linker(l, |t| t)?;
        Ok(())
    }
}

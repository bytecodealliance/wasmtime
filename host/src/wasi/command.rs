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
       "instance_monotonic_clock": crate::wasi::instance_monotonic_clock,
       "instance_network": crate::wasi::instance_network,
       "instance_wall_clock": crate::wasi::instance_wall_clock,
       "ip_name_lookup": crate::wasi::ip_name_lookup,
       "monotonic_clock": crate::wasi::monotonic_clock,
       "network": crate::wasi::network,
       "poll": crate::wasi::poll,
       "streams": crate::wasi::streams,
       "tcp": crate::wasi::tcp,
       "tcp_create_socket": crate::wasi::tcp_create_socket,
       "timezone": crate::wasi::timezone,
       "udp": crate::wasi::udp,
       "udp_create_socket": crate::wasi::udp_create_socket,
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
    crate::wasi::tcp::add_to_linker(l, f)?;
    crate::wasi::tcp_create_socket::add_to_linker(l, f)?;
    crate::wasi::udp::add_to_linker(l, f)?;
    crate::wasi::udp_create_socket::add_to_linker(l, f)?;
    crate::wasi::ip_name_lookup::add_to_linker(l, f)?;
    crate::wasi::instance_network::add_to_linker(l, f)?;
    crate::wasi::network::add_to_linker(l, f)?;
    crate::wasi::exit::add_to_linker(l, f)?;
    crate::wasi::environment::add_to_linker(l, f)?;
    crate::wasi::preopens::add_to_linker(l, f)?;
    Ok(())
}

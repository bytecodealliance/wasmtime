use crate::WasiCtx;

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
    crate::wasi::random::add_to_linker(l, f)?;
    crate::wasi::console::add_to_linker(l, f)?;
    crate::wasi::types::add_to_linker(l, f)?;
    crate::wasi::poll::add_to_linker(l, f)?;
    crate::wasi::streams::add_to_linker(l, f)?;
    crate::wasi::default_outgoing_http::add_to_linker(l, f)?;
    Ok(())
}

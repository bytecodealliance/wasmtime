use core::net::SocketAddr;

use wasmtime::component::Accessor;

use crate::p3::bindings::sockets::types::{Host, HostConcurrent};
use crate::p3::sockets::WasiSockets;
use crate::sockets::{SocketAddrCheck, SocketAddrUse, WasiSocketsCtxView};

mod tcp;
mod udp;

impl Host for WasiSocketsCtxView<'_> {}

impl HostConcurrent for WasiSockets {}

fn get_socket_addr_check<T>(store: &Accessor<T, WasiSockets>) -> SocketAddrCheck {
    store.with(|mut view| view.get().ctx.socket_addr_check.clone())
}

async fn is_addr_allowed<T>(
    store: &Accessor<T, WasiSockets>,
    addr: SocketAddr,
    reason: SocketAddrUse,
) -> bool {
    get_socket_addr_check(store)(addr, reason).await
}

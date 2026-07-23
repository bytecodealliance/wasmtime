use crate::p3::bindings::sockets::ip_name_lookup::{ErrorCode, Host, HostWithStore};
use crate::p3::bindings::sockets::types;
use crate::p3::sockets::WasiSockets;
use crate::sockets::WasiSocketsCtxView;
use crate::sockets::ip_name_lookup::resolve_addresses;
use wasmtime::component::Accessor;

impl<U> HostWithStore<U> for WasiSockets {
    async fn resolve_addresses(
        store: &Accessor<U, Self>,
        name: String,
    ) -> wasmtime::Result<Result<Vec<types::IpAddress>, ErrorCode>> {
        let fut = store.with(|mut view| resolve_addresses(&view.get().ctx, name));
        Ok(match fut.await {
            Ok(addrs) => Ok(addrs.into_iter().map(|addr| addr.into()).collect()),
            Err(err) => Err(err.into()),
        })
    }
}

impl Host for WasiSocketsCtxView<'_> {}

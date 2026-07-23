use crate::p2::SocketError;
use crate::p2::bindings::sockets::ip_name_lookup::{Host, HostResolveAddressStream};
use crate::p2::bindings::sockets::network::{ErrorCode, IpAddress, Network};
use crate::sockets::ip_name_lookup::resolve_addresses;
use crate::sockets::{MaybeReady, WasiSocketsCtxView, noop_cx};
use std::net::IpAddr;
use std::task::Poll;
use std::vec;
use wasmtime::Result;
use wasmtime::component::Resource;
use wasmtime_wasi_io::poll::{DynPollable, Pollable, subscribe};

pub struct ResolveAddressStream(MaybeReady<Result<vec::IntoIter<IpAddr>, ErrorCode>>);

impl Host for WasiSocketsCtxView<'_> {
    fn resolve_addresses(
        &mut self,
        network: Resource<Network>,
        name: String,
    ) -> Result<Resource<ResolveAddressStream>, SocketError> {
        // The network resource itself represents the capability to use this
        // method, so we need to check its validity. Other than that, we have no
        // use for it.
        _ = self.table.get(&network)?;

        let fut = resolve_addresses(&self.ctx, name);
        let stream = ResolveAddressStream(MaybeReady::poll_or_spawn(async move {
            Ok(fut.await?.into_iter())
        }));

        // Attempt to surface errors immediately.
        if let MaybeReady::Ready(Err(err)) = &stream.0 {
            return Err((*err).into());
        }
        Ok(self.table.push(stream)?)
    }
}

impl HostResolveAddressStream for WasiSocketsCtxView<'_> {
    fn resolve_next_address(
        &mut self,
        resource: Resource<ResolveAddressStream>,
    ) -> Result<Option<IpAddress>, SocketError> {
        let stream: &mut ResolveAddressStream = self.table.get_mut(&resource)?;
        let Poll::Ready(result) = stream.0.poll_ready(&mut noop_cx()) else {
            return Err(ErrorCode::WouldBlock.into());
        };

        match result {
            Ok(iter) => Ok(iter.next().map(|addr| addr.into())),
            Err(err) => Err((*err).into()),
        }
    }

    fn subscribe(
        &mut self,
        resource: Resource<ResolveAddressStream>,
    ) -> Result<Resource<DynPollable>> {
        subscribe(self.table, resource)
    }

    fn drop(&mut self, resource: Resource<ResolveAddressStream>) -> Result<()> {
        self.table.delete(resource)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Pollable for ResolveAddressStream {
    async fn ready(&mut self) {
        std::future::poll_fn(|cx| self.0.poll_ready(cx).map(|_| ())).await
    }
}

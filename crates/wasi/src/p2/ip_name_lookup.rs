use crate::p2::SocketError;
use crate::p2::bindings::sockets::ip_name_lookup::{Host, HostResolveAddressStream};
use crate::p2::bindings::sockets::network::{ErrorCode, IpAddress, Network};
use crate::runtime::poll_now;
use crate::sockets::ip_name_lookup::resolve_addresses;
use crate::sockets::{MaybeSpawned, WasiSocketsCtxView};
use std::net::IpAddr;
use std::vec;
use wasmtime::Result;
use wasmtime::component::Resource;
use wasmtime_wasi_io::poll::{DynPollable, Pollable, subscribe};

pub struct ResolveAddressStream(MaybeSpawned<Result<vec::IntoIter<IpAddr>, ErrorCode>>);

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
        let stream = ResolveAddressStream(MaybeSpawned::poll_or_spawn(async move {
            Ok(fut.await?.into_iter())
        }));

        // Attempt to surface errors immediately.
        if let MaybeSpawned::Ready(Err(err)) = &stream.0 {
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
        let Some(result) = poll_now(|cx| stream.0.poll_ready(cx)) else {
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

    async fn drop(&mut self, resource: Resource<ResolveAddressStream>) -> Result<()> {
        let stream = self.table.delete(resource)?;
        if let MaybeSpawned::Pending(fut) = stream.0 {
            fut.cancel().await;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Pollable for ResolveAddressStream {
    async fn ready(&mut self) {
        std::future::poll_fn(|cx| self.0.poll_ready(cx).map(|_| ())).await
    }
}

mod sync {
    use super::ResolveAddressStream;
    use crate::p2::SocketError;
    use crate::p2::bindings::sockets::network::{IpAddress, Network};
    use crate::p2::bindings::sync::sockets::ip_name_lookup::{Host, HostResolveAddressStream};
    use crate::runtime::in_tokio;
    use crate::sockets::WasiSocketsCtxView;
    use wasmtime::Result;
    use wasmtime::component::Resource;
    use wasmtime_wasi_io::poll::DynPollable;

    impl Host for WasiSocketsCtxView<'_> {
        fn resolve_addresses(
            &mut self,
            network: Resource<Network>,
            name: String,
        ) -> Result<Resource<ResolveAddressStream>, SocketError> {
            <Self as super::Host>::resolve_addresses(self, network, name)
        }
    }

    impl HostResolveAddressStream for WasiSocketsCtxView<'_> {
        fn resolve_next_address(
            &mut self,
            resource: Resource<ResolveAddressStream>,
        ) -> Result<Option<IpAddress>, SocketError> {
            <Self as super::HostResolveAddressStream>::resolve_next_address(self, resource)
        }

        fn subscribe(
            &mut self,
            resource: Resource<ResolveAddressStream>,
        ) -> Result<Resource<DynPollable>> {
            <Self as super::HostResolveAddressStream>::subscribe(self, resource)
        }

        fn drop(&mut self, resource: Resource<ResolveAddressStream>) -> Result<()> {
            in_tokio(<Self as super::HostResolveAddressStream>::drop(
                self, resource,
            ))
        }
    }
}

mod named {
    use crate::p2::SocketError;
    use crate::p2::bindings::named_imports::wasi::sockets::ip_name_lookup;
    use crate::p2::bindings::sockets::network::ErrorCode;
    use crate::p2::bindings::sockets::network::{IpAddress, Network};
    use crate::p2::ip_name_lookup::ResolveAddressStream;
    use crate::sockets::WasiSocketsNamedView;
    use crate::{NamedId, WasiCtxNamedView};
    use wasmtime::Result;
    use wasmtime::component::Resource;
    use wasmtime_wasi_io::poll::DynPollable;

    impl<T> ip_name_lookup::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiSocketsNamedView,
    {
        fn convert_error_code(&mut self, err: SocketError) -> wasmtime::Result<ErrorCode> {
            err.downcast()
        }

        fn resolve_addresses(
            &mut self,
            id: NamedId,
            network: Resource<Network>,
            name: String,
        ) -> Result<Resource<ResolveAddressStream>, SocketError> {
            super::Host::resolve_addresses(&mut self.0.sockets(id), network, name)
        }
    }

    impl<T> ip_name_lookup::HostResolveAddressStream for WasiCtxNamedView<'_, T>
    where
        T: WasiSocketsNamedView,
    {
        fn resolve_next_address(
            &mut self,
            id: NamedId,
            resource: Resource<ResolveAddressStream>,
        ) -> Result<Option<IpAddress>, SocketError> {
            super::HostResolveAddressStream::resolve_next_address(&mut self.0.sockets(id), resource)
        }

        fn subscribe(
            &mut self,
            id: NamedId,
            resource: Resource<ResolveAddressStream>,
        ) -> Result<Resource<DynPollable>> {
            super::HostResolveAddressStream::subscribe(&mut self.0.sockets(id), resource)
        }

        async fn drop(
            &mut self,
            id: NamedId,
            resource: Resource<ResolveAddressStream>,
        ) -> Result<()> {
            super::HostResolveAddressStream::drop(&mut self.0.sockets(id), resource).await
        }
    }
}

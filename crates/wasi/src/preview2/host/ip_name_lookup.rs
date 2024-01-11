use crate::preview2::bindings::sockets::ip_name_lookup::{Host, HostResolveAddressStream};
use crate::preview2::bindings::sockets::network::{ErrorCode, IpAddress, Network};
use crate::preview2::poll::{subscribe, Pollable, Subscribe};
use crate::preview2::{DynFuture, SocketError, WasiView};
use anyhow::Result;
use std::io;
use std::net::IpAddr;
use std::vec;
use wasmtime::component::Resource;

pub enum ResolveAddressStream {
    Waiting(DynFuture<io::Result<Vec<IpAddr>>>),
    Iterating(vec::IntoIter<IpAddr>),
    Done,
}

#[async_trait::async_trait]
impl<T: WasiView + Sized> Host for T {
    fn resolve_addresses(
        &mut self,
        network: Resource<Network>,
        name: String,
    ) -> Result<Resource<ResolveAddressStream>, SocketError> {
        // At the moment, there's only one network handle (`instance-network`)
        // in existence. All we have to do here is validate that the caller indeed
        // has possesion of a valid handle and then we're good to go:
        let _network = self.table().get(&network)?;

        let mut future = self.ctx_mut().network.resolve_addresses(name);
        let stream = match future.try_resolve() {
            None => ResolveAddressStream::Waiting(future),
            Some(Ok(addresses)) => ResolveAddressStream::Iterating(addresses.into_iter()),
            Some(Err(e)) => return Err(e.into()),
        };

        let resource = self.table_mut().push(stream)?;
        Ok(resource)
    }
}

#[async_trait::async_trait]
impl<T: WasiView> HostResolveAddressStream for T {
    fn resolve_next_address(
        &mut self,
        resource: Resource<ResolveAddressStream>,
    ) -> Result<Option<IpAddress>, SocketError> {
        let stream: &mut ResolveAddressStream = self.table_mut().get_mut(&resource)?;

        if let ResolveAddressStream::Waiting(future) = stream {
            match future.try_resolve() {
                None => return Err(ErrorCode::WouldBlock.into()),
                Some(Ok(addresses)) => {
                    *stream = ResolveAddressStream::Iterating(addresses.into_iter());
                    // Fall through to if-statements below.
                }
                Some(Err(e)) => {
                    *stream = ResolveAddressStream::Done;
                    return Err(e.into());
                }
            }
        }

        if let ResolveAddressStream::Iterating(iter) = stream {
            match iter.next() {
                Some(address) => return Ok(Some(address.into())),
                None => {
                    *stream = ResolveAddressStream::Done;
                    // Fall through to if-statement below.
                }
            }
        }

        if let ResolveAddressStream::Done = stream {
            return Ok(None);
        }

        unreachable!()
    }

    fn subscribe(
        &mut self,
        resource: Resource<ResolveAddressStream>,
    ) -> Result<Resource<Pollable>> {
        subscribe(self.table_mut(), resource)
    }

    fn drop(&mut self, resource: Resource<ResolveAddressStream>) -> Result<()> {
        self.table_mut().delete(resource)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Subscribe for ResolveAddressStream {
    async fn ready(&mut self) {
        match self {
            Self::Waiting(future) => future.wait().await,
            Self::Iterating(_) | Self::Done => {}
        }
    }
}

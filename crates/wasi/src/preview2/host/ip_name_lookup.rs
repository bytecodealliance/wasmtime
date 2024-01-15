use crate::preview2::bindings::sockets::ip_name_lookup::{Host, HostResolveAddressStream};
use crate::preview2::bindings::sockets::network::{ErrorCode, IpAddress, Network};
use crate::preview2::poll::{subscribe, Pollable, Subscribe};
use crate::preview2::{Preview2Future, SocketError, WasiView};
use anyhow::Result;
use std::io;
use std::net::IpAddr;
use std::vec;
use wasmtime::component::Resource;

pub enum ResolveAddressStreamResource {
    Waiting(Preview2Future<io::Result<Vec<IpAddr>>>),
    Iterating(vec::IntoIter<IpAddr>),
    Done,
}

#[async_trait::async_trait]
impl<T: WasiView + Sized> Host for T {
    fn resolve_addresses(
        &mut self,
        network: Resource<Network>,
        name: String,
    ) -> Result<Resource<ResolveAddressStreamResource>, SocketError> {
        self.table().get(&network)?.check_access()?;

        let mut future = Preview2Future::new(self.ctx_mut().network.resolve_addresses(name));
        // Attempt to eagerly return errors:
        let stream = match future.try_resolve() {
            None => ResolveAddressStreamResource::Waiting(future),
            Some(Ok(addresses)) => ResolveAddressStreamResource::Iterating(addresses.into_iter()),
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
        resource: Resource<ResolveAddressStreamResource>,
    ) -> Result<Option<IpAddress>, SocketError> {
        let stream: &mut ResolveAddressStreamResource = self.table_mut().get_mut(&resource)?;

        if let ResolveAddressStreamResource::Waiting(future) = stream {
            match future.try_resolve() {
                None => return Err(ErrorCode::WouldBlock.into()),
                Some(Ok(addresses)) => {
                    *stream = ResolveAddressStreamResource::Iterating(addresses.into_iter());
                    // Fall through to if-statements below.
                }
                Some(Err(e)) => {
                    *stream = ResolveAddressStreamResource::Done;
                    return Err(e.into());
                }
            }
        }

        if let ResolveAddressStreamResource::Iterating(iter) = stream {
            match iter.next() {
                Some(address) => return Ok(Some(address.into())),
                None => {
                    *stream = ResolveAddressStreamResource::Done;
                    // Fall through to if-statement below.
                }
            }
        }

        if let ResolveAddressStreamResource::Done = stream {
            return Ok(None);
        }

        unreachable!()
    }

    fn subscribe(
        &mut self,
        resource: Resource<ResolveAddressStreamResource>,
    ) -> Result<Resource<Pollable>> {
        subscribe(self.table_mut(), resource)
    }

    fn drop(&mut self, resource: Resource<ResolveAddressStreamResource>) -> Result<()> {
        self.table_mut().delete(resource)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Subscribe for ResolveAddressStreamResource {
    async fn ready(&mut self) {
        match self {
            Self::Waiting(future) => future.ready().await,
            Self::Iterating(_) | Self::Done => {}
        }
    }
}

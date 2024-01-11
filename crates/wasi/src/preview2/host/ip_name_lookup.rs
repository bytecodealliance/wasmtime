use crate::preview2::bindings::sockets::ip_name_lookup::{Host, HostResolveAddressStream};
use crate::preview2::bindings::sockets::network::{ErrorCode, IpAddress, Network};
use crate::preview2::poll::{subscribe, Pollable, Subscribe};
use crate::preview2::{SocketError, WasiView};
use anyhow::Result;
use std::future::Future;
use std::mem;
use std::net::IpAddr;
use std::pin::Pin;
use std::vec;
use wasmtime::component::Resource;

pub enum ResolveAddressStream {
    Waiting(Pin<Box<dyn Future<Output = std::io::Result<Vec<IpAddr>>> + Send + Sync>>),
    Done(std::io::Result<vec::IntoIter<IpAddr>>),
}

impl ResolveAddressStream {
    pub fn wait(
        future: impl Future<Output = std::io::Result<Vec<IpAddr>>> + Send + Sync + 'static,
    ) -> Self {
        Self::Waiting(Box::pin(future))
    }

    pub fn done(result: std::io::Result<Vec<IpAddr>>) -> Self {
        Self::Done(result.map(|v| v.into_iter()))
    }
}

#[async_trait::async_trait]
impl<T: WasiView + Sized> Host for T {
    fn resolve_addresses(
        &mut self,
        network: Resource<Network>,
        name: String,
    ) -> Result<Resource<ResolveAddressStream>, SocketError> {
        // Check that the network resource is valid
        let _network = self.table().get(&network)?;
        let stream = self.ctx_mut().network.resolve_addresses(name);
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
        let stream = self.table_mut().get_mut(&resource)?;
        loop {
            match stream {
                ResolveAddressStream::Waiting(future) => {
                    let result = crate::preview2::poll_noop(Pin::new(&mut *future));
                    match result {
                        Some(result) => {
                            *stream = ResolveAddressStream::Done(result.map(|v| v.into_iter()));
                        }
                        None => return Err(ErrorCode::WouldBlock.into()),
                    };
                }
                ResolveAddressStream::Done(slot @ Err(_)) => {
                    mem::replace(slot, Ok(Vec::new().into_iter()))?;
                    unreachable!();
                }
                ResolveAddressStream::Done(Ok(iter)) => return Ok(iter.next().map(|v| v.into())),
            }
        }
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
        if let ResolveAddressStream::Waiting(future) = self {
            let result = future.await.map(|v| v.into_iter());
            *self = ResolveAddressStream::Done(result);
        }
    }
}

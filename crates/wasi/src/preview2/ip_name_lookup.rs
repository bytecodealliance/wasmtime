use crate::preview2::bindings::sockets::ip_name_lookup::{Host, HostResolveAddressStream};
use crate::preview2::bindings::sockets::network::{ErrorCode, IpAddress, Network};
use crate::preview2::host::network::util;
use crate::preview2::poll::{subscribe, Pollable, Subscribe};
use crate::preview2::{spawn, spawn_blocking, AbortOnDropJoinHandle, SocketError, WasiView};
use anyhow::Result;
use std::mem;
use std::net::{IpAddr, Ipv6Addr, ToSocketAddrs};
use std::pin::Pin;
use std::str::FromStr;
use std::vec;
use wasmtime::component::Resource;

pub enum ResolveAddressStream {
    Waiting(AbortOnDropJoinHandle<std::io::Result<Vec<IpAddr>>>),
    Done(std::io::Result<vec::IntoIter<IpAddr>>),
}

/// A trait for providing a `IpNameLookup` implementation.
pub trait WasiIpNameLookupView: WasiView {
    /// The type that implements `IpNameLookup`.
    type IpNameLookup: IpNameLookup + Send + 'static;

    /// Get a new instance of the name lookup.
    fn ip_name_lookup(&self) -> Self::IpNameLookup;
}

/// A trait for resolving IP addresses from a name.
#[async_trait::async_trait]
pub trait IpNameLookup {
    /// Given a name, resolve to a list of IP addresses
    async fn resolve_addresses(&mut self, name: String) -> std::io::Result<Vec<IpAddr>>;
}

/// The default implementation for `WasiIpNameLookupView`.
pub struct SystemIpNameLookup {
    _priv: (),
}

impl SystemIpNameLookup {
    /// Create a new `SystemIpNameLookup`
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

#[async_trait::async_trait]
impl IpNameLookup for SystemIpNameLookup {
    async fn resolve_addresses(&mut self, name: String) -> std::io::Result<Vec<IpAddr>> {
        let host = parse(&name)?;

        spawn_blocking(move || blocking_resolve(&host)).await
    }
}

#[async_trait::async_trait]
impl<T: WasiIpNameLookupView + Sized> Host for T {
    fn resolve_addresses(
        &mut self,
        network: Resource<Network>,
        name: String,
    ) -> Result<Resource<ResolveAddressStream>, SocketError> {
        let network = self.table().get(&network)?;

        if !network.allow_ip_name_lookup {
            return Err(ErrorCode::PermanentResolverFailure.into());
        }
        let mut lookup = self.ip_name_lookup();
        let task = spawn(async move { lookup.resolve_addresses(name).await });

        let resource = self.table_mut().push(ResolveAddressStream::Waiting(task))?;
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
                    let result = crate::preview2::poll_noop(Pin::new(future));
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

fn parse(name: &str) -> std::io::Result<url::Host> {
    // `url::Host::parse` serves us two functions:
    // 1. validate the input is a valid domain name or IP,
    // 2. convert unicode domains to punycode.
    match url::Host::parse(&name) {
        Ok(host) => Ok(host),

        // `url::Host::parse` doesn't understand bare IPv6 addresses without [brackets]
        Err(_) => {
            if let Ok(addr) = Ipv6Addr::from_str(name) {
                Ok(url::Host::Ipv6(addr))
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "invalid domain name",
                ))
            }
        }
    }
}

fn blocking_resolve(host: &url::Host) -> std::io::Result<Vec<IpAddr>> {
    match host {
        url::Host::Ipv4(v4addr) => Ok(vec![IpAddr::V4(*v4addr)]),
        url::Host::Ipv6(v6addr) => Ok(vec![IpAddr::V6(*v6addr)]),
        url::Host::Domain(domain) => {
            // For now use the standard library to perform actual resolution through
            // the usage of the `ToSocketAddrs` trait. This is only
            // resolving names, not ports, so force the port to be 0.
            let addresses = (domain.as_str(), 0)
                .to_socket_addrs()?
                .map(|addr| util::to_canonical(&addr.ip()).into())
                .collect();

            Ok(addresses)
        }
    }
}

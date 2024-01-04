use crate::preview2::bindings::sockets::ip_name_lookup::{Host, HostResolveAddressStream};
use crate::preview2::bindings::sockets::network::{ErrorCode, IpAddress, Network};
use crate::preview2::host::network::util;
use crate::preview2::poll::{subscribe, Pollable, Subscribe};
use crate::preview2::{spawn_blocking, AbortOnDropJoinHandle, SocketError, WasiView};
use anyhow::Result;
use std::mem;
use std::net::{IpAddr, Ipv6Addr, ToSocketAddrs};
use std::pin::Pin;
use std::str::FromStr;
use std::vec;
use wasmtime::component::Resource;

use super::WasiCtx;

pub enum ResolveAddressStream {
    Waiting(AbortOnDropJoinHandle<std::io::Result<Vec<IpAddr>>>),
    Done(std::io::Result<vec::IntoIter<IpAddr>>),
}

/// A trait for providing a `IpNameLookup` implementation.
pub trait WasiNetworkView: Send {
    /// Given a name, resolve to a list of IP addresses
    fn resolve_addresses(
        &mut self,
        name: String,
    ) -> AbortOnDropJoinHandle<std::io::Result<Vec<IpAddr>>>;
}

/// The default implementation for `WasiIpNameLookupView`.
#[derive(Debug, Clone, Default)]
pub struct SystemNetwork {
    allowed: bool,
}

impl SystemNetwork {
    /// Create a new `SystemIpNameLookup`
    pub fn new(ctx: &WasiCtx) -> Self {
        Self {
            allowed: ctx.allowed_network_uses.ip_name_lookup,
        }
    }
}

impl WasiNetworkView for SystemNetwork {
    fn resolve_addresses(
        &mut self,
        name: String,
    ) -> AbortOnDropJoinHandle<std::io::Result<Vec<IpAddr>>> {
        let allowed = self.allowed;

        spawn_blocking(move || {
            if !allowed {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "IP name lookup is not allowed",
                ));
            }
            let host = parse(&name)?;
            blocking_resolve(&host)
        })
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
        let task = self.network_view_mut().resolve_addresses(name);
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

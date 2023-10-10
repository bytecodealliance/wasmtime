use crate::preview2::bindings::sockets::ip_name_lookup::{Host, HostResolveAddressStream};
use crate::preview2::bindings::sockets::network::{ErrorCode, IpAddress, IpAddressFamily, Network};
use crate::preview2::poll::{subscribe, Pollable, Subscribe};
use crate::preview2::{spawn_blocking, AbortOnDropJoinHandle, SocketError, WasiView};
use anyhow::Result;
use std::mem;
use std::net::{SocketAddr, ToSocketAddrs};
use std::pin::Pin;
use std::vec;
use wasmtime::component::Resource;

pub enum ResolveAddressStream {
    Waiting(AbortOnDropJoinHandle<Result<Vec<IpAddress>, SocketError>>),
    Done(Result<vec::IntoIter<IpAddress>, SocketError>),
}

#[async_trait::async_trait]
impl<T: WasiView> Host for T {
    fn resolve_addresses(
        &mut self,
        network: Resource<Network>,
        name: String,
        family: Option<IpAddressFamily>,
        include_unavailable: bool,
    ) -> Result<Resource<ResolveAddressStream>, SocketError> {
        let network = self.table().get(&network)?;

        // `Host::parse` serves us two functions:
        // 1. validate the input is not an IP address,
        // 2. convert unicode domains to punycode.
        let name = match url::Host::parse(&name).map_err(|_| ErrorCode::InvalidArgument)? {
            url::Host::Domain(name) => name,
            url::Host::Ipv4(_) => return Err(ErrorCode::InvalidArgument.into()),
            url::Host::Ipv6(_) => return Err(ErrorCode::InvalidArgument.into()),
        };

        if !network.allow_ip_name_lookup {
            return Err(ErrorCode::PermanentResolverFailure.into());
        }

        // ignored for now, should probably have a future PR to actually take
        // this into account. This would require invoking `getaddrinfo` directly
        // rather than using the standard library to do it for us.
        let _ = include_unavailable;

        // For now use the standard library to perform actual resolution through
        // the usage of the `ToSocketAddrs` trait. This blocks the current
        // thread, so use `spawn_blocking`. Finally note that this is only
        // resolving names, not ports, so force the port to be 0.
        let task = spawn_blocking(move || -> Result<Vec<_>, SocketError> {
            let result = (name.as_str(), 0)
                .to_socket_addrs()
                .map_err(|_| ErrorCode::NameUnresolvable)?; // If/when we use `getaddrinfo` directly, map the error properly.
            Ok(result
                .filter_map(|addr| {
                    // In lieu of preventing these addresses from being resolved
                    // in the first place, filter them out here.
                    match addr {
                        SocketAddr::V4(addr) => match family {
                            None | Some(IpAddressFamily::Ipv4) => {
                                let [a, b, c, d] = addr.ip().octets();
                                Some(IpAddress::Ipv4((a, b, c, d)))
                            }
                            Some(IpAddressFamily::Ipv6) => None,
                        },
                        SocketAddr::V6(addr) => match family {
                            None | Some(IpAddressFamily::Ipv6) => {
                                let [a, b, c, d, e, f, g, h] = addr.ip().segments();
                                Some(IpAddress::Ipv6((a, b, c, d, e, f, g, h)))
                            }
                            Some(IpAddressFamily::Ipv4) => None,
                        },
                    }
                })
                .collect())
        });
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
                    match crate::preview2::poll_noop(Pin::new(future)) {
                        Some(result) => {
                            *stream = ResolveAddressStream::Done(result.map(|v| v.into_iter()));
                        }
                        None => return Err(ErrorCode::WouldBlock.into()),
                    }
                }
                ResolveAddressStream::Done(slot @ Err(_)) => {
                    mem::replace(slot, Ok(Vec::new().into_iter()))?;
                    unreachable!();
                }
                ResolveAddressStream::Done(Ok(iter)) => return Ok(iter.next()),
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
            *self = ResolveAddressStream::Done(future.await.map(|v| v.into_iter()));
        }
    }
}

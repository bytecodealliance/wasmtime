use crate::preview2::host::network::util;
use crate::preview2::DynFuture;
use std::net::{IpAddr, Ipv6Addr, ToSocketAddrs};
use std::str::FromStr;
use std::{io, vec};

pub(crate) fn resolve_addresses(name: &str) -> DynFuture<io::Result<Vec<IpAddr>>> {
    match parse(name) {
        Err(e) => DynFuture::ready(Err(e)),
        Ok(url::Host::Ipv4(v4addr)) => DynFuture::ready(Ok(vec![IpAddr::V4(v4addr)])),
        Ok(url::Host::Ipv6(v6addr)) => DynFuture::ready(Ok(vec![IpAddr::V6(v6addr)])),
        Ok(url::Host::Domain(domain)) => {
            DynFuture::boxed(super::spawn_blocking(move || blocking_resolve(&domain)))
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

fn blocking_resolve(domain: &str) -> std::io::Result<Vec<IpAddr>> {
    // For now use the standard library to perform actual resolution through
    // the usage of the `ToSocketAddrs` trait. This is only
    // resolving names, not ports, so force the port to be 0.
    let addresses = (domain, 0)
        .to_socket_addrs()?
        .map(|addr| util::to_canonical(&addr.ip()).into())
        .collect();

    Ok(addresses)
}

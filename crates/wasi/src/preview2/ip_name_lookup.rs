use crate::preview2::host::network::util;
use std::net::{IpAddr, Ipv6Addr, ToSocketAddrs};
use std::str::FromStr;
use std::vec;

pub(crate) fn parse_and_resolve(name: &str) -> std::io::Result<Vec<IpAddr>> {
    let host = parse(name)?;
    blocking_resolve(&host)
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

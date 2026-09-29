use tracing::debug;

use crate::sockets::WasiSocketsCtx;
use std::{net::IpAddr, str::FromStr};

#[derive(Debug, Clone)]
#[allow(unused, reason = "want to retain parity with the WIT enum")]
pub enum ErrorCode {
    AccessDenied,
    InvalidArgument,
    NameUnresolvable,
    TemporaryResolverFailure,
    PermanentResolverFailure,
    Other,
}

pub(crate) fn resolve_addresses(
    ctx: &WasiSocketsCtx,
    name: String,
) -> impl Future<Output = Result<Vec<IpAddr>, ErrorCode>> + Send + use<> {
    let allowed = ctx.allowed_network_uses.ip_name_lookup;

    async move {
        if !allowed {
            return Err(ErrorCode::PermanentResolverFailure);
        }

        // `url::Host::parse` serves us two purposes:
        // 1. validate the input is a valid domain name or IP,
        // 2. convert unicode domains to punycode.
        let domain = match url::Host::parse(&name) {
            Ok(url::Host::Domain(domain)) => domain,
            Ok(url::Host::Ipv4(addr)) => return Ok(vec![IpAddr::V4(addr)]),
            Ok(url::Host::Ipv6(addr)) => return Ok(vec![IpAddr::V6(addr)]),
            // `url::Host::parse` doesn't understand bare IPv6 addresses without [brackets]
            Err(_) if let Ok(addr) = std::net::Ipv6Addr::from_str(&name) => {
                return Ok(vec![IpAddr::V6(addr)]);
            }
            Err(_) => return Err(ErrorCode::InvalidArgument),
        };

        let addrs = tokio::net::lookup_host((domain.as_str(), 0))
            .await
            .map_err(|e| {
                debug!("DNS resolution of `{}` failed because: {}", domain, e);
                // If/when we use `getaddrinfo` directly, map the error properly.
                ErrorCode::NameUnresolvable
            })?;

        Ok(addrs.map(|addr| addr.ip().to_canonical()).collect())
    }
}

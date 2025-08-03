use core::ops::Range;

use crate::p3::wasi::random;
use crate::p3::wasi::sockets::types::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
    UdpSocket,
};

impl IpAddress {
    pub const IPV4_BROADCAST: IpAddress = IpAddress::Ipv4((255, 255, 255, 255));

    pub const IPV4_LOOPBACK: IpAddress = IpAddress::Ipv4((127, 0, 0, 1));
    pub const IPV6_LOOPBACK: IpAddress = IpAddress::Ipv6((0, 0, 0, 0, 0, 0, 0, 1));

    pub const IPV4_UNSPECIFIED: IpAddress = IpAddress::Ipv4((0, 0, 0, 0));
    pub const IPV6_UNSPECIFIED: IpAddress = IpAddress::Ipv6((0, 0, 0, 0, 0, 0, 0, 0));

    pub const IPV4_MAPPED_LOOPBACK: IpAddress =
        IpAddress::Ipv6((0, 0, 0, 0, 0, 0xFFFF, 0x7F00, 0x0001));

    pub const fn new_loopback(family: IpAddressFamily) -> IpAddress {
        match family {
            IpAddressFamily::Ipv4 => Self::IPV4_LOOPBACK,
            IpAddressFamily::Ipv6 => Self::IPV6_LOOPBACK,
        }
    }

    pub const fn new_unspecified(family: IpAddressFamily) -> IpAddress {
        match family {
            IpAddressFamily::Ipv4 => Self::IPV4_UNSPECIFIED,
            IpAddressFamily::Ipv6 => Self::IPV6_UNSPECIFIED,
        }
    }

    pub const fn family(&self) -> IpAddressFamily {
        match self {
            IpAddress::Ipv4(_) => IpAddressFamily::Ipv4,
            IpAddress::Ipv6(_) => IpAddressFamily::Ipv6,
        }
    }
}

impl PartialEq for IpAddress {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ipv4(left), Self::Ipv4(right)) => left == right,
            (Self::Ipv6(left), Self::Ipv6(right)) => left == right,
            _ => false,
        }
    }
}

impl IpSocketAddress {
    pub const fn new(ip: IpAddress, port: u16) -> IpSocketAddress {
        match ip {
            IpAddress::Ipv4(addr) => IpSocketAddress::Ipv4(Ipv4SocketAddress {
                port,
                address: addr,
            }),
            IpAddress::Ipv6(addr) => IpSocketAddress::Ipv6(Ipv6SocketAddress {
                port,
                address: addr,
                flow_info: 0,
                scope_id: 0,
            }),
        }
    }

    pub const fn ip(&self) -> IpAddress {
        match self {
            IpSocketAddress::Ipv4(addr) => IpAddress::Ipv4(addr.address),
            IpSocketAddress::Ipv6(addr) => IpAddress::Ipv6(addr.address),
        }
    }

    pub const fn port(&self) -> u16 {
        match self {
            IpSocketAddress::Ipv4(addr) => addr.port,
            IpSocketAddress::Ipv6(addr) => addr.port,
        }
    }

    pub const fn family(&self) -> IpAddressFamily {
        match self {
            IpSocketAddress::Ipv4(_) => IpAddressFamily::Ipv4,
            IpSocketAddress::Ipv6(_) => IpAddressFamily::Ipv6,
        }
    }
}

impl PartialEq for Ipv4SocketAddress {
    fn eq(&self, other: &Self) -> bool {
        self.port == other.port && self.address == other.address
    }
}

impl PartialEq for Ipv6SocketAddress {
    fn eq(&self, other: &Self) -> bool {
        self.port == other.port
            && self.flow_info == other.flow_info
            && self.address == other.address
            && self.scope_id == other.scope_id
    }
}

impl PartialEq for IpSocketAddress {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ipv4(l0), Self::Ipv4(r0)) => l0 == r0,
            (Self::Ipv6(l0), Self::Ipv6(r0)) => l0 == r0,
            _ => false,
        }
    }
}

fn generate_random_u16(range: Range<u16>) -> u16 {
    let start = range.start as u64;
    let end = range.end as u64;
    let port = start + (random::random::get_random_u64() % (end - start));
    port as u16
}

/// Execute the inner function with a randomly generated port.
/// To prevent random failures, we make a few attempts before giving up.
pub fn attempt_random_port<F>(
    local_address: IpAddress,
    mut f: F,
) -> Result<IpSocketAddress, ErrorCode>
where
    F: FnMut(IpSocketAddress) -> Result<(), ErrorCode>,
{
    const MAX_ATTEMPTS: u32 = 10;
    let mut i = 0;
    loop {
        i += 1;

        let port: u16 = generate_random_u16(1024..u16::MAX);
        let sock_addr = IpSocketAddress::new(local_address, port);

        match f(sock_addr) {
            Ok(_) => return Ok(sock_addr),
            Err(e) if i >= MAX_ATTEMPTS => return Err(e),
            // Try again if the port is already taken. This can sometimes show up as `AccessDenied` on Windows.
            Err(ErrorCode::AddressInUse | ErrorCode::AccessDenied) => {}
            Err(e) => return Err(e),
        }
    }
}

impl UdpSocket {
    pub fn bind_unspecified(&self) -> Result<(), ErrorCode> {
        let ip = IpAddress::new_unspecified(self.address_family());
        let port = 0;

        self.bind(IpSocketAddress::new(ip, port))
    }
}

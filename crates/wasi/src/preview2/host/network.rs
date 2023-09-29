use crate::preview2::bindings::sockets::network::{
    self, ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4Address, Ipv4SocketAddress, Ipv6Address,
    Ipv6SocketAddress,
};
use crate::preview2::network::TableNetworkExt;
use crate::preview2::{TableError, WasiView};
use std::io;
use wasmtime::component::Resource;

impl<T: WasiView> network::Host for T {}

impl<T: WasiView> crate::preview2::bindings::sockets::network::HostNetwork for T {
    fn drop(&mut self, this: Resource<network::Network>) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        table.delete_network(this)?;

        Ok(())
    }
}

impl From<TableError> for network::Error {
    fn from(error: TableError) -> Self {
        Self::trap(error.into())
    }
}

impl From<io::Error> for network::Error {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            // Errors that we can directly map.
            io::ErrorKind::PermissionDenied => ErrorCode::AccessDenied,
            io::ErrorKind::ConnectionRefused => ErrorCode::ConnectionRefused,
            io::ErrorKind::ConnectionReset => ErrorCode::ConnectionReset,
            io::ErrorKind::NotConnected => ErrorCode::NotConnected,
            io::ErrorKind::AddrInUse => ErrorCode::AddressInUse,
            io::ErrorKind::AddrNotAvailable => ErrorCode::AddressNotBindable,
            io::ErrorKind::WouldBlock => ErrorCode::WouldBlock,
            io::ErrorKind::TimedOut => ErrorCode::Timeout,
            io::ErrorKind::Unsupported => ErrorCode::NotSupported,
            io::ErrorKind::OutOfMemory => ErrorCode::OutOfMemory,

            // Errors we don't expect to see here.
            io::ErrorKind::Interrupted | io::ErrorKind::ConnectionAborted => {
                // Transient errors should be skipped.
                return Self::trap(error.into());
            }

            // Errors not expected from network APIs.
            io::ErrorKind::WriteZero
            | io::ErrorKind::InvalidInput
            | io::ErrorKind::InvalidData
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::NotFound
            | io::ErrorKind::UnexpectedEof
            | io::ErrorKind::AlreadyExists => return Self::trap(error.into()),

            // Errors that don't correspond to a Rust `io::ErrorKind`.
            io::ErrorKind::Other => match error.raw_os_error() {
                None => return Self::trap(error.into()),
                Some(libc::ENOBUFS) | Some(libc::ENOMEM) => ErrorCode::OutOfMemory,
                Some(libc::EOPNOTSUPP) => ErrorCode::NotSupported,
                Some(libc::ENETUNREACH) | Some(libc::EHOSTUNREACH) | Some(libc::ENETDOWN) => {
                    ErrorCode::RemoteUnreachable
                }
                Some(libc::ECONNRESET) => ErrorCode::ConnectionReset,
                Some(libc::ECONNREFUSED) => ErrorCode::ConnectionRefused,
                Some(libc::EADDRINUSE) => ErrorCode::AddressInUse,
                Some(_) => return Self::trap(error.into()),
            },
            _ => return Self::trap(error.into()),
        }
        .into()
    }
}

impl From<rustix::io::Errno> for network::Error {
    fn from(error: rustix::io::Errno) -> Self {
        std::io::Error::from(error).into()
    }
}

impl From<IpSocketAddress> for std::net::SocketAddr {
    fn from(addr: IpSocketAddress) -> Self {
        match addr {
            IpSocketAddress::Ipv4(ipv4) => Self::V4(ipv4.into()),
            IpSocketAddress::Ipv6(ipv6) => Self::V6(ipv6.into()),
        }
    }
}

impl From<std::net::SocketAddr> for IpSocketAddress {
    fn from(addr: std::net::SocketAddr) -> Self {
        match addr {
            std::net::SocketAddr::V4(v4) => Self::Ipv4(v4.into()),
            std::net::SocketAddr::V6(v6) => Self::Ipv6(v6.into()),
        }
    }
}

impl From<Ipv4SocketAddress> for std::net::SocketAddrV4 {
    fn from(addr: Ipv4SocketAddress) -> Self {
        Self::new(to_ipv4_addr(addr.address), addr.port)
    }
}

impl From<std::net::SocketAddrV4> for Ipv4SocketAddress {
    fn from(addr: std::net::SocketAddrV4) -> Self {
        Self {
            address: from_ipv4_addr(*addr.ip()),
            port: addr.port(),
        }
    }
}

impl From<Ipv6SocketAddress> for std::net::SocketAddrV6 {
    fn from(addr: Ipv6SocketAddress) -> Self {
        Self::new(
            to_ipv6_addr(addr.address),
            addr.port,
            addr.flow_info,
            addr.scope_id,
        )
    }
}

impl From<std::net::SocketAddrV6> for Ipv6SocketAddress {
    fn from(addr: std::net::SocketAddrV6) -> Self {
        Self {
            address: from_ipv6_addr(*addr.ip()),
            port: addr.port(),
            flow_info: addr.flowinfo(),
            scope_id: addr.scope_id(),
        }
    }
}

fn to_ipv4_addr(addr: Ipv4Address) -> std::net::Ipv4Addr {
    let (x0, x1, x2, x3) = addr;
    std::net::Ipv4Addr::new(x0, x1, x2, x3)
}

fn from_ipv4_addr(addr: std::net::Ipv4Addr) -> Ipv4Address {
    let [x0, x1, x2, x3] = addr.octets();
    (x0, x1, x2, x3)
}

fn to_ipv6_addr(addr: Ipv6Address) -> std::net::Ipv6Addr {
    let (x0, x1, x2, x3, x4, x5, x6, x7) = addr;
    std::net::Ipv6Addr::new(x0, x1, x2, x3, x4, x5, x6, x7)
}

fn from_ipv6_addr(addr: std::net::Ipv6Addr) -> Ipv6Address {
    let [x0, x1, x2, x3, x4, x5, x6, x7] = addr.segments();
    (x0, x1, x2, x3, x4, x5, x6, x7)
}

impl std::net::ToSocketAddrs for IpSocketAddress {
    type Iter = <std::net::SocketAddr as std::net::ToSocketAddrs>::Iter;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        std::net::SocketAddr::from(*self).to_socket_addrs()
    }
}

impl std::net::ToSocketAddrs for Ipv4SocketAddress {
    type Iter = <std::net::SocketAddrV4 as std::net::ToSocketAddrs>::Iter;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        std::net::SocketAddrV4::from(*self).to_socket_addrs()
    }
}

impl std::net::ToSocketAddrs for Ipv6SocketAddress {
    type Iter = <std::net::SocketAddrV6 as std::net::ToSocketAddrs>::Iter;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        std::net::SocketAddrV6::from(*self).to_socket_addrs()
    }
}

impl From<IpAddressFamily> for cap_net_ext::AddressFamily {
    fn from(family: IpAddressFamily) -> Self {
        match family {
            IpAddressFamily::Ipv4 => cap_net_ext::AddressFamily::Ipv4,
            IpAddressFamily::Ipv6 => cap_net_ext::AddressFamily::Ipv6,
        }
    }
}

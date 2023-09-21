use rustix::io::Errno;

use crate::preview2::bindings::sockets::network::{
    self, ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4Address, Ipv4SocketAddress, Ipv6Address,
    Ipv6SocketAddress,
};
use crate::preview2::network::TableNetworkExt;
use crate::preview2::{TableError, WasiView};
use anyhow::anyhow;
use std::io;

impl<T: WasiView> network::Host for T {
    fn drop_network(&mut self, this: network::Network) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        table.delete_network(this)?;

        Ok(())
    }
}


/// Unfortunately, Rust's io::ErrorKind is missing more than half of the relevant error codes.
/// This trait provides access to a unified error code.
pub(crate) trait SystemError: std::error::Error {
    fn errno(&self) -> Option<Errno>;
}

impl SystemError for Errno {
    fn errno(&self) -> Option<Errno> {
        Some(*self)
    }
}

impl SystemError for std::io::Error {
    fn errno(&self) -> Option<Errno> {
        if let Some(errno) = Errno::from_io_error(self) {
            return Some(errno);
        }

        // Error is probably synthesized in Rust code. Luckily, the errors kinds map pretty straightforward back to native error codes.
        match self.kind() {
            std::io::ErrorKind::AddrInUse => Some(Errno::ADDRINUSE),
            std::io::ErrorKind::AddrNotAvailable => Some(Errno::ADDRNOTAVAIL),
            std::io::ErrorKind::AlreadyExists => Some(Errno::EXIST),
            std::io::ErrorKind::BrokenPipe => Some(Errno::PIPE),
            std::io::ErrorKind::ConnectionAborted => Some(Errno::CONNABORTED),
            std::io::ErrorKind::ConnectionRefused => Some(Errno::CONNREFUSED),
            std::io::ErrorKind::ConnectionReset => Some(Errno::CONNRESET),
            std::io::ErrorKind::Interrupted => Some(Errno::INTR),
            std::io::ErrorKind::InvalidInput => Some(Errno::INVAL),
            std::io::ErrorKind::NotConnected => Some(Errno::NOTCONN),
            std::io::ErrorKind::NotFound => Some(Errno::NOENT),
            std::io::ErrorKind::OutOfMemory => Some(Errno::NOMEM),
            std::io::ErrorKind::PermissionDenied => Some(Errno::ACCESS), // Alternative: EPERM
            std::io::ErrorKind::TimedOut => Some(Errno::TIMEDOUT),
            std::io::ErrorKind::Unsupported => Some(Errno::NOTSUP),
            std::io::ErrorKind::WouldBlock => Some(Errno::WOULDBLOCK), // Alternative: EAGAIN

            _ => None,
        }
    }
}


impl From<TableError> for network::Error {
    fn from(error: TableError) -> Self {
        Self::trap(error.into())
    }
}

impl<T: SystemError> From<T> for network::Error {
    fn from(error: T) -> Self {
        let errno = match error.errno() {
            Some(errno) => errno,
            None => return Self::trap(anyhow!("Unknown network error: {:?}", error)),
        };

        match errno {
            Errno::WOULDBLOCK => ErrorCode::WouldBlock,
            #[allow(unreachable_patterns)] // EWOULDBLOCK and EAGAIN can have the same value.
            Errno::AGAIN => ErrorCode::WouldBlock,
            Errno::INTR => ErrorCode::WouldBlock,
            #[cfg(not(windows))]
            Errno::PERM => ErrorCode::AccessDenied,
            Errno::ACCESS => ErrorCode::AccessDenied,
            Errno::ADDRINUSE => ErrorCode::AddressInUse,
            Errno::ADDRNOTAVAIL => ErrorCode::AddressNotBindable,
            Errno::ALREADY => ErrorCode::ConcurrencyConflict,
            Errno::TIMEDOUT => ErrorCode::Timeout,
            Errno::CONNREFUSED => ErrorCode::ConnectionRefused,
            Errno::CONNRESET => ErrorCode::ConnectionReset,
            // Errno::CONNABORTED => ErrorCode::Todo, // FIXME
            // Errno::INVAL => ErrorCode::Todo, // FIXME
            Errno::HOSTUNREACH => ErrorCode::RemoteUnreachable,
            Errno::HOSTDOWN => ErrorCode::RemoteUnreachable,
            Errno::NETDOWN => ErrorCode::RemoteUnreachable,
            Errno::NETUNREACH => ErrorCode::RemoteUnreachable,
            #[cfg(target_os = "linux")]
            Errno::NONET => ErrorCode::RemoteUnreachable,
            Errno::ISCONN => ErrorCode::AlreadyConnected,
            Errno::NOTCONN => ErrorCode::NotConnected,
            Errno::DESTADDRREQ => ErrorCode::NotConnected,
            #[cfg(not(windows))]
            Errno::NFILE => ErrorCode::NewSocketLimit,
            Errno::MFILE => ErrorCode::NewSocketLimit,
            Errno::MSGSIZE => ErrorCode::DatagramTooLarge,
            #[cfg(not(windows))]
            Errno::NOMEM => ErrorCode::OutOfMemory,
            Errno::NOBUFS => ErrorCode::OutOfMemory,
            Errno::OPNOTSUPP => ErrorCode::NotSupported,
            Errno::NOPROTOOPT => ErrorCode::NotSupported,
            Errno::PFNOSUPPORT => ErrorCode::NotSupported,
            Errno::PROTONOSUPPORT => ErrorCode::NotSupported,
            Errno::PROTOTYPE => ErrorCode::NotSupported,
            Errno::SOCKTNOSUPPORT => ErrorCode::NotSupported,
            Errno::AFNOSUPPORT => ErrorCode::NotSupported,

            // Trap on unexpected errors. These include:
            // - EINPROGRESS: Should have been handled by connect.
            // - ENOTSOCK, EFAULT, EBADF, EBADFD, ENOSYS: Implementation error on our side.
            _ => return network::Error::trap(anyhow!("Unexpected error: {:?}", error)),
        }
        .into()
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

use crate::preview2::bindings::sockets::network::{
    self, ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4Address, Ipv4SocketAddress, Ipv6Address,
    Ipv6SocketAddress,
};
use crate::preview2::{SocketError, WasiView};
use rustix::io::Errno;
use std::io;
use wasmtime::component::Resource;

impl<T: WasiView> network::Host for T {
    fn convert_error_code(&mut self, error: SocketError) -> anyhow::Result<ErrorCode> {
        error.downcast()
    }
}

impl<T: WasiView> crate::preview2::bindings::sockets::network::HostNetwork for T {
    fn drop(&mut self, this: Resource<network::Network>) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        table.delete(this)?;

        Ok(())
    }
}

impl From<io::Error> for ErrorCode {
    fn from(value: io::Error) -> Self {
        // Attempt the more detailed native error code first:
        if let Some(errno) = Errno::from_io_error(&value) {
            return errno.into();
        }

        match value.kind() {
            std::io::ErrorKind::AddrInUse => ErrorCode::AddressInUse,
            std::io::ErrorKind::AddrNotAvailable => ErrorCode::AddressNotBindable,
            std::io::ErrorKind::ConnectionAborted => ErrorCode::ConnectionAborted,
            std::io::ErrorKind::ConnectionRefused => ErrorCode::ConnectionRefused,
            std::io::ErrorKind::ConnectionReset => ErrorCode::ConnectionReset,
            std::io::ErrorKind::Interrupted => ErrorCode::WouldBlock,
            std::io::ErrorKind::InvalidInput => ErrorCode::InvalidArgument,
            std::io::ErrorKind::NotConnected => ErrorCode::InvalidState,
            std::io::ErrorKind::OutOfMemory => ErrorCode::OutOfMemory,
            std::io::ErrorKind::PermissionDenied => ErrorCode::AccessDenied,
            std::io::ErrorKind::TimedOut => ErrorCode::Timeout,
            std::io::ErrorKind::Unsupported => ErrorCode::NotSupported,
            std::io::ErrorKind::WouldBlock => ErrorCode::WouldBlock,

            _ => {
                log::debug!("unknown I/O error: {value}");
                ErrorCode::Unknown
            }
        }
    }
}

impl From<Errno> for ErrorCode {
    fn from(value: Errno) -> Self {
        match value {
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
            Errno::CONNABORTED => ErrorCode::ConnectionAborted,
            Errno::INVAL => ErrorCode::InvalidArgument,
            Errno::HOSTUNREACH => ErrorCode::RemoteUnreachable,
            Errno::HOSTDOWN => ErrorCode::RemoteUnreachable,
            Errno::NETDOWN => ErrorCode::RemoteUnreachable,
            Errno::NETUNREACH => ErrorCode::RemoteUnreachable,
            #[cfg(target_os = "linux")]
            Errno::NONET => ErrorCode::RemoteUnreachable,
            Errno::ISCONN => ErrorCode::InvalidState,
            Errno::NOTCONN => ErrorCode::InvalidState,
            Errno::DESTADDRREQ => ErrorCode::InvalidState,
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

            // FYI, EINPROGRESS should have already been handled by connect.
            _ => {
                log::debug!("unknown I/O error: {value}");
                ErrorCode::Unknown
            }
        }
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

impl From<cap_net_ext::AddressFamily> for IpAddressFamily {
    fn from(family: cap_net_ext::AddressFamily) -> Self {
        match family {
            cap_net_ext::AddressFamily::Ipv4 => IpAddressFamily::Ipv4,
            cap_net_ext::AddressFamily::Ipv6 => IpAddressFamily::Ipv6,
        }
    }
}

pub(crate) mod util {
    use std::net::{IpAddr, Ipv6Addr, SocketAddr};

    use crate::preview2::bindings::sockets::network::ErrorCode;
    use crate::preview2::network::SocketAddressFamily;
    use crate::preview2::SocketResult;
    use rustix::io::Errno;

    // On POSIX, non-blocking `connect` returns `EINPROGRESS`. Windows returns `WSAEWOULDBLOCK`.
    // <https://pubs.opengroup.org/onlinepubs/9699919799/functions/connect.html>
    // <https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-connect>
    pub const CONNECT_INPROGRESS: Errno = if cfg!(windows) {
        Errno::WOULDBLOCK
    } else {
        Errno::INPROGRESS
    };

    pub fn validate_unicast(addr: &SocketAddr) -> SocketResult<()> {
        match to_canonical(&addr.ip()) {
            IpAddr::V4(ipv4) => {
                if ipv4.is_multicast() || ipv4.is_broadcast() {
                    Err(ErrorCode::InvalidArgument.into())
                } else {
                    Ok(())
                }
            }
            IpAddr::V6(ipv6) => {
                if ipv6.is_multicast() {
                    Err(ErrorCode::InvalidArgument.into())
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn validate_remote_address(addr: &SocketAddr) -> SocketResult<()> {
        if to_canonical(&addr.ip()).is_unspecified() {
            return Err(ErrorCode::InvalidArgument.into());
        }

        if addr.port() == 0 {
            return Err(ErrorCode::InvalidArgument.into());
        }

        Ok(())
    }

    pub fn validate_address_family(
        addr: &SocketAddr,
        socket_family: &SocketAddressFamily,
    ) -> SocketResult<()> {
        match (socket_family, addr.ip()) {
            (SocketAddressFamily::Ipv4, IpAddr::V4(_)) => Ok(()),
            (SocketAddressFamily::Ipv6 { v6only }, IpAddr::V6(ipv6)) => {
                if is_deprecated_ipv4_compatible(&ipv6) {
                    // Reject IPv4-*compatible* IPv6 addresses. They have been deprecated
                    // since 2006, OS handling of them is inconsistent and our own
                    // validations don't take them into account either.
                    // Note that these are not the same as IPv4-*mapped* IPv6 addresses.
                    Err(ErrorCode::InvalidArgument.into())
                } else if *v6only && ipv6.to_ipv4_mapped().is_some() {
                    Err(ErrorCode::InvalidArgument.into())
                } else {
                    Ok(())
                }
            }
            _ => Err(ErrorCode::InvalidArgument.into()),
        }
    }

    // Can be removed once `IpAddr::to_canonical` becomes stable.
    fn to_canonical(addr: &IpAddr) -> IpAddr {
        match addr {
            IpAddr::V4(ipv4) => IpAddr::V4(*ipv4),
            IpAddr::V6(ipv6) => {
                if let Some(ipv4) = ipv6.to_ipv4_mapped() {
                    IpAddr::V4(ipv4)
                } else {
                    IpAddr::V6(*ipv6)
                }
            }
        }
    }

    fn is_deprecated_ipv4_compatible(addr: &Ipv6Addr) -> bool {
        matches!(addr.segments(), [0, 0, 0, 0, 0, 0, _, _])
            && *addr != Ipv6Addr::UNSPECIFIED
            && *addr != Ipv6Addr::LOCALHOST
    }

    pub fn normalize_setsockopt_buffer_size(value: u64) -> usize {
        value.clamp(1, i32::MAX as u64).try_into().unwrap()
    }

    pub fn normalize_getsockopt_buffer_size(value: u64) -> u64 {
        if cfg!(target_os = "linux") {
            // Linux doubles the value passed to setsockopt to allow space for bookkeeping overhead.
            // getsockopt returns this internally doubled value.
            // We'll half the value to at least get it back into the same ballpark that the application requested it in.
            value / 2
        } else {
            value
        }
    }
}

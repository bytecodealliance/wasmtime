use crate::bindings::sockets::network::{
    self, ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress,
    Ipv6SocketAddress,
};
use crate::network::{from_ipv4_addr, from_ipv6_addr, to_ipv4_addr, to_ipv6_addr};
use crate::{IoView, SocketError, WasiImpl, WasiView};
use anyhow::Error;
use rustix::io::Errno;
use std::io;
use wasmtime::component::Resource;

impl<T> network::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn convert_error_code(&mut self, error: SocketError) -> anyhow::Result<ErrorCode> {
        error.downcast()
    }

    fn network_error_code(&mut self, err: Resource<Error>) -> anyhow::Result<Option<ErrorCode>> {
        let err = self.table().get(&err)?;

        if let Some(err) = err.downcast_ref::<std::io::Error>() {
            return Ok(Some(ErrorCode::from(err)));
        }

        Ok(None)
    }
}

impl<T> crate::bindings::sockets::network::HostNetwork for WasiImpl<T>
where
    T: WasiView,
{
    fn drop(&mut self, this: Resource<network::Network>) -> Result<(), anyhow::Error> {
        let table = self.table();

        table.delete(this)?;

        Ok(())
    }
}

impl From<io::Error> for ErrorCode {
    fn from(value: io::Error) -> Self {
        (&value).into()
    }
}

impl From<&io::Error> for ErrorCode {
    fn from(value: &io::Error) -> Self {
        // Attempt the more detailed native error code first:
        if let Some(errno) = Errno::from_io_error(value) {
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
                tracing::debug!("unknown I/O error: {value}");
                ErrorCode::Unknown
            }
        }
    }
}

impl From<Errno> for ErrorCode {
    fn from(value: Errno) -> Self {
        (&value).into()
    }
}

impl From<&Errno> for ErrorCode {
    fn from(value: &Errno) -> Self {
        match *value {
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
                tracing::debug!("unknown I/O error: {value}");
                ErrorCode::Unknown
            }
        }
    }
}

impl From<std::net::IpAddr> for IpAddress {
    fn from(addr: std::net::IpAddr) -> Self {
        match addr {
            std::net::IpAddr::V4(v4) => Self::Ipv4(from_ipv4_addr(v4)),
            std::net::IpAddr::V6(v6) => Self::Ipv6(from_ipv6_addr(v6)),
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
    use std::io;
    use std::net::{IpAddr, Ipv6Addr, SocketAddr};
    use std::time::Duration;

    use crate::network::SocketAddressFamily;
    use cap_net_ext::{AddressFamily, Blocking, UdpSocketExt};
    use rustix::fd::{AsFd, OwnedFd};
    use rustix::io::Errno;
    use rustix::net::sockopt;

    pub fn validate_unicast(addr: &SocketAddr) -> io::Result<()> {
        match to_canonical(&addr.ip()) {
            IpAddr::V4(ipv4) => {
                if ipv4.is_multicast() || ipv4.is_broadcast() {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Both IPv4 broadcast and multicast addresses are not supported",
                    ))
                } else {
                    Ok(())
                }
            }
            IpAddr::V6(ipv6) => {
                if ipv6.is_multicast() {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "IPv6 multicast addresses are not supported",
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn validate_remote_address(addr: &SocketAddr) -> io::Result<()> {
        if to_canonical(&addr.ip()).is_unspecified() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Remote address may not be `0.0.0.0` or `::`",
            ));
        }

        if addr.port() == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Remote port may not be 0",
            ));
        }

        Ok(())
    }

    pub fn validate_address_family(
        addr: &SocketAddr,
        socket_family: &SocketAddressFamily,
    ) -> io::Result<()> {
        match (socket_family, addr.ip()) {
            (SocketAddressFamily::Ipv4, IpAddr::V4(_)) => Ok(()),
            (SocketAddressFamily::Ipv6, IpAddr::V6(ipv6)) => {
                if is_deprecated_ipv4_compatible(&ipv6) {
                    // Reject IPv4-*compatible* IPv6 addresses. They have been deprecated
                    // since 2006, OS handling of them is inconsistent and our own
                    // validations don't take them into account either.
                    // Note that these are not the same as IPv4-*mapped* IPv6 addresses.
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "IPv4-compatible IPv6 addresses are not supported",
                    ))
                } else if ipv6.to_ipv4_mapped().is_some() {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "IPv4-mapped IPv6 address passed to an IPv6-only socket",
                    ))
                } else {
                    Ok(())
                }
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Address family mismatch",
            )),
        }
    }

    // Can be removed once `IpAddr::to_canonical` becomes stable.
    pub fn to_canonical(addr: &IpAddr) -> IpAddr {
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

    /*
     * Syscalls wrappers with (opinionated) portability fixes.
     */

    pub fn udp_socket(family: AddressFamily, blocking: Blocking) -> io::Result<OwnedFd> {
        // Delegate socket creation to cap_net_ext. They handle a couple of things for us:
        // - On Windows: call WSAStartup if not done before.
        // - Set the NONBLOCK and CLOEXEC flags. Either immediately during socket creation,
        //   or afterwards using ioctl or fcntl. Exact method depends on the platform.

        let socket = cap_std::net::UdpSocket::new(family, blocking)?;
        Ok(OwnedFd::from(socket))
    }

    pub fn udp_bind<Fd: AsFd>(sockfd: Fd, addr: &SocketAddr) -> rustix::io::Result<()> {
        rustix::net::bind(sockfd, addr).map_err(|error| match error {
            // See: https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-bind#:~:text=WSAENOBUFS
            // Windows returns WSAENOBUFS when the ephemeral ports have been exhausted.
            #[cfg(windows)]
            Errno::NOBUFS => Errno::ADDRINUSE,
            _ => error,
        })
    }

    pub fn udp_disconnect<Fd: AsFd>(sockfd: Fd) -> rustix::io::Result<()> {
        match rustix::net::connect_unspec(sockfd) {
            // BSD platforms return an error even if the UDP socket was disconnected successfully.
            //
            // MacOS was kind enough to document this: https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/connect.2.html
            // > Datagram sockets may dissolve the association by connecting to an
            // > invalid address, such as a null address or an address with the address
            // > family set to AF_UNSPEC (the error EAFNOSUPPORT will be harmlessly
            // > returned).
            //
            // ... except that this appears to be incomplete, because experiments
            // have shown that MacOS actually returns EINVAL, depending on the
            // address family of the socket.
            #[cfg(target_os = "macos")]
            Err(Errno::INVAL | Errno::AFNOSUPPORT) => Ok(()),
            r => r,
        }
    }

    // Even though SO_REUSEADDR is a SOL_* level option, this function contain a
    // compatibility fix specific to TCP. That's why it contains the `_tcp_` infix instead of `_socket_`.
    #[allow(unused_variables)] // Parameters are not used on Windows
    pub fn set_tcp_reuseaddr<Fd: AsFd>(sockfd: Fd, value: bool) -> rustix::io::Result<()> {
        // When a TCP socket is closed, the system may
        // temporarily reserve that specific address+port pair in a so called
        // TIME_WAIT state. During that period, any attempt to rebind to that pair
        // will fail. Setting SO_REUSEADDR to true bypasses that behaviour. Unlike
        // the name "SO_REUSEADDR" might suggest, it does not allow multiple
        // active sockets to share the same local address.

        // On Windows that behavior is the default, so there is no need to manually
        // configure such an option. But (!), Windows _does_ have an identically
        // named socket option which allows users to "hijack" active sockets.
        // This is definitely not what we want to do here.

        // Microsoft's own documentation[1] states that we should set SO_EXCLUSIVEADDRUSE
        // instead (to the inverse value), however the github issue below[2] seems
        // to indicate that that may no longer be correct.
        // [1]: https://docs.microsoft.com/en-us/windows/win32/winsock/using-so-reuseaddr-and-so-exclusiveaddruse
        // [2]: https://github.com/python-trio/trio/issues/928

        #[cfg(not(windows))]
        sockopt::set_socket_reuseaddr(sockfd, value)?;

        Ok(())
    }

    pub fn set_tcp_keepidle<Fd: AsFd>(sockfd: Fd, value: Duration) -> rustix::io::Result<()> {
        if value <= Duration::ZERO {
            // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
            return Err(Errno::INVAL);
        }

        // Ensure that the value passed to the actual syscall never gets rounded down to 0.
        const MIN_SECS: u64 = 1;

        // Cap it at Linux' maximum, which appears to have the lowest limit across our supported platforms.
        const MAX_SECS: u64 = i16::MAX as u64;

        sockopt::set_tcp_keepidle(
            sockfd,
            value.clamp(Duration::from_secs(MIN_SECS), Duration::from_secs(MAX_SECS)),
        )
    }

    pub fn set_tcp_keepintvl<Fd: AsFd>(sockfd: Fd, value: Duration) -> rustix::io::Result<()> {
        if value <= Duration::ZERO {
            // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
            return Err(Errno::INVAL);
        }

        // Ensure that any fractional value passed to the actual syscall never gets rounded down to 0.
        const MIN_SECS: u64 = 1;

        // Cap it at Linux' maximum, which appears to have the lowest limit across our supported platforms.
        const MAX_SECS: u64 = i16::MAX as u64;

        sockopt::set_tcp_keepintvl(
            sockfd,
            value.clamp(Duration::from_secs(MIN_SECS), Duration::from_secs(MAX_SECS)),
        )
    }

    pub fn set_tcp_keepcnt<Fd: AsFd>(sockfd: Fd, value: u32) -> rustix::io::Result<()> {
        if value == 0 {
            // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
            return Err(Errno::INVAL);
        }

        const MIN_CNT: u32 = 1;
        // Cap it at Linux' maximum, which appears to have the lowest limit across our supported platforms.
        const MAX_CNT: u32 = i8::MAX as u32;

        sockopt::set_tcp_keepcnt(sockfd, value.clamp(MIN_CNT, MAX_CNT))
    }

    pub fn get_ip_ttl<Fd: AsFd>(sockfd: Fd) -> rustix::io::Result<u8> {
        sockopt::get_ip_ttl(sockfd)?
            .try_into()
            .map_err(|_| Errno::OPNOTSUPP)
    }

    pub fn get_ipv6_unicast_hops<Fd: AsFd>(sockfd: Fd) -> rustix::io::Result<u8> {
        sockopt::get_ipv6_unicast_hops(sockfd)
    }

    pub fn set_ip_ttl<Fd: AsFd>(sockfd: Fd, value: u8) -> rustix::io::Result<()> {
        match value {
            // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
            //
            // A well-behaved IP application should never send out new packets with TTL 0.
            // We validate the value ourselves because OS'es are not consistent in this.
            // On Linux the validation is even inconsistent between their IPv4 and IPv6 implementation.
            0 => Err(Errno::INVAL),
            _ => sockopt::set_ip_ttl(sockfd, value.into()),
        }
    }

    pub fn set_ipv6_unicast_hops<Fd: AsFd>(sockfd: Fd, value: u8) -> rustix::io::Result<()> {
        match value {
            0 => Err(Errno::INVAL), // See `set_ip_ttl`
            _ => sockopt::set_ipv6_unicast_hops(sockfd, Some(value)),
        }
    }

    fn normalize_get_buffer_size(value: usize) -> usize {
        if cfg!(target_os = "linux") {
            // Linux doubles the value passed to setsockopt to allow space for bookkeeping overhead.
            // getsockopt returns this internally doubled value.
            // We'll half the value to at least get it back into the same ballpark that the application requested it in.
            //
            // This normalized behavior is tested for in: test-programs/src/bin/preview2_tcp_sockopts.rs
            value / 2
        } else {
            value
        }
    }

    fn normalize_set_buffer_size(value: usize) -> usize {
        value.clamp(1, i32::MAX as usize)
    }

    pub fn get_socket_recv_buffer_size<Fd: AsFd>(sockfd: Fd) -> rustix::io::Result<usize> {
        let value = sockopt::get_socket_recv_buffer_size(sockfd)?;
        Ok(normalize_get_buffer_size(value))
    }

    pub fn get_socket_send_buffer_size<Fd: AsFd>(sockfd: Fd) -> rustix::io::Result<usize> {
        let value = sockopt::get_socket_send_buffer_size(sockfd)?;
        Ok(normalize_get_buffer_size(value))
    }

    pub fn set_socket_recv_buffer_size<Fd: AsFd>(
        sockfd: Fd,
        value: usize,
    ) -> rustix::io::Result<()> {
        if value == 0 {
            // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
            return Err(Errno::INVAL);
        }

        let value = normalize_set_buffer_size(value);

        match sockopt::set_socket_recv_buffer_size(sockfd, value) {
            // Most platforms (Linux, Windows, Fuchsia, Solaris, Illumos, Haiku, ESP-IDF, ..and more?) treat the value
            // passed to SO_SNDBUF/SO_RCVBUF as a performance tuning hint and silently clamp the input if it exceeds
            // their capability.
            // As far as I can see, only the *BSD family views this option as a hard requirement and fails when the
            // value is out of range. We normalize this behavior in favor of the more commonly understood
            // "performance hint" semantics. In other words; even ENOBUFS is "Ok".
            // A future improvement could be to query the corresponding sysctl on *BSD platforms and clamp the input
            // `size` ourselves, to completely close the gap with other platforms.
            //
            // This normalized behavior is tested for in: test-programs/src/bin/preview2_tcp_sockopts.rs
            Err(Errno::NOBUFS) => Ok(()),
            r => r,
        }
    }

    pub fn set_socket_send_buffer_size<Fd: AsFd>(
        sockfd: Fd,
        value: usize,
    ) -> rustix::io::Result<()> {
        if value == 0 {
            // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
            return Err(Errno::INVAL);
        }

        let value = normalize_set_buffer_size(value);

        match sockopt::set_socket_send_buffer_size(sockfd, value) {
            Err(Errno::NOBUFS) => Ok(()), // See set_socket_recv_buffer_size
            r => r,
        }
    }
}

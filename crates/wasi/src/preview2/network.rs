use crate::preview2::bindings::sockets::network::{Ipv4Address, Ipv6Address};
use crate::preview2::bindings::wasi::sockets::network::ErrorCode;
use crate::preview2::ip_name_lookup::ResolveAddressStream;
use crate::preview2::TrappableError;
use std::net::SocketAddr;
use std::sync::Arc;

/// A network implementation
pub trait Network: Sync + Send {
    /// Given a name, resolve to a list of IP addresses
    fn resolve_addresses(&mut self, name: String) -> ResolveAddressStream;
}

/// The default network implementation
#[derive(Debug, Clone, Default)]
pub struct DefaultNetwork {
    system: SystemNetwork,
    allowed: bool,
}

impl DefaultNetwork {
    /// Create a new `DefaultNetwork`
    pub fn new(allowed: bool) -> Self {
        Self {
            system: SystemNetwork::new(),
            allowed,
        }
    }
}

impl Network for DefaultNetwork {
    fn resolve_addresses(&mut self, name: String) -> ResolveAddressStream {
        let allowed = self.allowed;

        if !allowed {
            return ResolveAddressStream::done(Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "IP name lookup is not allowed",
            )
            .into()));
        }

        self.system.resolve_addresses(name)
    }
}

/// An implementation of `Networked` that uses the underlying system
#[derive(Debug, Clone, Default)]
pub struct SystemNetwork {}

impl SystemNetwork {
    /// Create a new `SystemNetwork`
    pub fn new() -> Self {
        Self {}
    }
}

impl Network for SystemNetwork {
    fn resolve_addresses(&mut self, name: String) -> ResolveAddressStream {
        ResolveAddressStream::wait(super::spawn_blocking(move || {
            super::ip_name_lookup::parse_and_resolve(&name)
        }))
    }
}

pub struct NetworkHandle {
    pub socket_addr_check: SocketAddrCheck,
}

impl NetworkHandle {
    pub fn check_socket_addr(
        &self,
        addr: &SocketAddr,
        reason: SocketAddrUse,
    ) -> std::io::Result<()> {
        self.socket_addr_check.check(addr, reason)
    }
}

/// A check that will be called for each socket address that is used of whether the address is permitted.
#[derive(Clone)]
pub struct SocketAddrCheck(
    pub(crate) Arc<dyn Fn(&SocketAddr, SocketAddrUse) -> bool + Send + Sync>,
);

impl SocketAddrCheck {
    pub fn check(&self, addr: &SocketAddr, reason: SocketAddrUse) -> std::io::Result<()> {
        if (self.0)(addr, reason) {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "An address was not permitted by the socket address check.",
            ))
        }
    }
}

impl Default for SocketAddrCheck {
    fn default() -> Self {
        Self(Arc::new(|_, _| false))
    }
}

/// The reason what a socket address is being used for.
#[derive(Clone, Copy, Debug)]
pub enum SocketAddrUse {
    /// Binding TCP socket
    TcpBind,
    /// Connecting TCP socket
    TcpConnect,
    /// Binding UDP socket
    UdpBind,
    /// Connecting UDP socket
    UdpConnect,
    /// Sending datagram on non-connected UDP socket
    UdpOutgoingDatagram,
}

pub type SocketResult<T> = Result<T, SocketError>;

pub type SocketError = TrappableError<ErrorCode>;

impl From<wasmtime::component::ResourceTableError> for SocketError {
    fn from(error: wasmtime::component::ResourceTableError) -> Self {
        Self::trap(error)
    }
}

impl From<std::io::Error> for SocketError {
    fn from(error: std::io::Error) -> Self {
        ErrorCode::from(error).into()
    }
}

impl From<rustix::io::Errno> for SocketError {
    fn from(error: rustix::io::Errno) -> Self {
        ErrorCode::from(error).into()
    }
}

#[derive(Copy, Clone)]
pub(crate) enum SocketAddressFamily {
    Ipv4,
    Ipv6 { v6only: bool },
}

/// IP version. Effectively the discriminant of `SocketAddr`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum SocketAddrFamily {
    V4,
    V6,
}

pub trait SocketAddrExt {
    fn family(&self) -> SocketAddrFamily;
}

impl SocketAddrExt for SocketAddr {
    fn family(&self) -> SocketAddrFamily {
        match self {
            SocketAddr::V4(_) => SocketAddrFamily::V4,
            SocketAddr::V6(_) => SocketAddrFamily::V6,
        }
    }
}

pub(crate) fn to_ipv4_addr(addr: Ipv4Address) -> std::net::Ipv4Addr {
    let (x0, x1, x2, x3) = addr;
    std::net::Ipv4Addr::new(x0, x1, x2, x3)
}

pub(crate) fn from_ipv4_addr(addr: std::net::Ipv4Addr) -> Ipv4Address {
    let [x0, x1, x2, x3] = addr.octets();
    (x0, x1, x2, x3)
}

pub(crate) fn to_ipv6_addr(addr: Ipv6Address) -> std::net::Ipv6Addr {
    let (x0, x1, x2, x3, x4, x5, x6, x7) = addr;
    std::net::Ipv6Addr::new(x0, x1, x2, x3, x4, x5, x6, x7)
}

pub(crate) fn from_ipv6_addr(addr: std::net::Ipv6Addr) -> Ipv6Address {
    let [x0, x1, x2, x3, x4, x5, x6, x7] = addr.segments();
    (x0, x1, x2, x3, x4, x5, x6, x7)
}

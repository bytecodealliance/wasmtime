use crate::preview2::bindings::sockets::network::{Ipv4Address, Ipv6Address};
use crate::preview2::bindings::wasi::sockets::network::ErrorCode;
use crate::preview2::ip_name_lookup::resolve_addresses;
use crate::preview2::tcp::{DefaultTcpSocket, SystemTcpSocket, TcpSocket};
use crate::preview2::{BoxSyncFuture, TrappableError};
use futures::Future;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

/// A network implementation
pub trait Network: Sync + Send {
    /// Given a name, resolve to a list of IP addresses
    ///
    /// Unicode domain names are automatically converted to ASCII using IDNA encoding.
    /// If the input is an IP address string, the address is parsed and returned
    /// as-is without making any external requests. The results are returned in
    /// connection order preference. This function never returns IPv4-mapped IPv6 addresses.
    fn resolve_addresses(&mut self, name: String) -> BoxSyncFuture<io::Result<Vec<IpAddr>>>;

    /// Create a new TCP socket.
    fn new_tcp_socket(&mut self, family: SocketAddrFamily) -> io::Result<Box<dyn TcpSocket>>;
}

/// The default network implementation
#[derive(Default)]
pub struct DefaultNetwork {
    system: SystemNetwork,
    allow_ip_name_lookup: bool,
    tcp_addr_check: SocketAddrCheck,
}

impl DefaultNetwork {
    /// Create a new `DefaultNetwork`
    pub fn new() -> Self {
        Self {
            system: SystemNetwork::new(),
            allow_ip_name_lookup: false,
            tcp_addr_check: SocketAddrCheck::deny(),
        }
    }

    pub fn allow_ip_name_lookup(&mut self, allowed: bool) {
        self.allow_ip_name_lookup = allowed;
    }

    pub fn allow_tcp(&mut self, check: SocketAddrCheck) {
        self.tcp_addr_check = check;
    }
}

impl Network for DefaultNetwork {
    fn resolve_addresses(&mut self, name: String) -> BoxSyncFuture<io::Result<Vec<IpAddr>>> {
        let allowed = self.allow_ip_name_lookup;

        if !allowed {
            return Box::pin(async {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "IP name lookup is not allowed",
                ))
            });
        }

        Network::resolve_addresses(&mut self.system, name)
    }

    fn new_tcp_socket(&mut self, family: SocketAddrFamily) -> io::Result<Box<dyn TcpSocket>> {
        Ok(Box::new(DefaultTcpSocket::new(
            self.system.new_tcp_socket(family)?,
            self.tcp_addr_check.clone(),
        )))
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

    /// Non-boxing variant of [Network::resolve_addresses]
    pub fn resolve_addresses(
        &mut self,
        name: String,
    ) -> impl Future<Output = io::Result<Vec<IpAddr>>> + Send + Sync + 'static {
        async move { resolve_addresses(&name).await }
    }

    /// Non-boxing variant of [Network::new_tcp_socket]
    pub fn new_tcp_socket(&mut self, family: SocketAddrFamily) -> io::Result<SystemTcpSocket> {
        SystemTcpSocket::new(family)
    }
}

impl Network for SystemNetwork {
    fn resolve_addresses(&mut self, name: String) -> BoxSyncFuture<io::Result<Vec<IpAddr>>> {
        Box::pin(self.resolve_addresses(name))
    }

    fn new_tcp_socket(&mut self, family: SocketAddrFamily) -> io::Result<Box<dyn TcpSocket>> {
        Ok(Box::new(self.new_tcp_socket(family)?))
    }
}

pub struct NetworkHandle {
    _priv: (),
}

impl NetworkHandle {
    pub fn new() -> Self {
        Self { _priv: () }
    }

    pub(crate) fn check_access(&self) -> io::Result<()> {
        // At the moment, there's only one network handle (`instance-network`)
        // in existence. The fact that we ended up in this method indicates that
        // the Wasm program had access to a valid network handle.
        // That's good enough for now:
        Ok(())
    }
}

/// A check that will be called for each socket address that is used of whether the address is permitted.
#[derive(Clone)]
pub struct SocketAddrCheck(
    pub(crate) Arc<dyn Fn(&SocketAddr, SocketAddrUse) -> bool + Send + Sync>,
);

impl SocketAddrCheck {
    pub fn deny() -> Self {
        Self(Arc::new(|_, _| false))
    }

    pub fn allow() -> Self {
        Self(Arc::new(|_, _| true))
    }

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
        Self::deny()
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
pub(crate) enum SocketProtocolMode {
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

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

/// Value taken from rust std library.
pub const DEFAULT_TCP_BACKLOG: u32 = 128;

pub struct Network {
    pub socket_addr_check: SocketAddrCheck,
    pub allow_ip_name_lookup: bool,
}

impl Network {
    pub async fn check_socket_addr(
        &self,
        addr: SocketAddr,
        reason: SocketAddrUse,
    ) -> std::io::Result<()> {
        self.socket_addr_check.check(addr, reason).await
    }
}

/// A check that will be called for each socket address that is used of whether the address is permitted.
#[derive(Clone)]
pub struct SocketAddrCheck(
    pub(crate)  Arc<
        dyn Fn(SocketAddr, SocketAddrUse) -> Pin<Box<dyn Future<Output = bool> + Send + Sync>>
            + Send
            + Sync,
    >,
);

impl SocketAddrCheck {
    pub async fn check(&self, addr: SocketAddr, reason: SocketAddrUse) -> std::io::Result<()> {
        if (self.0)(addr, reason).await {
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
        Self(Arc::new(|_, _| Box::pin(async { false })))
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

#[derive(Copy, Clone)]
pub enum SocketAddressFamily {
    Ipv4,
    Ipv6,
}

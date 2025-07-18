use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

/// Value taken from rust std library.
pub const DEFAULT_TCP_BACKLOG: u32 = 128;

#[derive(Clone, Default)]
pub struct WasiSocketsCtx {
    pub socket_addr_check: SocketAddrCheck,
    pub allowed_network_uses: AllowedNetworkUses,
}

#[derive(Clone)]
pub struct AllowedNetworkUses {
    pub ip_name_lookup: bool,
    pub udp: bool,
    pub tcp: bool,
}

impl Default for AllowedNetworkUses {
    fn default() -> Self {
        Self {
            ip_name_lookup: false,
            udp: true,
            tcp: true,
        }
    }
}

impl AllowedNetworkUses {
    pub(crate) fn check_allowed_udp(&self) -> std::io::Result<()> {
        if !self.udp {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "UDP is not allowed",
            ));
        }

        Ok(())
    }

    pub(crate) fn check_allowed_tcp(&self) -> std::io::Result<()> {
        if !self.tcp {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "TCP is not allowed",
            ));
        }

        Ok(())
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

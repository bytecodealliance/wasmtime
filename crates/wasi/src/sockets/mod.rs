use core::future::Future;
use core::ops::Deref;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use wasmtime::component::{HasData, ResourceTable};

mod udp;
pub(crate) mod util;

pub use udp::UdpSocket;

pub(crate) struct WasiSockets;

impl HasData for WasiSockets {
    type Data<'a> = WasiSocketsCtxView<'a>;
}

/// Value taken from rust std library.
pub(crate) const DEFAULT_TCP_BACKLOG: u32 = 128;

/// Theoretical maximum byte size of a UDP datagram, the real limit is lower,
/// but we do not account for e.g. the transport layer here for simplicity.
/// In practice, datagrams are typically less than 1500 bytes.
pub(crate) const MAX_UDP_DATAGRAM_SIZE: usize = u16::MAX as usize;

#[derive(Clone, Default)]
pub struct WasiSocketsCtx {
    pub(crate) socket_addr_check: SocketAddrCheck,
    pub(crate) allowed_network_uses: AllowedNetworkUses,
}

pub struct WasiSocketsCtxView<'a> {
    pub ctx: &'a mut WasiSocketsCtx,
    pub table: &'a mut ResourceTable,
}

pub trait WasiSocketsView: Send {
    fn sockets(&mut self) -> WasiSocketsCtxView<'_>;
}

#[derive(Copy, Clone)]
pub(crate) struct AllowedNetworkUses {
    pub(crate) ip_name_lookup: bool,
    pub(crate) udp: bool,
    pub(crate) tcp: bool,
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
pub(crate) struct SocketAddrCheck(
    Arc<
        dyn Fn(SocketAddr, SocketAddrUse) -> Pin<Box<dyn Future<Output = bool> + Send + Sync>>
            + Send
            + Sync,
    >,
);

impl SocketAddrCheck {
    /// A check that will be called for each socket address that is used.
    ///
    /// Returning `true` will permit socket connections to the `SocketAddr`,
    /// while returning `false` will reject the connection.
    pub(crate) fn new(
        f: impl Fn(SocketAddr, SocketAddrUse) -> Pin<Box<dyn Future<Output = bool> + Send + Sync>>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self(Arc::new(f))
    }

    pub(crate) async fn check(
        &self,
        addr: SocketAddr,
        reason: SocketAddrUse,
    ) -> std::io::Result<()> {
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

impl Deref for SocketAddrCheck {
    type Target = dyn Fn(SocketAddr, SocketAddrUse) -> Pin<Box<dyn Future<Output = bool> + Send + Sync>>
        + Send
        + Sync;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
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

#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum SocketAddressFamily {
    Ipv4,
    Ipv6,
}

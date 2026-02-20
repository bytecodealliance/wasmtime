use crate::sockets::{SocketAddrCheck, SocketAddressFamily};
use std::net::SocketAddr;
use std::sync::Arc;

pub struct IncomingDatagramStream {
    pub(crate) inner: Arc<tokio::net::UdpSocket>,

    /// If this has a value, the stream is "connected".
    pub(crate) remote_address: Option<SocketAddr>,
}

pub struct OutgoingDatagramStream {
    pub(crate) inner: Arc<tokio::net::UdpSocket>,

    /// If this has a value, the stream is "connected".
    pub(crate) remote_address: Option<SocketAddr>,

    /// Socket address family.
    pub(crate) family: SocketAddressFamily,

    /// The check of allowed addresses
    pub(crate) socket_addr_check: Option<SocketAddrCheck>,
}

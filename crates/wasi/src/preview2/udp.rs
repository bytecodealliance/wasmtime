use crate::preview2::poll::Subscribe;
use crate::preview2::with_ambient_tokio_runtime;
use async_trait::async_trait;
use cap_net_ext::{AddressFamily, Blocking, UdpSocketExt};
use io_lifetimes::raw::{FromRawSocketlike, IntoRawSocketlike};
use std::io;
use std::sync::Arc;
use tokio::io::Interest;

/// The state of a UDP socket.
///
/// This represents the various states a socket can be in during the
/// activities of binding, and connecting.
pub(crate) enum UdpState {
    /// The initial state for a newly-created socket.
    Default,

    /// Binding started via `start_bind`.
    BindStarted,

    /// Binding finished via `finish_bind`. The socket has an address but
    /// is not yet listening for connections.
    Bound,

    /// An outgoing connection is started via `start_connect`.
    Connecting,

    /// An outgoing connection is ready to be established.
    ConnectReady,

    /// An outgoing connection has been established.
    Connected,
}

/// A host UDP socket, plus associated bookkeeping.
///
/// The inner state is wrapped in an Arc because the same underlying socket is
/// used for implementing the stream types.
pub struct UdpSocket {
    /// The part of a `UdpSocket` which is reference-counted so that we
    /// can pass it to async tasks.
    pub(crate) inner: Arc<tokio::net::UdpSocket>,

    /// The current state in the bind/connect progression.
    pub(crate) udp_state: UdpState,

    /// Socket address family.
    pub(crate) family: AddressFamily,
}

#[async_trait]
impl Subscribe for UdpSocket {
    async fn ready(&mut self) {
        // Some states are ready immediately.
        match self.udp_state {
            UdpState::BindStarted => return,
            _ => {}
        }

        // FIXME: Add `Interest::ERROR` when we update to tokio 1.32.
        self.inner
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await
            .expect("failed to await UDP socket readiness");
    }
}

impl UdpSocket {
    /// Create a new socket in the given family.
    pub fn new(family: AddressFamily) -> io::Result<Self> {
        // Create a new host socket and set it to non-blocking, which is needed
        // by our async implementation.
        let udp_socket = cap_std::net::UdpSocket::new(family, Blocking::No)?;
        Self::from_udp_socket(udp_socket, family)
    }

    pub fn from_udp_socket(
        udp_socket: cap_std::net::UdpSocket,
        family: AddressFamily,
    ) -> io::Result<Self> {
        let fd = udp_socket.into_raw_socketlike();
        let std_socket = unsafe { std::net::UdpSocket::from_raw_socketlike(fd) };
        let socket = with_ambient_tokio_runtime(|| tokio::net::UdpSocket::try_from(std_socket))?;
        Ok(Self {
            inner: Arc::new(socket),
            udp_state: UdpState::Default,
            family,
        })
    }

    pub fn udp_socket(&self) -> &tokio::net::UdpSocket {
        &self.inner
    }
}

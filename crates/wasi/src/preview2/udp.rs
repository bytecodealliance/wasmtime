use crate::preview2::poll::Subscribe;
use crate::preview2::with_ambient_tokio_runtime;
use async_trait::async_trait;
use cap_net_ext::{AddressFamily, Blocking, UdpSocketExt};
use io_lifetimes::raw::{FromRawSocketlike, IntoRawSocketlike};
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
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

    /// A connect call is in progress.
    Connecting(SocketAddr),

    /// The socket is "connected" to a peer address.
    Connected(SocketAddr),
}

/// Operational data shared between the UdpSocket, IncomingDatagramStream & OutgoingDatagramStream
pub(crate) struct UdpSocketInner {
    pub(crate) native_socket: Arc<tokio::net::UdpSocket>,

    /// The current state in the bind/connect progression.
    pub(crate) udp_state: UdpState,

    /// Socket address family.
    pub(crate) family: AddressFamily,
}

/// A host UDP socket.
pub struct UdpSocket {
    pub(crate) inner: Arc<Mutex<UdpSocketInner>>,
}

#[async_trait]
impl Subscribe for UdpSocket {
    async fn ready(&mut self) {
        // None of the socket-level operations block natively
    }
}

impl UdpSocket {
    /// Create a new socket in the given family.
    pub fn new(family: AddressFamily) -> io::Result<Self> {
        let inner = UdpSocketInner {
            native_socket: Arc::new(Self::new_tokio_socket(family)?),
            udp_state: UdpState::Default,
            family,
        };

        Ok(UdpSocket {
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    fn new_tokio_socket(family: AddressFamily) -> io::Result<tokio::net::UdpSocket> {
        // Create a new host socket and set it to non-blocking, which is needed
        // by our async implementation.
        let cap_std_socket = cap_std::net::UdpSocket::new(family, Blocking::No)?;
        let fd = cap_std_socket.into_raw_socketlike();
        let std_socket = unsafe { std::net::UdpSocket::from_raw_socketlike(fd) };
        let tokio_socket =
            with_ambient_tokio_runtime(|| tokio::net::UdpSocket::try_from(std_socket))?;

        Ok(tokio_socket)
    }

    pub(crate) fn new_incoming_stream(&self) -> IncomingDatagramStream {
        IncomingDatagramStream {
            inner: self.inner.clone(),
        }
    }

    pub(crate) fn new_outgoing_stream(&self) -> OutgoingDatagramStream {
        OutgoingDatagramStream {
            inner: self.inner.clone(),
        }
    }
}

impl UdpSocketInner {
    pub fn remote_address(&self) -> Option<SocketAddr> {
        match self.udp_state {
            UdpState::Connected(addr) => Some(addr),
            UdpState::Connecting(_) // Don't use address. From the consumer's perspective connecting isn't finished yet.
            | _ => None,
        }
    }

    pub fn udp_socket(&self) -> &tokio::net::UdpSocket {
        &self.native_socket
    }
}

pub struct IncomingDatagramStream {
    pub(crate) inner: Arc<Mutex<UdpSocketInner>>,
}

#[async_trait]
impl Subscribe for IncomingDatagramStream {
    async fn ready(&mut self) {
        let native_socket = {
            // Make sure the lock guard is released before the await.
            let inner = self.inner.lock().unwrap();
            inner.native_socket.clone()
        };

        // FIXME: Add `Interest::ERROR` when we update to tokio 1.32.
        native_socket
            .ready(Interest::READABLE)
            .await
            .expect("failed to await UDP socket readiness");
    }
}

pub struct OutgoingDatagramStream {
    pub(crate) inner: Arc<Mutex<UdpSocketInner>>,
}

#[async_trait]
impl Subscribe for OutgoingDatagramStream {
    async fn ready(&mut self) {
        let native_socket = {
            // Make sure the lock guard is released before the await.
            let inner = self.inner.lock().unwrap();
            inner.native_socket.clone()
        };

        // FIXME: Add `Interest::ERROR` when we update to tokio 1.32.
        native_socket
            .ready(Interest::WRITABLE)
            .await
            .expect("failed to await UDP socket readiness");
    }
}

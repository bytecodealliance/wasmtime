use crate::preview2::host::network::util;
use crate::preview2::with_ambient_tokio_runtime;
use async_trait::async_trait;
use cap_net_ext::Blocking;
use io_lifetimes::raw::{FromRawSocketlike, IntoRawSocketlike};
use rustix::io::Errno;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use super::network::{SocketAddrCheck, SocketAddrFamily, SocketAddrUse};

pub type OutgoingDatagramStream = Box<dyn OutgoingStream + Send + Sync + 'static>;
pub type IncomingDatagramStream = Box<dyn IncomingStream + Send + Sync + 'static>;

/// A WASI-compliant UDP socket.
///
/// Implementations are encouraged to wrap [SystemUdpSocket] as that provides a
/// cross-platform and WASI-compliant base implementation.
pub trait UdpSocket {
    /// Bind the socket to a specific network on the provided IP address and port.
    ///
    /// If the IP address is zero (`0.0.0.0` in IPv4, `::` in IPv6), it is left to the implementation to decide which
    /// network interface(s) to bind to.
    /// If the TCP/UDP port is zero, the socket will be bound to a random free port.
    ///
    /// The `local_address` must be of the same `SocketAddrFamily` as the family
    /// the socket was created with.
    fn bind(&self, local_address: SocketAddr) -> io::Result<()>;

    /// Connect to a remote endpoint.
    ///
    /// An [io::ErrorKind::InvalidInput] error is returned when `remote_address`:
    /// - is not of the same `SocketAddrFamily` as the family the socket was created with,
    /// - contains an [unspecified](std::net::IpAddr::is_unspecified) IP address.
    /// - has the port set to 0.
    fn connect(&self, remote_address: SocketAddr) -> io::Result<()>;

    /// Get the streams for sending and receiving datagrams.
    fn streams(&self) -> (IncomingDatagramStream, OutgoingDatagramStream);

    /// Disconnect from the remote endpoint.
    fn disconnect(&self) -> io::Result<()>;

    /// Get the bound local address.
    /// The returned value will always be of the same `SocketAddrFamily` as the
    /// the family the socket was created with.
    fn local_address(&self) -> io::Result<SocketAddr>;

    /// Get the remote address.
    /// The returned value will always be of the same `SocketAddrFamily` as the
    /// the family the socket was created with.
    fn remote_address(&self) -> io::Result<SocketAddr>;

    /// Whether this is a IPv4 or IPv6 socket.
    ///
    /// Equivalent to the SO_DOMAIN socket option.
    fn address_family(&self) -> SocketAddrFamily;

    /// Equivalent to the IP_TTL & IPV6_UNICAST_HOPS socket options.
    /// This function never returns 0.
    fn hop_limit(&self) -> io::Result<u8>;

    /// Equivalent to the IP_TTL & IPV6_UNICAST_HOPS socket options.
    ///
    /// If the provided value is 0, an [io::ErrorKind::InvalidInput] error is returned.
    fn set_hop_limit(&self, value: u8) -> io::Result<()>;

    /// The kernel buffer space reserved for receives on this socket.
    /// This function never returns 0.
    /// Equivalent to the SO_RCVBUF socket options.
    fn receive_buffer_size(&self) -> io::Result<usize>;

    /// The kernel buffer space reserved for receives on this socket.
    ///
    /// If the provided value is 0, an [io::ErrorKind::InvalidInput] error is returned.
    /// Any other value will never cause an error, but it might be silently clamped and/or rounded.
    /// I.e. after setting a value, reading the same setting back may return a different value.
    ///
    /// Equivalent to the SO_RCVBUF socket options.
    fn set_receive_buffer_size(&self, value: usize) -> io::Result<()>;

    /// The kernel buffer space reserved for sends on this socket.
    /// This function never returns 0.
    /// Equivalent to the SO_SNDBUF socket options.
    fn send_buffer_size(&self) -> io::Result<usize>;

    /// The kernel buffer space reserved for sends on this socket.
    ///
    /// If the provided value is 0, an [io::ErrorKind::InvalidInput] error is returned.
    /// Any other value will never cause an error, but it might be silently clamped and/or rounded.
    /// I.e. after setting a value, reading the same setting back may return a different value.
    ///
    /// Equivalent to the SO_SNDBUF socket options.
    fn set_send_buffer_size(&self, value: usize) -> io::Result<()>;
}

#[async_trait]
pub trait OutgoingStream {
    /// Send the given data to the target.
    fn send(&self, buf: &[u8], target: SocketAddr) -> io::Result<usize>;

    /// Wait until the stream is ready to send data.
    async fn ready(&self);
}

#[async_trait]
pub trait IncomingStream {
    /// Receive data into the given buffer.
    ///
    /// Returns the number of bytes received and the address of the remote peer.
    fn recv(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>;

    /// Wait until the stream is ready to receive data.
    async fn ready(&self);
}

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

    /// The socket is "connected" to a peer address.
    Connected,
}

/// A host UDP socket, plus address checks.
pub struct DefaultUdpSocket {
    /// The underlying system socket
    system: SystemUdpSocket,
    /// The check of allowed addresses
    addr_check: SocketAddrCheck,
}

impl DefaultUdpSocket {
    /// Create a new socket in the given family.
    pub fn new(system: SystemUdpSocket, socket_addr_check: SocketAddrCheck) -> Self {
        DefaultUdpSocket {
            system,
            addr_check: socket_addr_check,
        }
    }
}

impl UdpSocket for DefaultUdpSocket {
    fn bind(&self, local_address: SocketAddr) -> io::Result<()> {
        self.addr_check
            .check(&local_address, SocketAddrUse::UdpBind)?;
        self.system.bind(local_address)
    }

    fn connect(&self, remote_address: SocketAddr) -> io::Result<()> {
        self.addr_check
            .check(&remote_address, SocketAddrUse::UdpConnect)?;
        self.system.connect(remote_address)
    }

    fn streams(&self) -> (IncomingDatagramStream, OutgoingDatagramStream) {
        let (incoming, outgoing) = self.system.streams();
        (
            // Just use the system stream for incoming data
            incoming,
            Box::new(DefaultOutgoingStream {
                system: outgoing,
                addr_check: self.addr_check.clone(),
            }),
        )
    }

    fn disconnect(&self) -> io::Result<()> {
        self.system.disconnect()
    }

    fn local_address(&self) -> io::Result<SocketAddr> {
        self.system.local_address()
    }

    fn remote_address(&self) -> io::Result<SocketAddr> {
        self.system.remote_address()
    }

    fn address_family(&self) -> SocketAddrFamily {
        self.system.address_family()
    }

    fn hop_limit(&self) -> io::Result<u8> {
        self.system.hop_limit()
    }

    fn set_hop_limit(&self, value: u8) -> io::Result<()> {
        self.system.set_hop_limit(value)
    }

    fn receive_buffer_size(&self) -> io::Result<usize> {
        self.system.receive_buffer_size()
    }

    fn set_receive_buffer_size(&self, value: usize) -> io::Result<()> {
        self.system.set_receive_buffer_size(value)
    }

    fn send_buffer_size(&self) -> io::Result<usize> {
        self.system.send_buffer_size()
    }

    fn set_send_buffer_size(&self, value: usize) -> io::Result<()> {
        self.system.set_send_buffer_size(value)
    }
}

pub struct SystemUdpSocket {
    /// The part of a `UdpSocket` which is reference-counted so that we
    /// can pass it to async tasks.
    pub(crate) inner: Arc<tokio::net::UdpSocket>,

    /// Socket address family.
    pub(crate) family: SocketAddrFamily,
}

impl SystemUdpSocket {
    /// Create a new socket in the given family.
    pub fn new(family: SocketAddrFamily) -> io::Result<Self> {
        // Create a new host socket and set it to non-blocking, which is needed
        // by our async implementation.
        let fd = util::udp_socket(family, Blocking::No)?;

        if family == SocketAddrFamily::V6 {
            rustix::net::sockopt::set_ipv6_v6only(&fd, true)?;
        }

        let socket = Self::setup_tokio_udp_socket(fd)?;

        Ok(Self {
            inner: Arc::new(socket),
            family,
        })
    }

    fn setup_tokio_udp_socket(fd: rustix::fd::OwnedFd) -> io::Result<tokio::net::UdpSocket> {
        let std_socket =
            unsafe { std::net::UdpSocket::from_raw_socketlike(fd.into_raw_socketlike()) };
        with_ambient_tokio_runtime(|| tokio::net::UdpSocket::try_from(std_socket))
    }

    pub fn udp_socket(&self) -> &tokio::net::UdpSocket {
        &self.inner
    }
}

impl UdpSocket for SystemUdpSocket {
    fn bind(&self, local_address: SocketAddr) -> io::Result<()> {
        util::validate_address_family(&local_address, &self.family)?;

        // Perform the OS bind call.
        util::udp_bind(self.udp_socket(), &local_address).map_err(|error| match error {
            // From https://pubs.opengroup.org/onlinepubs/9699919799/functions/bind.html:
            // > [EAFNOSUPPORT] The specified address is not a valid address for the address family of the specified socket
            //
            // The most common reasons for this error should have already
            // been handled by our own validation slightly higher up in this
            // function. This error mapping is here just in case there is
            // an edge case we didn't catch.
            Errno::AFNOSUPPORT => Errno::INVAL,
            _ => error,
        })?;
        Ok(())
    }

    fn connect(&self, remote_address: SocketAddr) -> io::Result<()> {
        util::validate_remote_address(&remote_address)?;
        util::validate_address_family(&remote_address, &self.family)?;

        rustix::net::connect(self.udp_socket(), &remote_address).map_err(|error| match error {
            Errno::AFNOSUPPORT => Errno::INVAL, // See `bind` implementation.
            Errno::INPROGRESS => {
                log::debug!("UDP connect returned EINPROGRESS, which should never happen");
                todo!()
            }
            e => e,
        })?;

        Ok(())
    }

    fn streams(&self) -> (IncomingDatagramStream, OutgoingDatagramStream) {
        (
            Box::new(SystemIncomingStream {
                inner: self.inner.clone(),
            }),
            Box::new(SystemOutgoingStream {
                inner: self.inner.clone(),
                family: self.family,
            }),
        )
    }

    fn disconnect(&self) -> io::Result<()> {
        Ok(util::udp_disconnect(self.udp_socket())?)
    }

    fn local_address(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    fn remote_address(&self) -> io::Result<SocketAddr> {
        self.inner.peer_addr()
    }

    fn address_family(&self) -> SocketAddrFamily {
        self.family
    }

    fn hop_limit(&self) -> io::Result<u8> {
        let ttl = match self.family {
            SocketAddrFamily::V4 => util::get_ip_ttl(self.udp_socket())?,
            SocketAddrFamily::V6 => util::get_ipv6_unicast_hops(self.udp_socket())?,
        };
        Ok(ttl)
    }

    fn set_hop_limit(&self, value: u8) -> io::Result<()> {
        match self.family {
            SocketAddrFamily::V4 => util::set_ip_ttl(self.udp_socket(), value)?,
            SocketAddrFamily::V6 => util::set_ipv6_unicast_hops(self.udp_socket(), value)?,
        }
        Ok(())
    }

    fn receive_buffer_size(&self) -> io::Result<usize> {
        let value = util::get_socket_recv_buffer_size(self.udp_socket())?;
        Ok(value)
    }

    fn set_receive_buffer_size(&self, value: usize) -> io::Result<()> {
        util::set_socket_recv_buffer_size(self.udp_socket(), value)?;
        Ok(())
    }

    fn send_buffer_size(&self) -> io::Result<usize> {
        let value = util::get_socket_send_buffer_size(self.udp_socket())?;
        Ok(value)
    }

    fn set_send_buffer_size(&self, value: usize) -> io::Result<()> {
        util::set_socket_send_buffer_size(self.udp_socket(), value)?;
        Ok(())
    }
}

pub struct SystemIncomingStream {
    pub(crate) inner: Arc<tokio::net::UdpSocket>,
}

#[async_trait]
impl IncomingStream for SystemIncomingStream {
    fn recv(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.inner.try_recv_from(buf)
    }

    async fn ready(&self) {
        // FIXME: Add `Interest::ERROR` when we update to tokio 1.32.
        self.inner
            .ready(tokio::io::Interest::READABLE)
            .await
            .expect("failed to await UDP socket readiness");
    }
}

pub struct SystemOutgoingStream {
    pub(crate) inner: Arc<tokio::net::UdpSocket>,
    family: SocketAddrFamily,
}

#[async_trait]
impl OutgoingStream for SystemOutgoingStream {
    fn send(&self, buf: &[u8], target: SocketAddr) -> io::Result<usize> {
        util::validate_remote_address(&target)?;
        util::validate_address_family(&target, &self.family)?;
        self.inner.try_send_to(buf, target)
    }

    async fn ready(&self) {
        // FIXME: Add `Interest::ERROR` when we update to tokio 1.32.
        self.inner
            .ready(tokio::io::Interest::WRITABLE)
            .await
            .expect("failed to await UDP socket readiness");
    }
}

struct DefaultOutgoingStream {
    system: OutgoingDatagramStream,
    addr_check: SocketAddrCheck,
}

#[async_trait]
impl OutgoingStream for DefaultOutgoingStream {
    fn send(&self, buf: &[u8], target: SocketAddr) -> io::Result<usize> {
        self.addr_check
            .check(&target, SocketAddrUse::UdpOutgoingDatagram)?;
        self.system.send(buf, target)
    }

    async fn ready(&self) {
        self.system.ready().await
    }
}

use crate::runtime::with_ambient_tokio_runtime;
use crate::sockets::util::{
    ErrorCode, get_unicast_hop_limit, is_valid_address_family, is_valid_remote_address,
    receive_buffer_size, send_buffer_size, set_receive_buffer_size, set_send_buffer_size,
    set_unicast_hop_limit, udp_bind, udp_connect, udp_disconnect, udp_socket, unspecified_addr,
};
use crate::sockets::{
    MAX_UDP_DATAGRAM_SIZE, SocketAddrCheck, SocketAddrUse, SocketAddressFamily, WasiSocketsCtx,
};
use rustix::io::Errno;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::debug;

/// A host UDP socket, plus associated bookkeeping.
///
/// The inner state is wrapped in an Arc because the same underlying socket is
/// used for implementing the stream types.
pub struct UdpSocket {
    socket: Arc<tokio::net::UdpSocket>,
    family: SocketAddressFamily,

    /// The checks to perform before doing any noteworthy syscall.
    permissions: SocketAddrCheck,

    /// Cached value of whether the socket is bound. This is cached to avoid
    /// redundant syscalls in every `send` & `receive`.
    is_bound: bool,

    /// Cached value of the remote address. This is cached to avoid redundant
    /// syscalls in every `send`.
    remote_addr: Option<SocketAddr>,
}

impl UdpSocket {
    /// Create a new socket in the given family.
    pub(crate) async fn new(
        cx: &WasiSocketsCtx,
        family: SocketAddressFamily,
    ) -> Result<Self, ErrorCode> {
        cx.allowed_network_uses.check_allowed_udp()?;

        let socket = with_ambient_tokio_runtime(|| {
            // Manually construct a new socket, because neither std nor tokio
            // provides a way to create an unbound UDP socket.
            let fd = udp_socket(family)?;
            if family == SocketAddressFamily::Ipv6 {
                rustix::net::sockopt::set_ipv6_v6only(&fd, true)?;
            }
            tokio::net::UdpSocket::try_from(std::net::UdpSocket::from(fd))
        })?;

        // Native UDP sockets are immediately writable after creation and
        // existing guest code out in the wild depends on that. However, due to
        // the way Tokio is structured internally, a newly created Tokio socket
        // starts out as "not writable" and a background tokio thread updates
        // this state asynchronously soon after. Some more info in this thread:
        // https://github.com/bytecodealliance/wasmtime/issues/12612#issuecomment-3923714174
        //
        // To prevent exposing guests to this race condition, we wait for Tokio
        // to finish its internal setup:
        socket.writable().await?;

        Ok(Self {
            socket: Arc::new(socket),
            is_bound: false,
            remote_addr: None,
            permissions: cx.socket_addr_check.clone(),
            family,
        })
    }

    pub(crate) fn is_bound(&mut self) -> bool {
        // Once bound, a UDP socket can never become unbound again. So we can
        // skip all work after a previous call has already determined the
        // socket to be bound.
        if !self.is_bound {
            self.is_bound = self
                .socket
                .local_addr()
                .is_ok_and(|addr| addr != unspecified_addr(self.family));
        }
        self.is_bound
    }

    pub(crate) fn local_address(&mut self) -> Result<SocketAddr, ErrorCode> {
        if !self.is_bound() {
            return Err(ErrorCode::InvalidState);
        }
        self.socket.local_addr().map_err(|e| e.into())
    }

    pub(crate) fn remote_address(&mut self) -> Result<SocketAddr, ErrorCode> {
        self.remote_addr.ok_or(ErrorCode::InvalidState)
    }

    pub(crate) fn is_connected(&mut self) -> bool {
        self.remote_addr.is_some()
    }

    pub(crate) async fn bind(&mut self, addr: SocketAddr) -> Result<(), ErrorCode> {
        if self.is_bound() {
            return Err(ErrorCode::InvalidState);
        }
        if !is_valid_address_family(addr.ip(), self.family) {
            return Err(ErrorCode::InvalidArgument);
        }

        self.permissions.check(addr, SocketAddrUse::UdpBind).await?;

        udp_bind(&self.socket, addr)?;
        Ok(())
    }

    pub(crate) async fn connect(&mut self, addr: SocketAddr) -> Result<(), ErrorCode> {
        if !is_valid_address_family(addr.ip(), self.family) || !is_valid_remote_address(addr) {
            return Err(ErrorCode::InvalidArgument);
        }

        // Perform all permission checks before doing any syscalls.
        {
            if !self.is_bound() {
                // If not explicitly bound, the OS will implicitly bind the
                // socket to an ephemeral port when connecting.
                let implicit_bind_addr = unspecified_addr(self.family);
                self.permissions
                    .check(implicit_bind_addr, SocketAddrUse::UdpBind)
                    .await?;
            }

            // On UDP sockets, "connecting" is just a local operation that sets the
            // default remote address for future sends and receives. It does not
            // actually do any I/O on its own. We'll allow the `connect` call
            // if the address is permitted for sending or receiving.
            if self
                .permissions
                .check(addr, SocketAddrUse::UdpSend)
                .await
                .is_err()
            {
                self.permissions
                    .check(addr, SocketAddrUse::UdpReceive)
                    .await?;
            }
        }

        let result = udp_connect(&self.socket, addr).map_err(|e| match e {
            // The most common reason for AFNOSUPPORT is an invalid address
            // family. This should have already been handled by our own
            // validation slightly higher up in this function. This error
            // mapping is here just in case there is an edge case we didn't catch.
            Errno::AFNOSUPPORT => ErrorCode::InvalidArgument,
            Errno::INPROGRESS => {
                debug!("UDP connect returned EINPROGRESS, which should never happen");
                ErrorCode::Other
            }
            err => err.into(),
        });
        self.update_remote_address();
        result
    }

    pub(crate) fn disconnect(&mut self) -> Result<(), ErrorCode> {
        if !self.is_connected() {
            return Err(ErrorCode::InvalidState);
        }

        // On Linux, disconnecting a UDP socket relinquishes its local port
        // assignment in some cases. If the socket was bound to the wildcard
        // address, its local address will then read `0.0.0.0:0` or `[::]:0`
        // which is indistinguishable from an unbound socket. To ensure
        // `is_bound()` will continue to return `true` after the disconnect, we
        // manually settle the `is_bound` state here:
        self.is_bound = true;

        let result = udp_disconnect(&self.socket).map_err(|e| e.into());
        self.update_remote_address();
        result
    }

    /// Update our internal bookkeeping based on the actual state of the socket.
    /// This should be called after any operation that may change the remote
    /// address.
    fn update_remote_address(&mut self) {
        self.remote_addr = if let Ok(addr) = self.socket.peer_addr()
            && addr != unspecified_addr(self.family)
        {
            Some(addr)
        } else {
            None
        }
    }

    pub(crate) fn send(
        &mut self,
        data: Vec<u8>,
        addr: Option<SocketAddr>,
    ) -> impl Future<Output = Result<(), ErrorCode>> + Send + use<> {
        let family = self.family;
        let socket = self.socket.clone();
        let permissions = self.permissions.clone();
        let connected_addr = self.remote_address().ok();
        let is_bound = self.is_bound();

        async move {
            if data.len() > MAX_UDP_DATAGRAM_SIZE {
                return Err(ErrorCode::DatagramTooLarge);
            }

            let effective_addr = if let Some(addr) = addr {
                if !is_valid_remote_address(addr) || !is_valid_address_family(addr.ip(), family) {
                    return Err(ErrorCode::InvalidArgument);
                }

                // If the socket is connected, the provided address must match the
                // connected address.
                if connected_addr.is_some() && connected_addr != Some(addr) {
                    return Err(ErrorCode::InvalidArgument);
                }

                addr
            } else if let Some(connected_addr) = connected_addr {
                connected_addr
            } else {
                return Err(ErrorCode::InvalidArgument);
            };

            // Perform all permission checks before doing any syscalls.
            {
                if !is_bound {
                    // If not explicitly bound, the OS will implicitly bind the
                    // socket to an ephemeral port when sending.
                    let implicit_bind_addr = unspecified_addr(family);
                    permissions
                        .check(implicit_bind_addr, SocketAddrUse::UdpBind)
                        .await?;
                }

                permissions
                    .check(effective_addr, SocketAddrUse::UdpSend)
                    .await?;
            }

            if connected_addr == Some(effective_addr) {
                socket.send(&data).await?;
            } else {
                socket.send_to(&data, effective_addr).await?;
            }

            Ok(())
        }
    }

    pub(crate) fn recv(
        &mut self,
    ) -> impl Future<Output = Result<(Vec<u8>, SocketAddr), ErrorCode>> + Send + use<> {
        let socket = self.socket.clone();
        let permissions = self.permissions.clone();
        let is_bound = self.is_bound();

        async move {
            if !is_bound {
                return Err(ErrorCode::InvalidState);
            }

            loop {
                let mut data = vec![0; super::MAX_UDP_DATAGRAM_SIZE];
                let (len, addr) = socket.recv_from(&mut data).await?;
                data.truncate(len);

                match permissions.check(addr, SocketAddrUse::UdpReceive).await {
                    Ok(()) => return Ok((data, addr)),
                    Err(_) => {
                        // Not allowed. Drop the packet and poll again.
                        continue;
                    }
                }
            }
        }
    }

    pub(crate) fn address_family(&self) -> SocketAddressFamily {
        self.family
    }

    pub(crate) fn unicast_hop_limit(&self) -> Result<u8, ErrorCode> {
        let n = get_unicast_hop_limit(&self.socket, self.family)?;
        Ok(n)
    }

    pub(crate) fn set_unicast_hop_limit(&self, value: u8) -> Result<(), ErrorCode> {
        set_unicast_hop_limit(&self.socket, self.family, value)?;
        Ok(())
    }

    pub(crate) fn receive_buffer_size(&self) -> Result<u64, ErrorCode> {
        let n = receive_buffer_size(&self.socket)?;
        Ok(n)
    }

    pub(crate) fn set_receive_buffer_size(&self, value: u64) -> Result<(), ErrorCode> {
        set_receive_buffer_size(&self.socket, value)?;
        Ok(())
    }

    pub(crate) fn send_buffer_size(&self) -> Result<u64, ErrorCode> {
        let n = send_buffer_size(&self.socket)?;
        Ok(n)
    }

    pub(crate) fn set_send_buffer_size(&self, value: u64) -> Result<(), ErrorCode> {
        set_send_buffer_size(&self.socket, value)?;
        Ok(())
    }
}

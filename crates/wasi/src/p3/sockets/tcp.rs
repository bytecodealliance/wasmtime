use core::fmt::Debug;
use core::mem;
use core::net::SocketAddr;

use std::sync::Arc;

use cap_net_ext::AddressFamily;
use io_lifetimes::AsSocketlike as _;
use io_lifetimes::views::SocketlikeView;
use rustix::net::sockopt;

use crate::p3::bindings::sockets::types::{Duration, ErrorCode, IpAddressFamily, IpSocketAddress};
use crate::runtime::with_ambient_tokio_runtime;
use crate::sockets::util::{
    get_unicast_hop_limit, is_valid_address_family, is_valid_unicast_address, receive_buffer_size,
    send_buffer_size, set_keep_alive_count, set_keep_alive_idle_time, set_keep_alive_interval,
    set_receive_buffer_size, set_send_buffer_size, set_unicast_hop_limit, tcp_bind,
};
use crate::sockets::{DEFAULT_TCP_BACKLOG, SocketAddressFamily};

/// The state of a TCP socket.
///
/// This represents the various states a socket can be in during the
/// activities of binding, listening, accepting, and connecting.
pub enum TcpState {
    /// The initial state for a newly-created socket.
    Default(tokio::net::TcpSocket),

    /// Binding finished. The socket has an address but is not yet listening for connections.
    Bound(tokio::net::TcpSocket),

    /// The socket is now listening and waiting for an incoming connection.
    Listening(Arc<tokio::net::TcpListener>),

    /// An outgoing connection is started.
    Connecting,

    /// A connection has been established.
    Connected(Arc<tokio::net::TcpStream>),

    /// A connection has been established and `receive` has been called.
    Receiving(Arc<tokio::net::TcpStream>),

    Error(ErrorCode),

    Closed,
}

impl Debug for TcpState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default(_) => f.debug_tuple("Default").finish(),
            Self::Bound(_) => f.debug_tuple("Bound").finish(),
            Self::Listening { .. } => f.debug_tuple("Listening").finish(),
            Self::Connecting => f.debug_tuple("Connecting").finish(),
            Self::Connected { .. } => f.debug_tuple("Connected").finish(),
            Self::Receiving { .. } => f.debug_tuple("Receiving").finish(),
            Self::Error(..) => f.debug_tuple("Error").finish(),
            Self::Closed => write!(f, "Closed"),
        }
    }
}

/// A host TCP socket, plus associated bookkeeping.
pub struct TcpSocket {
    /// The current state in the bind/listen/accept/connect progression.
    pub tcp_state: TcpState,

    /// The desired listen queue size.
    pub listen_backlog_size: u32,

    pub family: SocketAddressFamily,

    // The socket options below are not automatically inherited from the listener
    // on all platforms. So we keep track of which options have been explicitly
    // set and manually apply those values to newly accepted clients.
    #[cfg(target_os = "macos")]
    pub receive_buffer_size: Arc<core::sync::atomic::AtomicUsize>,
    #[cfg(target_os = "macos")]
    pub send_buffer_size: Arc<core::sync::atomic::AtomicUsize>,
    #[cfg(target_os = "macos")]
    pub hop_limit: Arc<core::sync::atomic::AtomicU8>,
    #[cfg(target_os = "macos")]
    pub keep_alive_idle_time: Arc<core::sync::atomic::AtomicU64>, // nanoseconds
}

impl TcpSocket {
    /// Create a new socket in the given family.
    pub fn new(family: AddressFamily) -> std::io::Result<Self> {
        with_ambient_tokio_runtime(|| {
            let (socket, family) = match family {
                AddressFamily::Ipv4 => {
                    let socket = tokio::net::TcpSocket::new_v4()?;
                    (socket, SocketAddressFamily::Ipv4)
                }
                AddressFamily::Ipv6 => {
                    let socket = tokio::net::TcpSocket::new_v6()?;
                    sockopt::set_ipv6_v6only(&socket, true)?;
                    (socket, SocketAddressFamily::Ipv6)
                }
            };

            Ok(Self::from_state(TcpState::Default(socket), family))
        })
    }

    /// Create a `TcpSocket` from an existing socket.
    pub fn from_state(state: TcpState, family: SocketAddressFamily) -> Self {
        Self {
            tcp_state: state,
            listen_backlog_size: DEFAULT_TCP_BACKLOG,
            family,
            #[cfg(target_os = "macos")]
            receive_buffer_size: Arc::default(),
            #[cfg(target_os = "macos")]
            send_buffer_size: Arc::default(),
            #[cfg(target_os = "macos")]
            hop_limit: Arc::default(),
            #[cfg(target_os = "macos")]
            keep_alive_idle_time: Arc::default(),
        }
    }

    pub fn as_std_view(&self) -> Result<SocketlikeView<'_, std::net::TcpStream>, ErrorCode> {
        match &self.tcp_state {
            TcpState::Default(socket) | TcpState::Bound(socket) => Ok(socket.as_socketlike_view()),
            TcpState::Connected(stream) | TcpState::Receiving(stream) => {
                Ok(stream.as_socketlike_view())
            }
            TcpState::Listening(listener) => Ok(listener.as_socketlike_view()),
            TcpState::Connecting | TcpState::Closed => Err(ErrorCode::InvalidState),
            TcpState::Error(err) => Err(*err),
        }
    }

    pub fn bind(&mut self, addr: SocketAddr) -> Result<(), ErrorCode> {
        let ip = addr.ip();
        if !is_valid_unicast_address(ip) || !is_valid_address_family(ip, self.family) {
            return Err(ErrorCode::InvalidArgument);
        }
        match mem::replace(&mut self.tcp_state, TcpState::Closed) {
            TcpState::Default(sock) => {
                if let Err(err) = tcp_bind(&sock, addr) {
                    self.tcp_state = TcpState::Default(sock);
                    Err(err.into())
                } else {
                    self.tcp_state = TcpState::Bound(sock);
                    Ok(())
                }
            }
            tcp_state => {
                self.tcp_state = tcp_state;
                Err(ErrorCode::InvalidState)
            }
        }
    }

    pub fn local_address(&self) -> Result<IpSocketAddress, ErrorCode> {
        match &self.tcp_state {
            TcpState::Bound(socket) => {
                let addr = socket.local_addr()?;
                Ok(addr.into())
            }
            TcpState::Connected(stream) | TcpState::Receiving(stream) => {
                let addr = stream.local_addr()?;
                Ok(addr.into())
            }
            TcpState::Listening(listener) => {
                let addr = listener.local_addr()?;
                Ok(addr.into())
            }
            TcpState::Error(err) => Err(*err),
            _ => Err(ErrorCode::InvalidState),
        }
    }

    pub fn remote_address(&self) -> Result<IpSocketAddress, ErrorCode> {
        match &self.tcp_state {
            TcpState::Connected(stream) | TcpState::Receiving(stream) => {
                let addr = stream.peer_addr()?;
                Ok(addr.into())
            }
            TcpState::Error(err) => Err(*err),
            _ => Err(ErrorCode::InvalidState),
        }
    }

    pub fn is_listening(&self) -> bool {
        matches!(self.tcp_state, TcpState::Listening { .. })
    }

    pub fn address_family(&self) -> IpAddressFamily {
        match self.family {
            SocketAddressFamily::Ipv4 => IpAddressFamily::Ipv4,
            SocketAddressFamily::Ipv6 => IpAddressFamily::Ipv6,
        }
    }

    pub fn set_listen_backlog_size(&mut self, value: u64) -> Result<(), ErrorCode> {
        const MIN_BACKLOG: u32 = 1;
        const MAX_BACKLOG: u32 = i32::MAX as u32; // OS'es will most likely limit it down even further.

        if value == 0 {
            return Err(ErrorCode::InvalidArgument);
        }
        // Silently clamp backlog size. This is OK for us to do, because operating systems do this too.
        let value = value
            .try_into()
            .unwrap_or(MAX_BACKLOG)
            .clamp(MIN_BACKLOG, MAX_BACKLOG);
        match &self.tcp_state {
            TcpState::Default(..) | TcpState::Bound(..) => {
                // Socket not listening yet. Stash value for first invocation to `listen`.
                self.listen_backlog_size = value;
                Ok(())
            }
            TcpState::Listening(listener) => {
                // Try to update the backlog by calling `listen` again.
                // Not all platforms support this. We'll only update our own value if the OS supports changing the backlog size after the fact.
                if rustix::net::listen(&listener, value.try_into().unwrap_or(i32::MAX)).is_err() {
                    return Err(ErrorCode::NotSupported);
                }
                self.listen_backlog_size = value;
                Ok(())
            }
            TcpState::Error(err) => Err(*err),
            _ => Err(ErrorCode::InvalidState),
        }
    }

    pub fn keep_alive_enabled(&self) -> Result<bool, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let v = sockopt::socket_keepalive(fd)?;
        Ok(v)
    }

    pub fn set_keep_alive_enabled(&self, value: bool) -> Result<(), ErrorCode> {
        let fd = &*self.as_std_view()?;
        sockopt::set_socket_keepalive(fd, value)?;
        Ok(())
    }

    pub fn keep_alive_idle_time(&self) -> Result<Duration, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let v = sockopt::tcp_keepidle(fd)?;
        Ok(v.as_nanos().try_into().unwrap_or(u64::MAX))
    }

    pub fn set_keep_alive_idle_time(&mut self, value: Duration) -> Result<(), ErrorCode> {
        let fd = &*self.as_std_view()?;
        #[cfg_attr(not(target_os = "macos"), expect(unused))]
        let value = set_keep_alive_idle_time(fd, value)?;
        #[cfg(target_os = "macos")]
        {
            self.keep_alive_idle_time
                .store(value, core::sync::atomic::Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn keep_alive_interval(&self) -> Result<Duration, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let v = sockopt::tcp_keepintvl(fd)?;
        Ok(v.as_nanos().try_into().unwrap_or(u64::MAX))
    }

    pub fn set_keep_alive_interval(&self, value: Duration) -> Result<(), ErrorCode> {
        let fd = &*self.as_std_view()?;
        set_keep_alive_interval(fd, core::time::Duration::from_nanos(value))?;
        Ok(())
    }

    pub fn keep_alive_count(&self) -> Result<u32, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let v = sockopt::tcp_keepcnt(fd)?;
        Ok(v)
    }

    pub fn set_keep_alive_count(&self, value: u32) -> Result<(), ErrorCode> {
        let fd = &*self.as_std_view()?;
        set_keep_alive_count(fd, value)?;
        Ok(())
    }

    pub fn hop_limit(&self) -> Result<u8, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let n = get_unicast_hop_limit(fd, self.family)?;
        Ok(n)
    }

    pub fn set_hop_limit(&self, value: u8) -> Result<(), ErrorCode> {
        let fd = &*self.as_std_view()?;
        set_unicast_hop_limit(fd, self.family, value)?;
        #[cfg(target_os = "macos")]
        {
            self.hop_limit
                .store(value, core::sync::atomic::Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn receive_buffer_size(&self) -> Result<u64, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let n = receive_buffer_size(fd)?;
        Ok(n)
    }

    pub fn set_receive_buffer_size(&mut self, value: u64) -> Result<(), ErrorCode> {
        let fd = &*self.as_std_view()?;
        let res = set_receive_buffer_size(fd, value);
        #[cfg(target_os = "macos")]
        {
            let value = res?;
            self.receive_buffer_size
                .store(value, core::sync::atomic::Ordering::Relaxed);
        }
        #[cfg(not(target_os = "macos"))]
        {
            res?;
        }
        Ok(())
    }

    pub fn send_buffer_size(&self) -> Result<u64, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let n = send_buffer_size(fd)?;
        Ok(n)
    }

    pub fn set_send_buffer_size(&mut self, value: u64) -> Result<(), ErrorCode> {
        let fd = &*self.as_std_view()?;
        let res = set_send_buffer_size(fd, value);
        #[cfg(target_os = "macos")]
        {
            let value = res?;
            self.send_buffer_size
                .store(value, core::sync::atomic::Ordering::Relaxed);
        }
        #[cfg(not(target_os = "macos"))]
        {
            res?;
        }
        Ok(())
    }
}

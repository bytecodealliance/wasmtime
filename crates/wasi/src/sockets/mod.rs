use core::fmt;
use core::future::Future;
use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use core::ops::Deref;
use rustix::fd::AsFd;
use rustix::io::Errno;
use rustix::net::sockopt;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use tracing::debug;
use wasmtime::component::{HasData, ResourceTable};

pub(crate) mod ip_name_lookup;
mod tcp;
mod udp;
pub use tcp::TcpSocket;
pub(crate) use tcp::{TcpListenStream, TcpReceiveStream, TcpSendStream};
pub use udp::UdpSocket;

/// A helper struct which implements [`HasData`] for the `wasi:sockets` APIs.
///
/// This can be useful when directly calling `add_to_linker` functions directly,
/// such as [`wasmtime_wasi::p2::bindings::sockets::tcp::add_to_linker`] as the
/// `D` type parameter. See [`HasData`] for more information about the type
/// parameter's purpose.
///
/// When using this type you can skip the [`WasiSocketsView`] trait, for
/// example.
///
/// [`wasmtime_wasi::p2::bindings::sockets::tcp::add_to_linker`]: crate::p2::bindings::sockets::tcp::add_to_linker
///
/// # Examples
///
/// ```
/// use wasmtime::component::{Linker, ResourceTable};
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::sockets::*;
///
/// struct MyStoreState {
///     table: ResourceTable,
///     sockets: WasiSocketsCtx,
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let mut linker = Linker::new(&engine);
///
///     wasmtime_wasi::p2::bindings::sockets::tcp::add_to_linker::<MyStoreState, WasiSockets>(
///         &mut linker,
///         |state| WasiSocketsCtxView {
///             ctx: &mut state.sockets,
///             table: &mut state.table,
///         },
///     )?;
///     Ok(())
/// }
/// ```
pub struct WasiSockets;

impl HasData for WasiSockets {
    type Data<'a> = WasiSocketsCtxView<'a>;
}

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

#[derive(Copy, Clone, Default)]
pub(crate) struct AllowedNetworkUses {
    pub(crate) ip_name_lookup: bool,
    pub(crate) udp: bool,
    pub(crate) tcp: bool,
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
    /// Binding TCP socket.
    ///
    /// This is invoked for both explicit calls to `bind` as well as implicit
    /// binds that are about to be performed by the OS as part of
    /// e.g. `connect` & `listen`.
    ///
    /// The address that is passed to the check is the address provided to
    /// `bind` for explicit binds, or the wildcard address for implicit binds.
    TcpBind,

    /// Put a TCP socket in listener mode.
    ///
    /// If the socket was already bound at the time of the call, the actual
    /// local address of the socket is passed to the check. If the socket is
    /// about to be implicitly bound by `listen`, the wildcard address is passed.
    TcpListen,

    /// Accepting a new client TCP socket.
    ///
    /// The address passed to the check is the remote address of the client that
    /// is being accepted. If the check fails, the client socket will be
    /// silently dropped before reaching the guest.
    TcpAccept,

    /// Connecting a TCP socket.
    ///
    /// The address passed to the check is the remote address that the socket is
    /// attempting to connect to.
    TcpConnect,

    /// Binding UDP socket.
    ///
    /// This is invoked for both explicit calls to `bind` as well as implicit
    /// binds that are about to be performed by the OS as part of
    /// e.g. `connect` & `send`.
    ///
    /// The address that is passed to the check is the address provided to
    /// `bind` for explicit binds, or the wildcard address for implicit binds.
    UdpBind,

    /// Sending a datagram on a UDP socket.
    ///
    /// The address passed to the check is the remote address that the socket is
    /// attempting to send to.
    UdpSend,

    /// Receiving a datagram on a UDP socket.
    ///
    /// The address passed to the check is the remote address of the datagram
    /// that is being received. If the check fails, the datagram will be
    /// silently dropped before reaching the guest.
    UdpReceive,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum SocketAddressFamily {
    Ipv4,
    Ipv6,
}

/// A utility type that separates
/// (1) polling a future for completion and
/// (2) obtaining the output of a future
/// into separate operations. This is a common pattern in WASI 0.2.
pub(crate) enum MaybeReady<T> {
    Pending(Pin<Box<dyn Future<Output = T> + Send>>),
    Ready(T),
}
impl<T> MaybeReady<T> {
    pub(crate) fn new(fut: impl Future<Output = T> + Send + 'static) -> Self {
        Self::Pending(Box::pin(fut))
    }

    /// Poll the future and attempt to resolve it immediately. If the future is
    /// not ready yet, it will be moved to a background task.
    pub(crate) fn poll_or_spawn(fut: impl Future<Output = T> + Send + 'static) -> Self
    where
        T: Send + 'static,
    {
        let mut fut = Box::pin(fut);
        match crate::runtime::with_ambient_tokio_runtime(|| fut.as_mut().poll(&mut noop_cx())) {
            Poll::Ready(val) => Self::Ready(val),
            Poll::Pending => Self::new(crate::runtime::spawn(fut)),
        }
    }
    pub(crate) fn unwrap_ready(self) -> T {
        match self {
            Self::Ready(val) => val,
            Self::Pending(_) => panic!("future not ready"),
        }
    }
    pub(crate) fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<&mut T> {
        match self {
            Self::Pending(fut) => match fut.as_mut().poll(cx) {
                Poll::Ready(val) => {
                    *self = Self::Ready(val);
                    Poll::Ready(match self {
                        Self::Ready(val) => val,
                        _ => unreachable!(),
                    })
                }
                Poll::Pending => Poll::Pending,
            },
            Self::Ready(val) => Poll::Ready(val),
        }
    }
    pub(crate) async fn into_future(self) -> T {
        match self {
            Self::Ready(val) => val,
            Self::Pending(fut) => fut.await,
        }
    }
}

pub(crate) fn noop_cx() -> std::task::Context<'static> {
    std::task::Context::from_waker(futures::task::noop_waker_ref())
}

#[derive(Clone, Copy, Debug)]
pub enum ErrorCode {
    AccessDenied,
    NotSupported,
    InvalidArgument,
    OutOfMemory,
    Timeout,
    InvalidState,
    AddressNotBindable,
    AddressInUse,
    RemoteUnreachable,
    ConnectionRefused,
    ConnectionBroken,
    ConnectionReset,
    ConnectionAborted,
    DatagramTooLarge,
    Other,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for ErrorCode {}

impl From<std::io::Error> for ErrorCode {
    fn from(value: std::io::Error) -> Self {
        (&value).into()
    }
}

impl From<&std::io::Error> for ErrorCode {
    fn from(value: &std::io::Error) -> Self {
        // Attempt the more detailed native error code first:
        if let Some(errno) = Errno::from_io_error(value) {
            return errno.into();
        }

        match value.kind() {
            std::io::ErrorKind::AddrInUse => Self::AddressInUse,
            std::io::ErrorKind::AddrNotAvailable => Self::AddressNotBindable,
            std::io::ErrorKind::ConnectionAborted => Self::ConnectionAborted,
            std::io::ErrorKind::ConnectionRefused => Self::ConnectionRefused,
            std::io::ErrorKind::ConnectionReset => Self::ConnectionReset,
            std::io::ErrorKind::InvalidInput => Self::InvalidArgument,
            std::io::ErrorKind::NotConnected => Self::InvalidState,
            std::io::ErrorKind::OutOfMemory => Self::OutOfMemory,
            std::io::ErrorKind::PermissionDenied => Self::AccessDenied,
            std::io::ErrorKind::TimedOut => Self::Timeout,
            std::io::ErrorKind::Unsupported => Self::NotSupported,
            std::io::ErrorKind::HostUnreachable => Self::RemoteUnreachable,
            std::io::ErrorKind::NetworkUnreachable => Self::RemoteUnreachable,
            std::io::ErrorKind::NetworkDown => Self::RemoteUnreachable,
            std::io::ErrorKind::BrokenPipe => Self::ConnectionBroken,
            _ => {
                debug!("unknown I/O error: {value}");
                Self::Other
            }
        }
    }
}

impl From<Errno> for ErrorCode {
    fn from(value: Errno) -> Self {
        (&value).into()
    }
}

impl From<&Errno> for ErrorCode {
    fn from(value: &Errno) -> Self {
        match *value {
            #[cfg(not(windows))]
            Errno::PERM => Self::AccessDenied,
            Errno::ACCESS => Self::AccessDenied,
            Errno::ADDRINUSE => Self::AddressInUse,
            Errno::ADDRNOTAVAIL => Self::AddressNotBindable,
            Errno::TIMEDOUT => Self::Timeout,
            #[cfg(not(windows))]
            Errno::PIPE => Self::ConnectionBroken,
            Errno::CONNREFUSED => Self::ConnectionRefused,
            Errno::CONNRESET => Self::ConnectionReset,
            Errno::CONNABORTED => Self::ConnectionAborted,
            Errno::INVAL => Self::InvalidArgument,
            Errno::HOSTUNREACH => Self::RemoteUnreachable,
            Errno::HOSTDOWN => Self::RemoteUnreachable,
            Errno::NETDOWN => Self::RemoteUnreachable,
            Errno::NETUNREACH => Self::RemoteUnreachable,
            #[cfg(target_os = "linux")]
            Errno::NONET => Self::RemoteUnreachable,
            Errno::ISCONN => Self::InvalidState,
            Errno::NOTCONN => Self::InvalidState,
            Errno::DESTADDRREQ => Self::InvalidState,
            Errno::MSGSIZE => Self::DatagramTooLarge,
            #[cfg(not(windows))]
            Errno::NOMEM => Self::OutOfMemory,
            Errno::NOBUFS => Self::OutOfMemory,
            Errno::OPNOTSUPP => Self::NotSupported,
            Errno::NOPROTOOPT => Self::NotSupported,
            Errno::PFNOSUPPORT => Self::NotSupported,
            Errno::PROTONOSUPPORT => Self::NotSupported,
            Errno::PROTOTYPE => Self::NotSupported,
            Errno::SOCKTNOSUPPORT => Self::NotSupported,
            Errno::AFNOSUPPORT => Self::NotSupported,

            // FYI, EINPROGRESS should have already been handled by connect.
            _ => {
                debug!("unknown I/O error: {value}");
                Self::Other
            }
        }
    }
}

fn is_deprecated_ipv4_compatible(addr: Ipv6Addr) -> bool {
    matches!(addr.segments(), [0, 0, 0, 0, 0, 0, _, _])
        && addr != Ipv6Addr::UNSPECIFIED
        && addr != Ipv6Addr::LOCALHOST
}

pub(crate) fn is_valid_address_family(addr: IpAddr, socket_family: SocketAddressFamily) -> bool {
    match (socket_family, addr) {
        (SocketAddressFamily::Ipv4, IpAddr::V4(..)) => true,
        (SocketAddressFamily::Ipv6, IpAddr::V6(ipv6)) => {
            // Reject IPv4-*compatible* IPv6 addresses. They have been deprecated
            // since 2006, OS handling of them is inconsistent and our own
            // validations don't take them into account either.
            // Note that these are not the same as IPv4-*mapped* IPv6 addresses.
            !is_deprecated_ipv4_compatible(ipv6) && ipv6.to_ipv4_mapped().is_none()
        }
        _ => false,
    }
}

pub(crate) fn is_valid_remote_address(addr: SocketAddr) -> bool {
    !addr.ip().to_canonical().is_unspecified() && addr.port() != 0
}

pub(crate) fn is_valid_unicast_address(addr: IpAddr) -> bool {
    match addr.to_canonical() {
        IpAddr::V4(ipv4) => !ipv4.is_multicast() && !ipv4.is_broadcast(),
        IpAddr::V6(ipv6) => !ipv6.is_multicast(),
    }
}

pub(crate) fn to_ipv4_addr(addr: (u8, u8, u8, u8)) -> Ipv4Addr {
    let (x0, x1, x2, x3) = addr;
    Ipv4Addr::new(x0, x1, x2, x3)
}

pub(crate) fn from_ipv4_addr(addr: Ipv4Addr) -> (u8, u8, u8, u8) {
    let [x0, x1, x2, x3] = addr.octets();
    (x0, x1, x2, x3)
}

pub(crate) fn to_ipv6_addr(addr: (u16, u16, u16, u16, u16, u16, u16, u16)) -> Ipv6Addr {
    let (x0, x1, x2, x3, x4, x5, x6, x7) = addr;
    Ipv6Addr::new(x0, x1, x2, x3, x4, x5, x6, x7)
}

pub(crate) fn from_ipv6_addr(addr: Ipv6Addr) -> (u16, u16, u16, u16, u16, u16, u16, u16) {
    let [x0, x1, x2, x3, x4, x5, x6, x7] = addr.segments();
    (x0, x1, x2, x3, x4, x5, x6, x7)
}

/*
 * Syscalls wrappers with (opinionated) portability fixes.
 */

fn normalize_get_buffer_size(value: usize) -> usize {
    if cfg!(target_os = "linux") {
        // Linux doubles the value passed to setsockopt to allow space for bookkeeping overhead.
        // getsockopt returns this internally doubled value.
        // We'll half the value to at least get it back into the same ballpark that the application requested it in.
        //
        // This normalized behavior is tested for in: test-programs/src/bin/preview2_tcp_sockopts.rs
        value / 2
    } else {
        value
    }
}

fn normalize_set_buffer_size(value: usize) -> usize {
    value.clamp(1, i32::MAX as usize)
}

fn get_ip_ttl(fd: impl AsFd) -> Result<u8, ErrorCode> {
    let v = sockopt::ip_ttl(fd)?;
    let Ok(v) = v.try_into() else {
        return Err(ErrorCode::NotSupported);
    };
    Ok(v)
}

fn get_ipv6_unicast_hops(fd: impl AsFd) -> Result<u8, ErrorCode> {
    let v = sockopt::ipv6_unicast_hops(fd)?;
    Ok(v)
}

pub(crate) fn get_unicast_hop_limit(
    fd: impl AsFd,
    family: SocketAddressFamily,
) -> Result<u8, ErrorCode> {
    match family {
        SocketAddressFamily::Ipv4 => get_ip_ttl(fd),
        SocketAddressFamily::Ipv6 => get_ipv6_unicast_hops(fd),
    }
}

pub(crate) fn set_unicast_hop_limit(
    fd: impl AsFd,
    family: SocketAddressFamily,
    value: u8,
) -> Result<(), ErrorCode> {
    if value == 0 {
        // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
        //
        // A well-behaved IP application should never send out new packets with TTL 0.
        // We validate the value ourselves because OS'es are not consistent in this.
        // On Linux the validation is even inconsistent between their IPv4 and IPv6 implementation.
        return Err(ErrorCode::InvalidArgument);
    }
    match family {
        SocketAddressFamily::Ipv4 => {
            sockopt::set_ip_ttl(fd, value.into())?;
        }
        SocketAddressFamily::Ipv6 => {
            sockopt::set_ipv6_unicast_hops(fd, Some(value))?;
        }
    }
    Ok(())
}

pub(crate) fn get_receive_buffer_size(fd: impl AsFd) -> Result<u64, ErrorCode> {
    let v = sockopt::socket_recv_buffer_size(fd)?;
    Ok(normalize_get_buffer_size(v).try_into().unwrap_or(u64::MAX))
}

pub(crate) fn set_receive_buffer_size(fd: impl AsFd, value: u64) -> Result<usize, ErrorCode> {
    if value == 0 {
        // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
        return Err(ErrorCode::InvalidArgument);
    }
    let value = value.try_into().unwrap_or(usize::MAX);
    let value = normalize_set_buffer_size(value);
    match sockopt::set_socket_recv_buffer_size(fd, value) {
        // Most platforms (Linux, Windows, Fuchsia, Solaris, Illumos, Haiku, ESP-IDF, ..and more?) treat the value
        // passed to SO_SNDBUF/SO_RCVBUF as a performance tuning hint and silently clamp the input if it exceeds
        // their capability.
        // As far as I can see, only the *BSD family views this option as a hard requirement and fails when the
        // value is out of range. We normalize this behavior in favor of the more commonly understood
        // "performance hint" semantics. In other words; even ENOBUFS is "Ok".
        // A future improvement could be to query the corresponding sysctl on *BSD platforms and clamp the input
        // `size` ourselves, to completely close the gap with other platforms.
        //
        // This normalized behavior is tested for in: test-programs/src/bin/preview2_tcp_sockopts.rs
        Err(Errno::NOBUFS) => {}
        Err(err) => return Err(err.into()),
        _ => {}
    };
    Ok(value)
}

pub(crate) fn get_send_buffer_size(fd: impl AsFd) -> Result<u64, ErrorCode> {
    let v = sockopt::socket_send_buffer_size(fd)?;
    Ok(normalize_get_buffer_size(v).try_into().unwrap_or(u64::MAX))
}

pub(crate) fn set_send_buffer_size(fd: impl AsFd, value: u64) -> Result<usize, ErrorCode> {
    if value == 0 {
        // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
        return Err(ErrorCode::InvalidArgument);
    }
    let value = value.try_into().unwrap_or(usize::MAX);
    let value = normalize_set_buffer_size(value);
    match sockopt::set_socket_send_buffer_size(fd, value) {
        // See comment in `set_receive_buffer_size` for why we ignore NOBUFS.
        Err(Errno::NOBUFS) => {}
        Err(err) => return Err(err.into()),
        _ => {}
    };
    Ok(value)
}

pub(crate) fn unspecified_addr(family: SocketAddressFamily) -> SocketAddr {
    let ip = match family {
        SocketAddressFamily::Ipv4 => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        SocketAddressFamily::Ipv6 => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
    };
    SocketAddr::new(ip, 0)
}

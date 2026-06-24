use core::future::Future;
use core::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use std::{net::SocketAddr, task::Poll};
use wasmtime::component::{HasData, ResourceTable};

mod tcp;
mod udp;
pub(crate) mod util;
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
        match fut.as_mut().poll(&mut noop_cx()) {
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
    pub(crate) fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<()> {
        match self {
            Self::Pending(fut) => match fut.as_mut().poll(cx) {
                Poll::Ready(val) => {
                    *self = Self::Ready(val);
                    Poll::Ready(())
                }
                Poll::Pending => Poll::Pending,
            },
            Self::Ready(_) => Poll::Ready(()),
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

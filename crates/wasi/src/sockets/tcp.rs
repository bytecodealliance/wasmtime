use crate::runtime::with_ambient_tokio_runtime;
use crate::sockets::{
    ErrorCode, MaybeReady, SocketAddrCheck, SocketAddrUse, SocketAddressFamily, WasiSocketsCtx,
    get_receive_buffer_size, get_send_buffer_size, get_unicast_hop_limit, is_valid_address_family,
    is_valid_remote_address, is_valid_unicast_address, set_receive_buffer_size,
    set_send_buffer_size, set_unicast_hop_limit, unspecified_addr,
};
use rustix::fd::AsFd;
use rustix::io::Errno;
use rustix::net::sockopt;
use std::fmt::Debug;
use std::future::poll_fn;
use std::mem;
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Poll, ready};
use std::time::Duration;

/// Value taken from rust std library.
const DEFAULT_BACKLOG: u32 = 128;

const NANOS_PER_SEC: u64 = 1_000_000_000;

/// The state of a TCP socket.
///
/// This represents the various states a socket can be in during the
/// activities of listening, accepting, and connecting.
enum TcpState {
    /// The initial state for a newly-created socket.
    ///
    /// The socket may be bound to a local address in this state, but doesn't
    /// have to.
    ///
    /// From here a socket can transition to `Listening` or `Connecting`.
    Default(tokio::net::TcpSocket),

    /// The socket is now listening and waiting for an incoming connection.
    ///
    /// Sockets will not leave this state.
    Listening(Arc<tokio::net::TcpListener>),

    /// An outgoing connection is started.
    ///
    /// This is created via the `start_connect` method. The payload is a future
    /// for the eventual result of the connect.
    ///
    /// From here a socket can transition to `Connected` or `Closed`.
    Connecting(MaybeReady<Result<tokio::net::TcpStream, ErrorCode>>),

    /// A connection has been established.
    ///
    /// This is created either via `finish_connect` or for freshly accepted
    /// sockets from a TCP listener.
    ///
    /// A socket will not transition out of this state.
    Connected {
        stream: Arc<tokio::net::TcpStream>,
        receive_taken: bool,
        send_taken: bool,
    },

    /// The socket is closed and no more operations can be performed.
    Closed(ErrorCode),
}
impl TcpState {
    fn connected(stream: tokio::net::TcpStream) -> Self {
        TcpState::Connected {
            stream: Arc::new(stream),
            receive_taken: false,
            send_taken: false,
        }
    }
    fn take(&mut self) -> Self {
        mem::replace(self, TcpState::Closed(ErrorCode::Other))
    }
}
impl Debug for TcpState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default(_) => f.debug_tuple("Default").finish(),
            Self::Listening { .. } => f.debug_tuple("Listening").finish(),
            Self::Connecting(..) => f.debug_tuple("Connecting").finish(),
            Self::Connected { .. } => f.debug_tuple("Connected").finish(),
            Self::Closed(..) => write!(f, "Closed"),
        }
    }
}

/// A host TCP socket, plus associated bookkeeping.
pub struct TcpSocket {
    /// The current state in the bind/listen/accept/connect progression.
    tcp_state: TcpState,

    /// The desired listen queue size.
    listen_backlog_size: u32,

    family: SocketAddressFamily,

    /// The checks to perform before doing any noteworthy syscall.
    permissions: SocketAddrCheck,

    /// Persisted socket options to manually apply to newly accepted client
    /// sockets on platforms that don't inherit socket options from the listener.
    listener_options: NonInheritedOptions,

    /// Cached value of whether the socket is bound. Various methods use the
    /// `.is_bound()` method, so we cache it to avoid redundant syscalls.
    is_bound: bool,
}

impl TcpSocket {
    /// Create a new socket in the given family.
    pub(crate) fn new(
        ctx: &WasiSocketsCtx,
        family: SocketAddressFamily,
    ) -> Result<Self, ErrorCode> {
        ctx.allowed_network_uses.check_allowed_tcp()?;

        let socket = with_ambient_tokio_runtime(|| socket(family))?;

        Ok(Self {
            tcp_state: TcpState::Default(socket),
            listen_backlog_size: DEFAULT_BACKLOG,
            family,
            is_bound: false,
            listener_options: Default::default(),
            permissions: ctx.socket_addr_check.clone(),
        })
    }

    fn as_fd(&self) -> Result<rustix::fd::BorrowedFd<'_>, ErrorCode> {
        match &self.tcp_state {
            TcpState::Default(socket) => Ok(socket.as_fd()),
            TcpState::Connected { stream, .. } => Ok(stream.as_fd()),
            TcpState::Listening(listener) => Ok(listener.as_fd()),
            TcpState::Connecting(..) => Err(ErrorCode::InvalidState),
            TcpState::Closed(err) => Err(*err),
        }
    }

    pub(crate) fn is_bound(&mut self) -> bool {
        // Once bound, a TCP socket can never become unbound again. So we can
        // skip all work after a previous call has already determined the
        // socket to be bound.
        if !self.is_bound {
            self.is_bound = match &self.tcp_state {
                TcpState::Default(socket) => socket
                    .local_addr()
                    .is_ok_and(|addr| addr != unspecified_addr(self.family)),
                _ => true,
            };
        }
        self.is_bound
    }

    pub(crate) async fn bind(&mut self, addr: SocketAddr) -> Result<(), ErrorCode> {
        if self.is_bound() {
            return Err(ErrorCode::InvalidState);
        }
        let TcpState::Default(sock) = &self.tcp_state else {
            return Err(ErrorCode::InvalidState);
        };

        if !is_valid_unicast_address(addr.ip()) || !is_valid_address_family(addr.ip(), self.family)
        {
            return Err(ErrorCode::InvalidArgument);
        }

        self.permissions.check(addr, SocketAddrUse::TcpBind).await?;
        bind(sock, addr)?;
        Ok(())
    }

    pub(crate) fn start_connect(&mut self, addr: SocketAddr) -> Result<(), ErrorCode> {
        let TcpState::Default(_) = &self.tcp_state else {
            return Err(ErrorCode::InvalidState);
        };

        let permissions = self.permissions.clone();
        let family = self.family;
        let already_bound = self.is_bound();

        if !is_valid_unicast_address(addr.ip())
            || !is_valid_remote_address(addr)
            || !is_valid_address_family(addr.ip(), family)
        {
            return Err(ErrorCode::InvalidArgument);
        };

        let TcpState::Default(sock) = self.tcp_state.take() else {
            unreachable!();
        };

        self.tcp_state = TcpState::Connecting(MaybeReady::new(async move {
            // Perform all checks before doing any syscalls.
            {
                if !already_bound {
                    // If not explicitly bound, the OS will implicitly bind the
                    // socket to an ephemeral port when connecting. Unlike other
                    // operations (e.g. `listen`), we will *not* do the implicit
                    // bind ourselves because that may accelerate port exhaustion.
                    // For more info, see IP_BIND_ADDRESS_NO_PORT (Linux) or
                    // SO_REUSE_UNICASTPORT (Windows).
                    //
                    // Instead we check the permission to bind, but not perform
                    // the actual bind:
                    let implicit = unspecified_addr(family);
                    permissions.check(implicit, SocketAddrUse::TcpBind).await?;
                }

                permissions.check(addr, SocketAddrUse::TcpConnect).await?;
            }

            let stream = sock.connect(addr).await?;
            Ok(stream)
        }));

        Ok(())
    }

    pub(crate) fn poll_finish_connect(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), ErrorCode>> {
        match &mut self.tcp_state {
            TcpState::Connecting(connect) => {
                ready!(with_ambient_tokio_runtime(|| connect.poll_ready(cx)));
            }
            TcpState::Connected { .. } => return Poll::Ready(Ok(())),
            TcpState::Closed(e) => return Poll::Ready(Err(*e)),
            _ => return Poll::Ready(Err(ErrorCode::InvalidState)),
        }
        let TcpState::Connecting(connect) = self.tcp_state.take() else {
            unreachable!();
        };

        match connect.unwrap_ready() {
            Ok(stream) => {
                self.tcp_state = TcpState::connected(stream);
                Poll::Ready(Ok(()))
            }
            Err(err) => {
                self.tcp_state = TcpState::Closed(err);
                Poll::Ready(Err(err))
            }
        }
    }

    pub(crate) async fn listen(&mut self) -> Result<TcpListenStream, ErrorCode> {
        let already_bound = self.is_bound();
        let sock = match self.tcp_state.take() {
            TcpState::Default(sock) => sock,
            tcp_state => {
                self.tcp_state = tcp_state;
                return Err(ErrorCode::InvalidState);
            }
        };

        // Perform all checks before doing any syscalls.
        {
            if already_bound {
                self.permissions
                    .check(sock.local_addr()?, SocketAddrUse::TcpListen)
                    .await?;
            } else {
                let implicit = unspecified_addr(self.family);
                self.permissions
                    .check(implicit, SocketAddrUse::TcpBind)
                    .await?;
                self.permissions
                    .check(implicit, SocketAddrUse::TcpListen)
                    .await?;
            }
        }

        // Some platforms automatically perform an implicit bind as part of
        // the `listen` syscall. However this is not ubiquitous behavior:
        // - Linux mentions it in their docs [0] that they perform an
        //   implicit bind. This behavior has been experimentally verified.
        // - Windows requires a `bind` before `listen`. This is both
        //   documented [1] and experimentally verified.
        // - Other platforms (e.g. macOS, FreeBSD) do not explicitly
        //   document it either way and instead leave it up to the
        //   individual protocol to decide [2]. However, experiments
        //   show that MacOS in fact _does_ perform an implicit bind.
        //
        // Thus to ensure consistent behavior across all platforms, we
        // perform the implicit bind ourselves here for unbound sockets.
        //
        // [0]: https://man7.org/linux/man-pages/man7/ip.7.html
        // > An ephemeral port is allocated to a socket in the following
        // > circumstances: (...) listen(2) is called on a stream socket
        // > that was not previously bound;
        //
        // [1]: https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-listen
        // > WSAEINVAL: The socket has not been bound with bind.
        //
        // [2]: https://pubs.opengroup.org/onlinepubs/9699919799/functions/listen.html
        // > EDESTADDRREQ: The socket is not bound to a local address,
        // > and the protocol does not support listening on an unbound
        // > socket.
        if !already_bound {
            let implicit = unspecified_addr(self.family);
            bind(&sock, implicit)?;
        }

        let listener = sock.listen(self.listen_backlog_size).map_err(|err| {
            match Errno::from_io_error(&err) {
                // See: https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-listen#:~:text=WSAEMFILE
                // According to the docs, `listen` can return EMFILE on Windows.
                // This is odd, because we're not trying to create a new socket
                // or file descriptor of any kind. So we rewrite it to less
                // surprising error code.
                //
                // At the time of writing, this behavior has never been experimentally
                // observed by any of the wasmtime authors, so we're relying fully
                // on Microsoft's documentation here.
                #[cfg(windows)]
                Some(Errno::MFILE) => Errno::NOBUFS.into(),

                _ => err,
            }
        })?;
        let listener = Arc::new(listener);
        self.tcp_state = TcpState::Listening(listener.clone());

        Ok(TcpListenStream {
            inner: listener,
            listener_options: self.listener_options.clone(),
            family: self.family,
            permissions: self.permissions.clone(),
            pending_accept: None,
        })
    }

    pub(crate) fn take_send_stream(&mut self) -> Result<TcpSendStream, ErrorCode> {
        match &mut self.tcp_state {
            TcpState::Connected {
                stream, send_taken, ..
            } if !*send_taken => {
                *send_taken = true;
                Ok(TcpSendStream {
                    inner: stream.clone(),
                })
            }
            TcpState::Closed(err) => Err(*err),
            _ => Err(ErrorCode::InvalidState),
        }
    }

    pub(crate) fn take_receive_stream(&mut self) -> Result<TcpReceiveStream, ErrorCode> {
        match &mut self.tcp_state {
            TcpState::Connected {
                stream,
                receive_taken,
                ..
            } if !*receive_taken => {
                *receive_taken = true;
                Ok(TcpReceiveStream {
                    inner: stream.clone(),
                })
            }
            TcpState::Closed(err) => Err(*err),
            _ => Err(ErrorCode::InvalidState),
        }
    }

    pub(crate) fn local_address(&mut self) -> Result<SocketAddr, ErrorCode> {
        if !self.is_bound() {
            return Err(ErrorCode::InvalidState);
        }

        match &self.tcp_state {
            TcpState::Default(socket) => Ok(socket.local_addr()?),
            TcpState::Connecting(_) => Err(ErrorCode::InvalidState),
            TcpState::Connected { stream, .. } => Ok(stream.local_addr()?),
            TcpState::Listening(listener) => Ok(listener.local_addr()?),
            TcpState::Closed(err) => Err(*err),
        }
    }

    pub(crate) fn remote_address(&self) -> Result<SocketAddr, ErrorCode> {
        match &self.tcp_state {
            TcpState::Connected { stream, .. } => Ok(stream.peer_addr()?),
            TcpState::Closed(err) => Err(*err),
            _ => Err(ErrorCode::InvalidState),
        }
    }

    pub(crate) fn is_listening(&self) -> bool {
        matches!(self.tcp_state, TcpState::Listening(_))
    }

    pub(crate) fn address_family(&self) -> SocketAddressFamily {
        self.family
    }

    pub(crate) fn set_listen_backlog_size(&mut self, value: u64) -> Result<(), ErrorCode> {
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
            TcpState::Default(..) => {
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
            TcpState::Closed(err) => Err(*err),
            _ => Err(ErrorCode::InvalidState),
        }
    }

    pub(crate) fn keep_alive_enabled(&self) -> Result<bool, ErrorCode> {
        let fd = self.as_fd()?;
        let v = sockopt::socket_keepalive(fd)?;
        Ok(v)
    }

    pub(crate) fn set_keep_alive_enabled(&self, value: bool) -> Result<(), ErrorCode> {
        let fd = self.as_fd()?;
        sockopt::set_socket_keepalive(fd, value)?;
        Ok(())
    }

    pub(crate) fn keep_alive_idle_time(&self) -> Result<u64, ErrorCode> {
        let fd = self.as_fd()?;
        let v = sockopt::tcp_keepidle(fd)?;
        Ok(v.as_nanos().try_into().unwrap_or(u64::MAX))
    }

    pub(crate) fn set_keep_alive_idle_time(&mut self, value: u64) -> Result<(), ErrorCode> {
        if value == 0 {
            // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
            return Err(ErrorCode::InvalidArgument);
        }
        let fd = self.as_fd()?;
        let value = clamp_keep_alive_time(value);
        sockopt::set_tcp_keepidle(fd, Duration::from_nanos(value))?;
        self.listener_options.set_keep_alive_idle_time(value);
        Ok(())
    }

    pub(crate) fn keep_alive_interval(&self) -> Result<u64, ErrorCode> {
        let fd = self.as_fd()?;
        let v = sockopt::tcp_keepintvl(fd)?;
        Ok(v.as_nanos().try_into().unwrap_or(u64::MAX))
    }

    pub(crate) fn set_keep_alive_interval(&self, value: u64) -> Result<(), ErrorCode> {
        if value == 0 {
            // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
            return Err(ErrorCode::InvalidArgument);
        }
        let fd = self.as_fd()?;
        let value = clamp_keep_alive_time(value);
        sockopt::set_tcp_keepintvl(fd, Duration::from_nanos(value))?;
        Ok(())
    }

    pub(crate) fn keep_alive_count(&self) -> Result<u32, ErrorCode> {
        let fd = self.as_fd()?;
        let v = sockopt::tcp_keepcnt(fd)?;
        Ok(v)
    }

    pub(crate) fn set_keep_alive_count(&self, value: u32) -> Result<(), ErrorCode> {
        if value == 0 {
            // WIT: "If the provided value is 0, an `invalid-argument` error is returned."
            return Err(ErrorCode::InvalidArgument);
        }
        let value = clamp_keep_alive_count(value);
        let fd = self.as_fd()?;
        sockopt::set_tcp_keepcnt(fd, value)?;
        Ok(())
    }

    pub(crate) fn hop_limit(&self) -> Result<u8, ErrorCode> {
        let fd = self.as_fd()?;
        let n = get_unicast_hop_limit(fd, self.family)?;
        Ok(n)
    }

    pub(crate) fn set_hop_limit(&mut self, value: u8) -> Result<(), ErrorCode> {
        {
            let fd = self.as_fd()?;
            set_unicast_hop_limit(fd, self.family, value)?;
        }
        self.listener_options.set_hop_limit(value);
        Ok(())
    }

    pub(crate) fn receive_buffer_size(&self) -> Result<u64, ErrorCode> {
        let fd = self.as_fd()?;
        let n = get_receive_buffer_size(fd)?;
        Ok(n)
    }

    pub(crate) fn set_receive_buffer_size(&mut self, value: u64) -> Result<(), ErrorCode> {
        let res = {
            let fd = self.as_fd()?;
            set_receive_buffer_size(fd, value)?
        };
        self.listener_options.set_receive_buffer_size(res);
        Ok(())
    }

    pub(crate) fn send_buffer_size(&self) -> Result<u64, ErrorCode> {
        let fd = self.as_fd()?;
        let n = get_send_buffer_size(fd)?;
        Ok(n)
    }

    pub(crate) fn set_send_buffer_size(&mut self, value: u64) -> Result<(), ErrorCode> {
        let res = {
            let fd = self.as_fd()?;
            set_send_buffer_size(fd, value)?
        };
        self.listener_options.set_send_buffer_size(res);
        Ok(())
    }
}

pub(crate) struct TcpListenStream {
    inner: Arc<tokio::net::TcpListener>,
    family: SocketAddressFamily,
    listener_options: NonInheritedOptions,
    permissions: SocketAddrCheck,
    pending_accept: Option<MaybeReady<Result<tokio::net::TcpStream, ErrorCode>>>,
}
impl TcpListenStream {
    pub(crate) fn poll_accept(&mut self, cx: &mut std::task::Context<'_>) -> Poll<TcpSocket> {
        ready!(self.poll_ready(cx));
        let result = self.pending_accept.take().unwrap().unwrap_ready();
        Poll::Ready(TcpSocket {
            tcp_state: match result {
                Ok(client) => {
                    self.listener_options.apply(self.family, &client);
                    TcpState::connected(client)
                }
                Err(err) => TcpState::Closed(err),
            },
            listen_backlog_size: DEFAULT_BACKLOG,
            family: self.family,
            is_bound: true,
            listener_options: Default::default(),
            permissions: self.permissions.clone(),
        })
    }

    pub(crate) fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<()> {
        if self.pending_accept.is_none() {
            let listener = self.inner.clone();
            let permissions = self.permissions.clone();

            self.pending_accept = Some(MaybeReady::new(async move {
                loop {
                    match accept(&listener).await {
                        Ok((client, addr)) => {
                            if permissions
                                .check(addr, SocketAddrUse::TcpAccept)
                                .await
                                .is_ok()
                            {
                                return Ok(client);
                            } else {
                                reset(client);
                                continue;
                            }
                        }
                        Err(err) => {
                            return Err(err.into());
                        }
                    }
                }
            }));
        }

        with_ambient_tokio_runtime(|| {
            self.pending_accept
                .as_mut()
                .unwrap()
                .poll_ready(cx)
                .map(|_| ())
        })
    }
}

pub(crate) struct TcpSendStream {
    inner: Arc<tokio::net::TcpStream>,
}
impl TcpSendStream {
    pub(crate) fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<()> {
        self.inner.poll_write_ready(cx).map(|_| ())
    }

    pub(crate) fn poll_write(
        &mut self,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, ErrorCode>> {
        loop {
            return match self.inner.try_write(buf) {
                Ok(n) => Poll::Ready(Ok(n)),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    match self.inner.poll_write_ready(cx) {
                        Poll::Ready(Ok(())) => continue,
                        Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
                        Poll::Pending => Poll::Pending,
                    }
                }
                Err(e) => Poll::Ready(Err(match Errno::from_io_error(&e) {
                    #[cfg(windows)]
                    Some(Errno::SHUTDOWN) | Some(Errno::CONNABORTED) => ErrorCode::ConnectionBroken,
                    #[cfg(not(windows))]
                    Some(Errno::PIPE) => ErrorCode::ConnectionBroken,

                    _ => e.into(),
                })),
            };
        }
    }
    pub(crate) async fn write(&mut self, buf: &[u8]) -> Result<usize, ErrorCode> {
        poll_fn(|cx| self.poll_write(cx, buf)).await
    }
}
impl Drop for TcpSendStream {
    fn drop(&mut self) {
        _ = rustix::net::shutdown(&self.inner, rustix::net::Shutdown::Write);
    }
}

pub(crate) struct TcpReceiveStream {
    inner: Arc<tokio::net::TcpStream>,
}
impl TcpReceiveStream {
    pub(crate) fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<()> {
        self.inner.poll_read_ready(cx).map(|_| ())
    }

    pub(crate) fn poll_read(
        &mut self,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, ErrorCode>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        loop {
            return match self.inner.try_read(buf) {
                Ok(0) => Poll::Ready(Ok(0)),
                Ok(n) => Poll::Ready(Ok(n)),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    match self.inner.poll_read_ready(cx) {
                        Poll::Ready(Ok(())) => continue,
                        Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
                        Poll::Pending => Poll::Pending,
                    }
                }
                Err(e) => Poll::Ready(Err(e.into())),
            };
        }
    }
}
impl Drop for TcpReceiveStream {
    fn drop(&mut self) {
        _ = rustix::net::shutdown(&self.inner, rustix::net::Shutdown::Read);
    }
}

#[cfg(not(target_os = "macos"))]
pub use inherits_option::*;
#[cfg(not(target_os = "macos"))]
mod inherits_option {
    use crate::sockets::SocketAddressFamily;
    use tokio::net::TcpStream;

    #[derive(Default, Clone)]
    pub struct NonInheritedOptions;

    impl NonInheritedOptions {
        pub fn set_keep_alive_idle_time(&mut self, _value: u64) {}

        pub fn set_hop_limit(&mut self, _value: u8) {}

        pub fn set_receive_buffer_size(&mut self, _value: usize) {}

        pub fn set_send_buffer_size(&mut self, _value: usize) {}

        pub(crate) fn apply(&self, _family: SocketAddressFamily, _stream: &TcpStream) {}
    }
}

#[cfg(target_os = "macos")]
pub use does_not_inherit_options::*;
#[cfg(target_os = "macos")]
mod does_not_inherit_options {
    use crate::sockets::SocketAddressFamily;
    use rustix::net::sockopt;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering::Relaxed};
    use std::time::Duration;
    use tokio::net::TcpStream;

    // The socket options below are not automatically inherited from the listener
    // on all platforms. So we keep track of which options have been explicitly
    // set and manually apply those values to newly accepted clients.
    #[derive(Default, Clone)]
    pub struct NonInheritedOptions(Arc<Inner>);

    #[derive(Default)]
    struct Inner {
        receive_buffer_size: AtomicUsize,
        send_buffer_size: AtomicUsize,
        hop_limit: AtomicU8,
        keep_alive_idle_time: AtomicU64, // nanoseconds
    }

    impl NonInheritedOptions {
        pub fn set_keep_alive_idle_time(&mut self, value: u64) {
            self.0.keep_alive_idle_time.store(value, Relaxed);
        }

        pub fn set_hop_limit(&mut self, value: u8) {
            self.0.hop_limit.store(value, Relaxed);
        }

        pub fn set_receive_buffer_size(&mut self, value: usize) {
            self.0.receive_buffer_size.store(value, Relaxed);
        }

        pub fn set_send_buffer_size(&mut self, value: usize) {
            self.0.send_buffer_size.store(value, Relaxed);
        }

        pub(crate) fn apply(&self, family: SocketAddressFamily, stream: &TcpStream) {
            // Manually inherit socket options from listener. We only have to
            // do this on platforms that don't already do this automatically
            // and only if a specific value was explicitly set on the listener.

            let receive_buffer_size = self.0.receive_buffer_size.load(Relaxed);
            if receive_buffer_size > 0 {
                // Ignore potential error.
                _ = sockopt::set_socket_recv_buffer_size(&stream, receive_buffer_size);
            }

            let send_buffer_size = self.0.send_buffer_size.load(Relaxed);
            if send_buffer_size > 0 {
                // Ignore potential error.
                _ = sockopt::set_socket_send_buffer_size(&stream, send_buffer_size);
            }

            // For some reason, IP_TTL is inherited, but IPV6_UNICAST_HOPS isn't.
            if family == SocketAddressFamily::Ipv6 {
                let hop_limit = self.0.hop_limit.load(Relaxed);
                if hop_limit > 0 {
                    // Ignore potential error.
                    _ = sockopt::set_ipv6_unicast_hops(&stream, Some(hop_limit));
                }
            }

            let keep_alive_idle_time = self.0.keep_alive_idle_time.load(Relaxed);
            if keep_alive_idle_time > 0 {
                // Ignore potential error.
                _ = sockopt::set_tcp_keepidle(&stream, Duration::from_nanos(keep_alive_idle_time));
            }
        }
    }
}

fn socket(family: SocketAddressFamily) -> std::io::Result<tokio::net::TcpSocket> {
    match family {
        SocketAddressFamily::Ipv4 => tokio::net::TcpSocket::new_v4(),
        SocketAddressFamily::Ipv6 => {
            let socket = tokio::net::TcpSocket::new_v6()?;

            // From the WASI spec:
            // > On IPv6 sockets, IPV6_V6ONLY is enabled by default and can't
            // > be configured otherwise.
            sockopt::set_ipv6_v6only(&socket, true)?;
            Ok(socket)
        }
    }
}

fn bind(socket: &tokio::net::TcpSocket, local_address: SocketAddr) -> Result<(), ErrorCode> {
    // From the WASI spec:
    // > The bind operation shouldn't be affected by the TIME_WAIT state of a
    // > recently closed socket on the same local address. In practice this
    // > means that the SO_REUSEADDR socket option should be set implicitly on
    // > all platforms, except on Windows where this is the default behavior
    // > and SO_REUSEADDR performs something different.
    #[cfg(not(windows))]
    {
        _ = sockopt::set_socket_reuseaddr(&socket, true);
    }

    // Perform the OS bind call.
    socket
        .bind(local_address)
        .map_err(|err| match Errno::from_io_error(&err) {
            // From https://pubs.opengroup.org/onlinepubs/9699919799/functions/bind.html:
            // > [EAFNOSUPPORT] The specified address is not a valid address for the address family of the specified socket
            //
            // The most common reasons for this error should have already
            // been handled by our own validation.. This error mapping is here
            // just in case there is an edge case we didn't catch.
            Some(Errno::AFNOSUPPORT) => ErrorCode::InvalidArgument,
            // See: https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-bind#:~:text=WSAENOBUFS
            // Windows returns WSAENOBUFS when the ephemeral ports have been exhausted.
            #[cfg(windows)]
            Some(Errno::NOBUFS) => ErrorCode::AddressInUse,
            _ => err.into(),
        })
}

async fn accept(
    listener: &tokio::net::TcpListener,
) -> std::io::Result<(tokio::net::TcpStream, SocketAddr)> {
    listener
        .accept()
        .await
        .map_err(|err| match Errno::from_io_error(&err) {
            // From: https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-accept#:~:text=WSAEINPROGRESS
            // > WSAEINPROGRESS: A blocking Windows Sockets 1.1 call is in progress,
            // > or the service provider is still processing a callback function.
            //
            // wasi-sockets doesn't have an equivalent to the EINPROGRESS error,
            // because in POSIX this error is only returned by a non-blocking
            // `connect` and wasi-sockets has a different solution for that.
            #[cfg(windows)]
            Some(Errno::INPROGRESS) => Errno::INTR.into(),

            // Normalize Linux' non-standard behavior.
            //
            // From https://man7.org/linux/man-pages/man2/accept.2.html:
            // > Linux accept() passes already-pending network errors on the
            // > new socket as an error code from accept(). This behavior
            // > differs from other BSD socket implementations. (...)
            #[cfg(target_os = "linux")]
            Some(
                Errno::CONNRESET
                | Errno::NETRESET
                | Errno::HOSTUNREACH
                | Errno::HOSTDOWN
                | Errno::NETDOWN
                | Errno::NETUNREACH
                | Errno::PROTO
                | Errno::NOPROTOOPT
                | Errno::NONET
                | Errno::OPNOTSUPP,
            ) => Errno::CONNABORTED.into(),

            _ => err,
        })
}

fn reset(socket: tokio::net::TcpStream) {
    _ = socket.set_zero_linger();
    drop(socket);
}

fn clamp_keep_alive_time(value: u64) -> u64 {
    // Ensure that the value passed to the actual syscall never gets rounded down to 0.
    const MIN: u64 = 1 * NANOS_PER_SEC;

    // Cap it at Linux' maximum, which appears to have the lowest limit across our supported platforms.
    const MAX: u64 = (i16::MAX as u64) * NANOS_PER_SEC;

    value.clamp(MIN, MAX)
}

fn clamp_keep_alive_count(value: u32) -> u32 {
    const MIN_CNT: u32 = 1;
    // Cap it at Linux' maximum, which appears to have the lowest limit across our supported platforms.
    const MAX_CNT: u32 = i8::MAX as u32;

    value.clamp(MIN_CNT, MAX_CNT)
}

use crate::p2::P2TcpStreamingState;
use crate::runtime::with_ambient_tokio_runtime;
use crate::sockets::util::{
    ErrorCode, get_unicast_hop_limit, is_valid_address_family, is_valid_remote_address,
    is_valid_unicast_address, receive_buffer_size, send_buffer_size, set_keep_alive_count,
    set_keep_alive_idle_time, set_keep_alive_interval, set_receive_buffer_size,
    set_send_buffer_size, set_unicast_hop_limit, tcp_bind,
};
use crate::sockets::{DEFAULT_TCP_BACKLOG, SocketAddressFamily, WasiSocketsCtx};
use io_lifetimes::AsSocketlike as _;
use io_lifetimes::views::SocketlikeView;
use rustix::io::Errno;
use rustix::net::sockopt;
use std::fmt::Debug;
use std::io;
use std::mem;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// The state of a TCP socket.
///
/// This represents the various states a socket can be in during the
/// activities of binding, listening, accepting, and connecting. Note that this
/// state machine encompasses both WASIp2 and WASIp3.
enum TcpState {
    /// The initial state for a newly-created socket.
    ///
    /// From here a socket can transition to `BindStarted`, `ListenStarted`, or
    /// `Connecting`.
    Default(tokio::net::TcpSocket),

    /// A state indicating that a bind has been started and must be finished
    /// subsequently with `finish_bind`.
    ///
    /// From here a socket can transition to `Bound`.
    BindStarted(tokio::net::TcpSocket),

    /// Binding finished. The socket has an address but is not yet listening for
    /// connections.
    ///
    /// From here a socket can transition to `ListenStarted`, or `Connecting`.
    Bound(tokio::net::TcpSocket),

    /// Listening on a socket has started and must be completed with
    /// `finish_listen`.
    ///
    /// From here a socket can transition to `Listening`.
    ListenStarted(tokio::net::TcpSocket),

    /// The socket is now listening and waiting for an incoming connection.
    ///
    /// Sockets will not leave this state.
    Listening {
        /// The raw tokio-basd TCP listener managing the underyling socket.
        listener: Arc<tokio::net::TcpListener>,

        /// The last-accepted connection, set during the `ready` method and read
        /// during the `accept` method. Note that this is only used for WASIp2
        /// at this time.
        pending_accept: Option<io::Result<tokio::net::TcpStream>>,
    },

    /// An outgoing connection is started.
    ///
    /// This is created via the `start_connect` method. The payload here is an
    /// optionally-specified owned future for the result of the connect. In
    /// WASIp2 the future lives here, but in WASIp3 it lives on the event loop
    /// so this is `None`.
    ///
    /// From here a socket can transition to `ConnectReady` or `Connected`.
    Connecting(Option<Pin<Box<dyn Future<Output = io::Result<tokio::net::TcpStream>> + Send>>>),

    /// A connection via `Connecting` has completed.
    ///
    /// This is present for WASIp2 where the `Connecting` state stores `Some` of
    /// a future, and the result of that future is recorded here when it
    /// finishes as part of the `ready` method.
    ///
    /// From here a socket can transition to `Connected`.
    ConnectReady(io::Result<tokio::net::TcpStream>),

    /// A connection has been established.
    ///
    /// This is created either via `finish_connect` or for freshly accepted
    /// sockets from a TCP listener.
    ///
    /// From here a socket can transition to `Receiving` or `P2Streaming`.
    Connected(Arc<tokio::net::TcpStream>),

    /// A connection has been established and `receive` has been called.
    ///
    /// A socket will not transition out of this state.
    #[cfg(feature = "p3")]
    Receiving(Arc<tokio::net::TcpStream>),

    /// This is a WASIp2-bound socket which stores some extra state for
    /// read/write streams to handle TCP shutdown.
    ///
    /// A socket will not transition out of this state.
    P2Streaming(Box<P2TcpStreamingState>),

    /// This is not actually a socket but a deferred error.
    ///
    /// This error came out of `accept` and is deferred until the socket is
    /// operated on.
    #[cfg(feature = "p3")]
    Error(io::Error),

    /// The socket is closed and no more operations can be performed.
    Closed,
}

impl Debug for TcpState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default(_) => f.debug_tuple("Default").finish(),
            Self::BindStarted(_) => f.debug_tuple("BindStarted").finish(),
            Self::Bound(_) => f.debug_tuple("Bound").finish(),
            Self::ListenStarted { .. } => f.debug_tuple("ListenStarted").finish(),
            Self::Listening { .. } => f.debug_tuple("Listening").finish(),
            Self::Connecting(..) => f.debug_tuple("Connecting").finish(),
            Self::ConnectReady(..) => f.debug_tuple("ConnectReady").finish(),
            Self::Connected { .. } => f.debug_tuple("Connected").finish(),
            #[cfg(feature = "p3")]
            Self::Receiving { .. } => f.debug_tuple("Receiving").finish(),
            Self::P2Streaming(_) => f.debug_tuple("P2Streaming").finish(),
            #[cfg(feature = "p3")]
            Self::Error(..) => f.debug_tuple("Error").finish(),
            Self::Closed => write!(f, "Closed"),
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

    options: NonInheritedOptions,
}

impl TcpSocket {
    /// Create a new socket in the given family.
    pub(crate) fn new(ctx: &WasiSocketsCtx, family: SocketAddressFamily) -> std::io::Result<Self> {
        ctx.allowed_network_uses.check_allowed_tcp()?;

        with_ambient_tokio_runtime(|| {
            let socket = match family {
                SocketAddressFamily::Ipv4 => tokio::net::TcpSocket::new_v4()?,
                SocketAddressFamily::Ipv6 => {
                    let socket = tokio::net::TcpSocket::new_v6()?;
                    sockopt::set_ipv6_v6only(&socket, true)?;
                    socket
                }
            };

            Ok(Self::from_state(TcpState::Default(socket), family))
        })
    }

    #[cfg(feature = "p3")]
    pub(crate) fn new_error(err: io::Error, family: SocketAddressFamily) -> Self {
        TcpSocket::from_state(TcpState::Error(err), family)
    }

    /// Creates a new socket with the `result` of an accepted socket from a
    /// `TcpListener`.
    ///
    /// This will handle the `result` internally and `result` should be the raw
    /// result from a TCP listen operation.
    pub(crate) fn new_accept(
        result: io::Result<tokio::net::TcpStream>,
        options: &NonInheritedOptions,
        family: SocketAddressFamily,
    ) -> io::Result<Self> {
        let client = result.map_err(|err| match Errno::from_io_error(&err) {
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
        })?;
        options.apply(family, &client);
        Ok(Self::from_state(
            TcpState::Connected(Arc::new(client)),
            family,
        ))
    }

    /// Create a `TcpSocket` from an existing socket.
    fn from_state(state: TcpState, family: SocketAddressFamily) -> Self {
        Self {
            tcp_state: state,
            listen_backlog_size: DEFAULT_TCP_BACKLOG,
            family,
            options: Default::default(),
        }
    }

    pub(crate) fn as_std_view(&self) -> Result<SocketlikeView<'_, std::net::TcpStream>, ErrorCode> {
        match &self.tcp_state {
            TcpState::Default(socket)
            | TcpState::BindStarted(socket)
            | TcpState::Bound(socket)
            | TcpState::ListenStarted(socket) => Ok(socket.as_socketlike_view()),
            TcpState::Connected(stream) => Ok(stream.as_socketlike_view()),
            #[cfg(feature = "p3")]
            TcpState::Receiving(stream) => Ok(stream.as_socketlike_view()),
            TcpState::Listening { listener, .. } => Ok(listener.as_socketlike_view()),
            TcpState::P2Streaming(state) => Ok(state.stream.as_socketlike_view()),
            TcpState::Connecting(..) | TcpState::ConnectReady(_) | TcpState::Closed => {
                Err(ErrorCode::InvalidState)
            }
            #[cfg(feature = "p3")]
            TcpState::Error(err) => Err(err.into()),
        }
    }

    pub(crate) fn start_bind(&mut self, addr: SocketAddr) -> Result<(), ErrorCode> {
        let ip = addr.ip();
        if !is_valid_unicast_address(ip) || !is_valid_address_family(ip, self.family) {
            return Err(ErrorCode::InvalidArgument);
        }
        match mem::replace(&mut self.tcp_state, TcpState::Closed) {
            TcpState::Default(sock) => {
                if let Err(err) = tcp_bind(&sock, addr) {
                    self.tcp_state = TcpState::Default(sock);
                    Err(err)
                } else {
                    self.tcp_state = TcpState::BindStarted(sock);
                    Ok(())
                }
            }
            tcp_state => {
                self.tcp_state = tcp_state;
                Err(ErrorCode::InvalidState)
            }
        }
    }

    pub(crate) fn finish_bind(&mut self) -> Result<(), ErrorCode> {
        match mem::replace(&mut self.tcp_state, TcpState::Closed) {
            TcpState::BindStarted(socket) => {
                self.tcp_state = TcpState::Bound(socket);
                Ok(())
            }
            current_state => {
                // Reset the state so that the outside world doesn't see this socket as closed
                self.tcp_state = current_state;
                Err(ErrorCode::NotInProgress)
            }
        }
    }

    pub(crate) fn start_connect(
        &mut self,
        addr: &SocketAddr,
    ) -> Result<tokio::net::TcpSocket, ErrorCode> {
        match self.tcp_state {
            TcpState::Default(..) | TcpState::Bound(..) => {}
            TcpState::Connecting(..) => {
                return Err(ErrorCode::ConcurrencyConflict);
            }
            _ => return Err(ErrorCode::InvalidState),
        };

        if !is_valid_unicast_address(addr.ip())
            || !is_valid_remote_address(*addr)
            || !is_valid_address_family(addr.ip(), self.family)
        {
            return Err(ErrorCode::InvalidArgument);
        };

        let (TcpState::Default(tokio_socket) | TcpState::Bound(tokio_socket)) =
            mem::replace(&mut self.tcp_state, TcpState::Connecting(None))
        else {
            unreachable!();
        };

        Ok(tokio_socket)
    }

    /// For WASIp2 this is used to record the actual connection future as part
    /// of `start_connect` within this socket state.
    pub(crate) fn set_pending_connect(
        &mut self,
        future: impl Future<Output = io::Result<tokio::net::TcpStream>> + Send + 'static,
    ) -> Result<(), ErrorCode> {
        match &mut self.tcp_state {
            TcpState::Connecting(slot @ None) => {
                *slot = Some(Box::pin(future));
                Ok(())
            }
            _ => Err(ErrorCode::InvalidState),
        }
    }

    /// For WASIp2 this retreives the result from the future passed to
    /// `set_pending_connect`.
    ///
    /// Return states here are:
    ///
    /// * `Ok(Some(res))` - where `res` is the result of the connect operation.
    /// * `Ok(None)` - the connect operation isn't ready yet.
    /// * `Err(e)` - a connect operation is not in progress.
    pub(crate) fn take_pending_connect(
        &mut self,
    ) -> Result<Option<io::Result<tokio::net::TcpStream>>, ErrorCode> {
        match mem::replace(&mut self.tcp_state, TcpState::Connecting(None)) {
            TcpState::ConnectReady(result) => Ok(Some(result)),
            TcpState::Connecting(Some(mut future)) => {
                let mut cx = Context::from_waker(Waker::noop());
                match with_ambient_tokio_runtime(|| future.as_mut().poll(&mut cx)) {
                    Poll::Ready(result) => Ok(Some(result)),
                    Poll::Pending => {
                        self.tcp_state = TcpState::Connecting(Some(future));
                        Ok(None)
                    }
                }
            }
            current_state => {
                self.tcp_state = current_state;
                Err(ErrorCode::NotInProgress)
            }
        }
    }

    pub(crate) fn finish_connect(
        &mut self,
        result: io::Result<tokio::net::TcpStream>,
    ) -> Result<(), ErrorCode> {
        if !matches!(self.tcp_state, TcpState::Connecting(None)) {
            return Err(ErrorCode::InvalidState);
        }
        match result {
            Ok(stream) => {
                self.tcp_state = TcpState::Connected(Arc::new(stream));
                Ok(())
            }
            Err(err) => {
                self.tcp_state = TcpState::Closed;
                Err(ErrorCode::from(err))
            }
        }
    }

    pub(crate) fn start_listen(&mut self) -> Result<(), ErrorCode> {
        match mem::replace(&mut self.tcp_state, TcpState::Closed) {
            TcpState::Bound(tokio_socket) => {
                self.tcp_state = TcpState::ListenStarted(tokio_socket);
                Ok(())
            }
            previous_state => {
                self.tcp_state = previous_state;
                Err(ErrorCode::InvalidState)
            }
        }
    }

    pub(crate) fn finish_listen(&mut self) -> Result<(), ErrorCode> {
        let tokio_socket = match mem::replace(&mut self.tcp_state, TcpState::Closed) {
            TcpState::ListenStarted(tokio_socket) => tokio_socket,
            previous_state => {
                self.tcp_state = previous_state;
                return Err(ErrorCode::NotInProgress);
            }
        };

        match with_ambient_tokio_runtime(|| tokio_socket.listen(self.listen_backlog_size)) {
            Ok(listener) => {
                self.tcp_state = TcpState::Listening {
                    listener: Arc::new(listener),
                    pending_accept: None,
                };
                Ok(())
            }
            Err(err) => {
                self.tcp_state = TcpState::Closed;

                Err(match Errno::from_io_error(&err) {
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

                    _ => err.into(),
                })
            }
        }
    }

    pub(crate) fn accept(&mut self) -> Result<Option<Self>, ErrorCode> {
        let TcpState::Listening {
            listener,
            pending_accept,
        } = &mut self.tcp_state
        else {
            return Err(ErrorCode::InvalidState);
        };

        let result = match pending_accept.take() {
            Some(result) => result,
            None => {
                let mut cx = std::task::Context::from_waker(Waker::noop());
                match with_ambient_tokio_runtime(|| listener.poll_accept(&mut cx))
                    .map_ok(|(stream, _)| stream)
                {
                    Poll::Ready(result) => result,
                    Poll::Pending => return Ok(None),
                }
            }
        };

        Ok(Some(Self::new_accept(result, &self.options, self.family)?))
    }

    #[cfg(feature = "p3")]
    pub(crate) fn start_receive(&mut self) -> Option<&Arc<tokio::net::TcpStream>> {
        match mem::replace(&mut self.tcp_state, TcpState::Closed) {
            TcpState::Connected(stream) => {
                self.tcp_state = TcpState::Receiving(stream);
                Some(self.tcp_stream_arc().unwrap())
            }
            prev => {
                self.tcp_state = prev;
                None
            }
        }
    }

    pub(crate) fn local_address(&self) -> Result<SocketAddr, ErrorCode> {
        match &self.tcp_state {
            TcpState::Bound(socket) => Ok(socket.local_addr()?),
            TcpState::Connected(stream) => Ok(stream.local_addr()?),
            #[cfg(feature = "p3")]
            TcpState::Receiving(stream) => Ok(stream.local_addr()?),
            TcpState::P2Streaming(state) => Ok(state.stream.local_addr()?),
            TcpState::Listening { listener, .. } => Ok(listener.local_addr()?),
            #[cfg(feature = "p3")]
            TcpState::Error(err) => Err(err.into()),
            _ => Err(ErrorCode::InvalidState),
        }
    }

    pub(crate) fn remote_address(&self) -> Result<SocketAddr, ErrorCode> {
        let stream = self.tcp_stream_arc()?;
        let addr = stream.peer_addr()?;
        Ok(addr)
    }

    pub(crate) fn is_listening(&self) -> bool {
        matches!(self.tcp_state, TcpState::Listening { .. })
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
            TcpState::Default(..) | TcpState::Bound(..) => {
                // Socket not listening yet. Stash value for first invocation to `listen`.
                self.listen_backlog_size = value;
                Ok(())
            }
            TcpState::Listening { listener, .. } => {
                // Try to update the backlog by calling `listen` again.
                // Not all platforms support this. We'll only update our own value if the OS supports changing the backlog size after the fact.
                if rustix::net::listen(&listener, value.try_into().unwrap_or(i32::MAX)).is_err() {
                    return Err(ErrorCode::NotSupported);
                }
                self.listen_backlog_size = value;
                Ok(())
            }
            #[cfg(feature = "p3")]
            TcpState::Error(err) => Err(err.into()),
            _ => Err(ErrorCode::InvalidState),
        }
    }

    pub(crate) fn keep_alive_enabled(&self) -> Result<bool, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let v = sockopt::socket_keepalive(fd)?;
        Ok(v)
    }

    pub(crate) fn set_keep_alive_enabled(&self, value: bool) -> Result<(), ErrorCode> {
        let fd = &*self.as_std_view()?;
        sockopt::set_socket_keepalive(fd, value)?;
        Ok(())
    }

    pub(crate) fn keep_alive_idle_time(&self) -> Result<u64, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let v = sockopt::tcp_keepidle(fd)?;
        Ok(v.as_nanos().try_into().unwrap_or(u64::MAX))
    }

    pub(crate) fn set_keep_alive_idle_time(&mut self, value: u64) -> Result<(), ErrorCode> {
        let value = {
            let fd = self.as_std_view()?;
            set_keep_alive_idle_time(&*fd, value)?
        };
        self.options.set_keep_alive_idle_time(value);
        Ok(())
    }

    pub(crate) fn keep_alive_interval(&self) -> Result<u64, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let v = sockopt::tcp_keepintvl(fd)?;
        Ok(v.as_nanos().try_into().unwrap_or(u64::MAX))
    }

    pub(crate) fn set_keep_alive_interval(&self, value: u64) -> Result<(), ErrorCode> {
        let fd = &*self.as_std_view()?;
        set_keep_alive_interval(fd, Duration::from_nanos(value))?;
        Ok(())
    }

    pub(crate) fn keep_alive_count(&self) -> Result<u32, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let v = sockopt::tcp_keepcnt(fd)?;
        Ok(v)
    }

    pub(crate) fn set_keep_alive_count(&self, value: u32) -> Result<(), ErrorCode> {
        let fd = &*self.as_std_view()?;
        set_keep_alive_count(fd, value)?;
        Ok(())
    }

    pub(crate) fn hop_limit(&self) -> Result<u8, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let n = get_unicast_hop_limit(fd, self.family)?;
        Ok(n)
    }

    pub(crate) fn set_hop_limit(&mut self, value: u8) -> Result<(), ErrorCode> {
        {
            let fd = &*self.as_std_view()?;
            set_unicast_hop_limit(fd, self.family, value)?;
        }
        self.options.set_hop_limit(value);
        Ok(())
    }

    pub(crate) fn receive_buffer_size(&self) -> Result<u64, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let n = receive_buffer_size(fd)?;
        Ok(n)
    }

    pub(crate) fn set_receive_buffer_size(&mut self, value: u64) -> Result<(), ErrorCode> {
        let res = {
            let fd = &*self.as_std_view()?;
            set_receive_buffer_size(fd, value)?
        };
        self.options.set_receive_buffer_size(res);
        Ok(())
    }

    pub(crate) fn send_buffer_size(&self) -> Result<u64, ErrorCode> {
        let fd = &*self.as_std_view()?;
        let n = send_buffer_size(fd)?;
        Ok(n)
    }

    pub(crate) fn set_send_buffer_size(&mut self, value: u64) -> Result<(), ErrorCode> {
        let res = {
            let fd = &*self.as_std_view()?;
            set_send_buffer_size(fd, value)?
        };
        self.options.set_send_buffer_size(res);
        Ok(())
    }

    #[cfg(feature = "p3")]
    pub(crate) fn non_inherited_options(&self) -> &NonInheritedOptions {
        &self.options
    }

    #[cfg(feature = "p3")]
    pub(crate) fn tcp_listener_arc(&self) -> Result<&Arc<tokio::net::TcpListener>, ErrorCode> {
        match &self.tcp_state {
            TcpState::Listening { listener, .. } => Ok(listener),
            #[cfg(feature = "p3")]
            TcpState::Error(err) => Err(err.into()),
            _ => Err(ErrorCode::InvalidState),
        }
    }

    pub(crate) fn tcp_stream_arc(&self) -> Result<&Arc<tokio::net::TcpStream>, ErrorCode> {
        match &self.tcp_state {
            TcpState::Connected(socket) => Ok(socket),
            #[cfg(feature = "p3")]
            TcpState::Receiving(socket) => Ok(socket),
            TcpState::P2Streaming(state) => Ok(&state.stream),
            #[cfg(feature = "p3")]
            TcpState::Error(err) => Err(err.into()),
            _ => Err(ErrorCode::InvalidState),
        }
    }

    pub(crate) fn p2_streaming_state(&self) -> Result<&P2TcpStreamingState, ErrorCode> {
        match &self.tcp_state {
            TcpState::P2Streaming(state) => Ok(state),
            #[cfg(feature = "p3")]
            TcpState::Error(err) => Err(err.into()),
            _ => Err(ErrorCode::InvalidState),
        }
    }

    pub(crate) fn set_p2_streaming_state(
        &mut self,
        state: P2TcpStreamingState,
    ) -> Result<(), ErrorCode> {
        if !matches!(self.tcp_state, TcpState::Connected(_)) {
            return Err(ErrorCode::InvalidState);
        }
        self.tcp_state = TcpState::P2Streaming(Box::new(state));
        Ok(())
    }

    /// Used for `Pollable` in the WASIp2 implementation this awaits the socket
    /// to be connected, if in the connecting state, or for a TCP accept to be
    /// ready, if this is in the listening state.
    ///
    /// For all other states this method immediately returns.
    pub(crate) async fn ready(&mut self) {
        match &mut self.tcp_state {
            TcpState::Default(..)
            | TcpState::BindStarted(..)
            | TcpState::Bound(..)
            | TcpState::ListenStarted(..)
            | TcpState::ConnectReady(..)
            | TcpState::Closed
            | TcpState::Connected { .. }
            | TcpState::Connecting(None)
            | TcpState::Listening {
                pending_accept: Some(_),
                ..
            }
            | TcpState::P2Streaming(_) => {}

            #[cfg(feature = "p3")]
            TcpState::Receiving(_) | TcpState::Error(_) => {}

            TcpState::Connecting(Some(future)) => {
                self.tcp_state = TcpState::ConnectReady(future.as_mut().await);
            }

            TcpState::Listening {
                listener,
                pending_accept: slot @ None,
            } => {
                let result = futures::future::poll_fn(|cx| {
                    listener.poll_accept(cx).map_ok(|(stream, _)| stream)
                })
                .await;
                *slot = Some(result);
            }
        }
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

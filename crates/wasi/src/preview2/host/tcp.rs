use crate::preview2::bindings::sockets::{
    network::{ErrorCode, IpAddressFamily, IpSocketAddress, Network},
    tcp::ShutdownType,
};
use crate::preview2::host::network::util;
use crate::preview2::network::{SocketAddrUse, SocketAddressFamily};
use crate::preview2::pipe::{AsyncReadStream, AsyncWriteStream};
use crate::preview2::tcp::{SystemTcpReader, SystemTcpWriter};
use crate::preview2::{
    with_ambient_tokio_runtime, InputStream, OutputStream, Pollable, SocketAddrFamily,
    SocketResult, Subscribe, WasiView,
};
use cap_net_ext::Blocking;
use io_lifetimes::raw::{FromRawSocketlike, IntoRawSocketlike};
use io_lifetimes::AsSocketlike;
use rustix::io::Errno;
use rustix::net::sockopt;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::Interest;
use wasmtime::component::Resource;

impl<T: WasiView> crate::preview2::bindings::sockets::tcp::Host for T {}

/// The state of a TCP socket.
///
/// This represents the various states a socket can be in during the
/// activities of binding, listening, accepting, and connecting.
enum TcpState {
    /// The initial state for a newly-created socket.
    Default,

    /// Binding started via `start_bind`.
    BindStarted,

    /// Binding finished via `finish_bind`. The socket has an address but
    /// is not yet listening for connections.
    Bound,

    /// Listening started via `listen_start`.
    ListenStarted,

    /// The socket is now listening and waiting for an incoming connection.
    Listening,

    /// An outgoing connection is started via `start_connect`.
    Connecting,

    /// An outgoing connection is ready to be established.
    ConnectReady,

    /// An outgoing connection was attempted but failed.
    ConnectFailed,

    /// An outgoing connection has been established.
    Connected,
}

/// A host TCP socket, plus associated bookkeeping.
///
/// The inner state is wrapped in an Arc because the same underlying socket is
/// used for implementing the stream types.
pub struct TcpSocketWrapper {
    /// The part of a `TcpSocket` which is reference-counted so that we
    /// can pass it to async tasks.
    inner: Arc<tokio::net::TcpStream>,

    /// The current state in the bind/listen/accept/connect progression.
    tcp_state: TcpState,

    /// The desired listen queue size. Set to None to use the system's default.
    listen_backlog_size: Option<i32>,

    family: SocketAddressFamily,

    // The socket options below are not automatically inherited from the listener
    // on all platforms. So we keep track of which options have been explicitly
    // set and manually apply those values to newly accepted clients.
    #[cfg(target_os = "macos")]
    receive_buffer_size: Option<usize>,
    #[cfg(target_os = "macos")]
    send_buffer_size: Option<usize>,
    #[cfg(target_os = "macos")]
    hop_limit: Option<u8>,
    #[cfg(target_os = "macos")]
    keep_alive_idle_time: Option<std::time::Duration>,
}

impl TcpSocketWrapper {
    /// Create a new socket in the given family.
    pub fn new(family: SocketAddrFamily) -> io::Result<Self> {
        // Create a new host socket and set it to non-blocking, which is needed
        // by our async implementation.
        let fd = util::tcp_socket(family, Blocking::No)?;

        let socket_address_family = match family {
            SocketAddrFamily::V4 => SocketAddressFamily::Ipv4,
            SocketAddrFamily::V6 => SocketAddressFamily::Ipv6 {
                v6only: sockopt::get_ipv6_v6only(&fd)?,
            },
        };

        Self::from_fd(fd, socket_address_family)
    }

    /// Create a `TcpSocket` from an existing socket.
    ///
    /// The socket must be in non-blocking mode.
    pub(crate) fn from_fd(
        fd: rustix::fd::OwnedFd,
        family: SocketAddressFamily,
    ) -> io::Result<Self> {
        let stream = Self::setup_tokio_tcp_stream(fd)?;

        Ok(Self {
            inner: Arc::new(stream),
            tcp_state: TcpState::Default,
            listen_backlog_size: None,
            family,
            #[cfg(target_os = "macos")]
            receive_buffer_size: None,
            #[cfg(target_os = "macos")]
            send_buffer_size: None,
            #[cfg(target_os = "macos")]
            hop_limit: None,
            #[cfg(target_os = "macos")]
            keep_alive_idle_time: None,
        })
    }

    fn setup_tokio_tcp_stream(fd: rustix::fd::OwnedFd) -> io::Result<tokio::net::TcpStream> {
        let std_stream =
            unsafe { std::net::TcpStream::from_raw_socketlike(fd.into_raw_socketlike()) };
        with_ambient_tokio_runtime(|| tokio::net::TcpStream::try_from(std_stream))
    }

    pub fn tcp_socket(&self) -> &tokio::net::TcpStream {
        &self.inner
    }

    /// Create the input/output stream pair for a tcp socket.
    pub fn as_split(&self) -> (InputStream, OutputStream) {
        const SOCKET_READY_SIZE: usize = 1024 * 1024 * 1024;

        let reader = SystemTcpReader::new(self.inner.clone());
        let writer = SystemTcpWriter::new(self.inner.clone());

        let input = Box::new(AsyncReadStream::new(reader));
        let output = Box::new(AsyncWriteStream::new(SOCKET_READY_SIZE, writer));
        (InputStream::Host(input), output)
    }
}

impl<T: WasiView> crate::preview2::bindings::sockets::tcp_create_socket::Host for T {
    fn create_tcp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<TcpSocketWrapper>> {
        let socket = TcpSocketWrapper::new(address_family.into())?;
        let socket = self.table_mut().push(socket)?;
        Ok(socket)
    }
}

impl<T: WasiView> crate::preview2::bindings::sockets::tcp::HostTcpSocket for T {
    fn start_bind(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        self.ctx().allowed_network_uses.check_allowed_tcp()?;
        let table = self.table_mut();
        let socket = table.get(&this)?;
        let network = table.get(&network)?;
        let local_address: SocketAddr = local_address.into();

        match socket.tcp_state {
            TcpState::Default => {}
            TcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        util::validate_unicast(&local_address)?;
        util::validate_address_family(&local_address, &socket.family)?;

        {
            // Ensure that we're allowed to connect to this address.
            network.check_socket_addr(&local_address, SocketAddrUse::TcpBind)?;

            // Automatically bypass the TIME_WAIT state when the user is trying
            // to bind to a specific port:
            let reuse_addr = local_address.port() > 0;

            // Unconditionally (re)set SO_REUSEADDR, even when the value is false.
            // This ensures we're not accidentally affected by any socket option
            // state left behind by a previous failed call to this method (start_bind).
            util::set_tcp_reuseaddr(socket.tcp_socket(), reuse_addr)?;

            // Perform the OS bind call.
            util::tcp_bind(socket.tcp_socket(), &local_address).map_err(|error| match error {
                // From https://pubs.opengroup.org/onlinepubs/9699919799/functions/bind.html:
                // > [EAFNOSUPPORT] The specified address is not a valid address for the address family of the specified socket
                //
                // The most common reasons for this error should have already
                // been handled by our own validation slightly higher up in this
                // function. This error mapping is here just in case there is
                // an edge case we didn't catch.
                Errno::AFNOSUPPORT => ErrorCode::InvalidArgument,
                _ => ErrorCode::from(error),
            })?;
        }

        let socket = table.get_mut(&this)?;
        socket.tcp_state = TcpState::BindStarted;

        Ok(())
    }

    fn finish_bind(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        match socket.tcp_state {
            TcpState::BindStarted => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        socket.tcp_state = TcpState::Bound;

        Ok(())
    }

    fn start_connect(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        network: Resource<Network>,
        remote_address: IpSocketAddress,
    ) -> SocketResult<()> {
        self.ctx().allowed_network_uses.check_allowed_tcp()?;
        let table = self.table_mut();
        let r = {
            let socket = table.get(&this)?;
            let network = table.get(&network)?;
            let remote_address: SocketAddr = remote_address.into();

            match socket.tcp_state {
                TcpState::Default => {}
                TcpState::Bound
                | TcpState::Connected
                | TcpState::ConnectFailed
                | TcpState::Listening => return Err(ErrorCode::InvalidState.into()),
                TcpState::Connecting
                | TcpState::ConnectReady
                | TcpState::ListenStarted
                | TcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            }

            util::validate_unicast(&remote_address)?;
            util::validate_remote_address(&remote_address)?;
            util::validate_address_family(&remote_address, &socket.family)?;

            // Ensure that we're allowed to connect to this address.
            network.check_socket_addr(&remote_address, SocketAddrUse::TcpConnect)?;

            // Do an OS `connect`. Our socket is non-blocking, so it'll either...
            util::tcp_connect(socket.tcp_socket(), &remote_address)
        };

        match r {
            // succeed immediately,
            Ok(()) => {
                let socket = table.get_mut(&this)?;
                socket.tcp_state = TcpState::ConnectReady;
                return Ok(());
            }
            // continue in progress,
            Err(err) if err == Errno::INPROGRESS => {}
            // or fail immediately.
            Err(err) => {
                return Err(match err {
                    Errno::AFNOSUPPORT => ErrorCode::InvalidArgument.into(), // See `bind` implementation.
                    _ => err.into(),
                });
            }
        }

        let socket = table.get_mut(&this)?;
        socket.tcp_state = TcpState::Connecting;

        Ok(())
    }

    fn finish_connect(
        &mut self,
        this: Resource<TcpSocketWrapper>,
    ) -> SocketResult<(Resource<InputStream>, Resource<OutputStream>)> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        match socket.tcp_state {
            TcpState::ConnectReady => {}
            TcpState::Connecting => {
                // Do a `poll` to test for completion, using a timeout of zero
                // to avoid blocking.
                match rustix::event::poll(
                    &mut [rustix::event::PollFd::new(
                        socket.tcp_socket(),
                        rustix::event::PollFlags::OUT,
                    )],
                    0,
                ) {
                    Ok(0) => return Err(ErrorCode::WouldBlock.into()),
                    Ok(_) => (),
                    Err(err) => Err(err).unwrap(),
                }

                // Check whether the connect succeeded.
                match sockopt::get_socket_error(socket.tcp_socket()) {
                    Ok(Ok(())) => {}
                    Err(err) | Ok(Err(err)) => {
                        socket.tcp_state = TcpState::ConnectFailed;
                        return Err(err.into());
                    }
                }
            }
            _ => return Err(ErrorCode::NotInProgress.into()),
        };

        socket.tcp_state = TcpState::Connected;
        let (input, output) = socket.as_split();
        let input_stream = self.table_mut().push_child(input, &this)?;
        let output_stream = self.table_mut().push_child(output, &this)?;

        Ok((input_stream, output_stream))
    }

    fn start_listen(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<()> {
        self.ctx().allowed_network_uses.check_allowed_tcp()?;
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        match socket.tcp_state {
            TcpState::Bound => {}
            TcpState::Default
            | TcpState::Connected
            | TcpState::ConnectFailed
            | TcpState::Listening => return Err(ErrorCode::InvalidState.into()),
            TcpState::ListenStarted
            | TcpState::Connecting
            | TcpState::ConnectReady
            | TcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
        }

        util::tcp_listen(socket.tcp_socket(), socket.listen_backlog_size)?;

        socket.tcp_state = TcpState::ListenStarted;

        Ok(())
    }

    fn finish_listen(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        match socket.tcp_state {
            TcpState::ListenStarted => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        socket.tcp_state = TcpState::Listening;

        Ok(())
    }

    fn accept(
        &mut self,
        this: Resource<TcpSocketWrapper>,
    ) -> SocketResult<(
        Resource<TcpSocketWrapper>,
        Resource<InputStream>,
        Resource<OutputStream>,
    )> {
        self.ctx().allowed_network_uses.check_allowed_tcp()?;
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.tcp_state {
            TcpState::Listening => {}
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        // Do the OS accept call.
        let tcp_socket = socket.tcp_socket();
        let (client_fd, _addr) = tcp_socket.try_io(Interest::READABLE, || {
            util::tcp_accept(tcp_socket, Blocking::No)
        })?;

        #[cfg(target_os = "macos")]
        {
            // Manually inherit socket options from listener. We only have to
            // do this on platforms that don't already do this automatically
            // and only if a specific value was explicitly set on the listener.

            if let Some(size) = socket.receive_buffer_size {
                _ = util::set_socket_recv_buffer_size(&client_fd, size); // Ignore potential error.
            }

            if let Some(size) = socket.send_buffer_size {
                _ = util::set_socket_send_buffer_size(&client_fd, size); // Ignore potential error.
            }

            // For some reason, IP_TTL is inherited, but IPV6_UNICAST_HOPS isn't.
            if let (SocketAddressFamily::Ipv6 { .. }, Some(ttl)) = (socket.family, socket.hop_limit)
            {
                _ = util::set_ipv6_unicast_hops(&client_fd, ttl); // Ignore potential error.
            }

            if let Some(value) = socket.keep_alive_idle_time {
                _ = util::set_tcp_keepidle(&client_fd, value); // Ignore potential error.
            }
        }

        let mut tcp_socket = TcpSocketWrapper::from_fd(client_fd, socket.family)?;

        // Mark the socket as connected so that we can exit early from methods like `start-bind`.
        tcp_socket.tcp_state = TcpState::Connected;

        let (input, output) = tcp_socket.as_split();
        let output: OutputStream = output;

        let tcp_socket = self.table_mut().push(tcp_socket)?;
        let input_stream = self.table_mut().push_child(input, &tcp_socket)?;
        let output_stream = self.table_mut().push_child(output, &tcp_socket)?;

        Ok((tcp_socket, input_stream, output_stream))
    }

    fn local_address(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.tcp_state {
            TcpState::Default => return Err(ErrorCode::InvalidState.into()),
            TcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => {}
        }

        let addr = socket
            .tcp_socket()
            .as_socketlike_view::<std::net::TcpStream>()
            .local_addr()?;
        Ok(addr.into())
    }

    fn remote_address(
        &mut self,
        this: Resource<TcpSocketWrapper>,
    ) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.tcp_state {
            TcpState::Connected => {}
            TcpState::Connecting | TcpState::ConnectReady => {
                return Err(ErrorCode::ConcurrencyConflict.into())
            }
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        let addr = socket
            .tcp_socket()
            .as_socketlike_view::<std::net::TcpStream>()
            .peer_addr()?;
        Ok(addr.into())
    }

    fn is_listening(&mut self, this: Resource<TcpSocketWrapper>) -> Result<bool, anyhow::Error> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.tcp_state {
            TcpState::Listening => Ok(true),
            _ => Ok(false),
        }
    }

    fn address_family(
        &mut self,
        this: Resource<TcpSocketWrapper>,
    ) -> Result<IpAddressFamily, anyhow::Error> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.family {
            SocketAddressFamily::Ipv4 => Ok(IpAddressFamily::Ipv4),
            SocketAddressFamily::Ipv6 { .. } => Ok(IpAddressFamily::Ipv6),
        }
    }

    fn ipv6_only(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<bool> {
        let table = self.table();
        let socket = table.get(&this)?;

        // Instead of just calling the OS we return our own internal state, because
        // MacOS doesn't propagate the V6ONLY state on to accepted client sockets.

        match socket.family {
            SocketAddressFamily::Ipv4 => Err(ErrorCode::NotSupported.into()),
            SocketAddressFamily::Ipv6 { v6only } => Ok(v6only),
        }
    }

    fn set_ipv6_only(&mut self, this: Resource<TcpSocketWrapper>, value: bool) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        match socket.family {
            SocketAddressFamily::Ipv4 => Err(ErrorCode::NotSupported.into()),
            SocketAddressFamily::Ipv6 { .. } => match socket.tcp_state {
                TcpState::Default => {
                    sockopt::set_ipv6_v6only(socket.tcp_socket(), value)?;
                    socket.family = SocketAddressFamily::Ipv6 { v6only: value };
                    Ok(())
                }
                TcpState::BindStarted => Err(ErrorCode::ConcurrencyConflict.into()),
                _ => Err(ErrorCode::InvalidState.into()),
            },
        }
    }

    fn set_listen_backlog_size(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: u64,
    ) -> SocketResult<()> {
        const MIN_BACKLOG: i32 = 1;
        const MAX_BACKLOG: i32 = i32::MAX; // OS'es will most likely limit it down even further.

        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        if value == 0 {
            return Err(ErrorCode::InvalidArgument.into());
        }

        // Silently clamp backlog size. This is OK for us to do, because operating systems do this too.
        let value = value
            .try_into()
            .unwrap_or(i32::MAX)
            .clamp(MIN_BACKLOG, MAX_BACKLOG);

        match socket.tcp_state {
            TcpState::Default | TcpState::BindStarted | TcpState::Bound => {
                // Socket not listening yet. Stash value for first invocation to `listen`.
                socket.listen_backlog_size = Some(value);

                Ok(())
            }
            TcpState::Listening => {
                // Try to update the backlog by calling `listen` again.
                // Not all platforms support this. We'll only update our own value if the OS supports changing the backlog size after the fact.

                rustix::net::listen(socket.tcp_socket(), value)
                    .map_err(|_| ErrorCode::NotSupported)?;

                socket.listen_backlog_size = Some(value);

                Ok(())
            }
            TcpState::Connected | TcpState::ConnectFailed => Err(ErrorCode::InvalidState.into()),
            TcpState::Connecting | TcpState::ConnectReady | TcpState::ListenStarted => {
                Err(ErrorCode::ConcurrencyConflict.into())
            }
        }
    }

    fn keep_alive_enabled(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<bool> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(sockopt::get_socket_keepalive(socket.tcp_socket())?)
    }

    fn set_keep_alive_enabled(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: bool,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(sockopt::set_socket_keepalive(socket.tcp_socket(), value)?)
    }

    fn keep_alive_idle_time(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(sockopt::get_tcp_keepidle(socket.tcp_socket())?.as_nanos() as u64)
    }

    fn set_keep_alive_idle_time(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        let duration = Duration::from_nanos(value);

        util::set_tcp_keepidle(socket.tcp_socket(), duration)?;

        #[cfg(target_os = "macos")]
        {
            socket.keep_alive_idle_time = Some(duration);
        }

        Ok(())
    }

    fn keep_alive_interval(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(sockopt::get_tcp_keepintvl(socket.tcp_socket())?.as_nanos() as u64)
    }

    fn set_keep_alive_interval(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(util::set_tcp_keepintvl(
            socket.tcp_socket(),
            Duration::from_nanos(value),
        )?)
    }

    fn keep_alive_count(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<u32> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(sockopt::get_tcp_keepcnt(socket.tcp_socket())?)
    }

    fn set_keep_alive_count(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: u32,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(util::set_tcp_keepcnt(socket.tcp_socket(), value)?)
    }

    fn hop_limit(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<u8> {
        let table = self.table();
        let socket = table.get(&this)?;

        let ttl = match socket.family {
            SocketAddressFamily::Ipv4 => util::get_ip_ttl(socket.tcp_socket())?,
            SocketAddressFamily::Ipv6 { .. } => util::get_ipv6_unicast_hops(socket.tcp_socket())?,
        };

        Ok(ttl)
    }

    fn set_hop_limit(&mut self, this: Resource<TcpSocketWrapper>, value: u8) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        match socket.family {
            SocketAddressFamily::Ipv4 => util::set_ip_ttl(socket.tcp_socket(), value)?,
            SocketAddressFamily::Ipv6 { .. } => {
                util::set_ipv6_unicast_hops(socket.tcp_socket(), value)?
            }
        }

        #[cfg(target_os = "macos")]
        {
            socket.hop_limit = Some(value);
        }

        Ok(())
    }

    fn receive_buffer_size(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;

        let value = util::get_socket_recv_buffer_size(socket.tcp_socket())?;
        Ok(value as u64)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);

        util::set_socket_recv_buffer_size(socket.tcp_socket(), value)?;

        #[cfg(target_os = "macos")]
        {
            socket.receive_buffer_size = Some(value);
        }

        Ok(())
    }

    fn send_buffer_size(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;

        let value = util::get_socket_send_buffer_size(socket.tcp_socket())?;
        Ok(value as u64)
    }

    fn set_send_buffer_size(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);

        util::set_socket_send_buffer_size(socket.tcp_socket(), value)?;

        #[cfg(target_os = "macos")]
        {
            socket.send_buffer_size = Some(value);
        }

        Ok(())
    }

    fn subscribe(
        &mut self,
        this: Resource<TcpSocketWrapper>,
    ) -> anyhow::Result<Resource<Pollable>> {
        crate::preview2::poll::subscribe(self.table_mut(), this)
    }

    fn shutdown(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        shutdown_type: ShutdownType,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.tcp_state {
            TcpState::Connected => {}
            TcpState::Connecting | TcpState::ConnectReady => {
                return Err(ErrorCode::ConcurrencyConflict.into())
            }
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        let how = match shutdown_type {
            ShutdownType::Receive => std::net::Shutdown::Read,
            ShutdownType::Send => std::net::Shutdown::Write,
            ShutdownType::Both => std::net::Shutdown::Both,
        };

        socket
            .tcp_socket()
            .as_socketlike_view::<std::net::TcpStream>()
            .shutdown(how)?;
        Ok(())
    }

    fn drop(&mut self, this: Resource<TcpSocketWrapper>) -> anyhow::Result<()> {
        let table = self.table_mut();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

#[async_trait::async_trait]
impl Subscribe for TcpSocketWrapper {
    async fn ready(&mut self) {
        // Some states are ready immediately.
        match self.tcp_state {
            TcpState::BindStarted | TcpState::ListenStarted | TcpState::ConnectReady => return,
            _ => {}
        }

        // FIXME: Add `Interest::ERROR` when we update to tokio 1.32.
        self.inner
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await
            .unwrap();
    }
}

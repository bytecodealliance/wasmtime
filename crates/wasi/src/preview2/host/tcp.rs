use crate::preview2::bindings::{
    io::streams::{InputStream, OutputStream},
    poll::poll::Pollable,
    sockets::network::{self, ErrorCode, IpAddressFamily, IpSocketAddress, Network},
    sockets::tcp::{self, ShutdownType},
};
use crate::preview2::network::TableNetworkExt;
use crate::preview2::poll::TablePollableExt;
use crate::preview2::stream::TableStreamExt;
use crate::preview2::tcp::{HostTcpSocket, HostTcpSocketInner, HostTcpState, TableTcpSocketExt};
use crate::preview2::{HostPollable, PollableFuture, WasiView};
use cap_net_ext::{Blocking, PoolExt, TcpListenerExt};
use io_lifetimes::AsSocketlike;
use rustix::io::Errno;
use rustix::net::sockopt;
use std::any::Any;
use std::mem;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLockWriteGuard;
#[cfg(unix)]
use tokio::task::spawn;
#[cfg(not(unix))]
use tokio::task::spawn_blocking;
use tokio::task::JoinHandle;

impl<T: WasiView> tcp::Host for T {
    fn start_bind(
        &mut self,
        this: tcp::TcpSocket,
        network: Network,
        local_address: IpSocketAddress,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let tcp_state = socket.tcp_state_write_lock();
        match &*tcp_state {
            HostTcpState::Default => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        let network = table.get_network(network)?;
        let binder = network.0.tcp_binder(local_address)?;

        // Perform the OS bind call.
        binder.bind_existing_tcp_listener(socket.tcp_socket())?;

        set_state(tcp_state, HostTcpState::BindStarted);
        socket.notify();

        Ok(())
    }

    fn finish_bind(&mut self, this: tcp::TcpSocket) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let tcp_state = socket.tcp_state_write_lock();
        match &*tcp_state {
            HostTcpState::BindStarted => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        set_state(tcp_state, HostTcpState::Bound);

        Ok(())
    }

    fn start_connect(
        &mut self,
        this: tcp::TcpSocket,
        network: Network,
        remote_address: IpSocketAddress,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let tcp_state = socket.tcp_state_write_lock();
        match &*tcp_state {
            HostTcpState::Default => {}
            HostTcpState::Connected => return Err(ErrorCode::AlreadyConnected.into()),
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        let network = table.get_network(network)?;
        let connecter = network.0.tcp_connecter(remote_address)?;

        // Do an OS `connect`. Our socket is non-blocking, so it'll either...
        match connecter.connect_existing_tcp_listener(socket.tcp_socket()) {
            // succeed immediately,
            Ok(()) => {
                set_state(tcp_state, HostTcpState::ConnectReady(Ok(())));
                socket.notify();
                return Ok(());
            }
            // continue in progress,
            Err(err) if err.raw_os_error() == Some(INPROGRESS.raw_os_error()) => {}
            // or fail immediately.
            Err(err) => return Err(err.into()),
        }

        // The connect is continuing in progress. Set up the join handle.

        let clone = socket.clone_inner();

        #[cfg(unix)]
        let join = spawn(async move {
            let result = match clone.tcp_socket.writable().await {
                Ok(mut writable) => {
                    writable.retain_ready();

                    // Check whether the connect succeeded.
                    match sockopt::get_socket_error(&clone.tcp_socket) {
                        Ok(Ok(())) => Ok(()),
                        Err(err) | Ok(Err(err)) => Err(err.into()),
                    }
                }
                Err(err) => Err(err),
            };

            clone.set_state_and_notify(HostTcpState::ConnectReady(result));
        });

        #[cfg(not(unix))]
        let join = spawn_blocking(move || {
            let result = match rustix::event::poll(
                &mut [rustix::event::PollFd::new(
                    &clone.tcp_socket,
                    rustix::event::PollFlags::OUT,
                )],
                -1,
            ) {
                Ok(_) => {
                    // Check whether the connect succeeded.
                    match sockopt::get_socket_error(&clone.tcp_socket) {
                        Ok(Ok(())) => Ok(()),
                        Err(err) | Ok(Err(err)) => Err(err.into()),
                    }
                }
                Err(err) => Err(err.into()),
            };

            clone.set_state_and_notify(HostTcpState::ConnectReady(result));
        });

        set_state(
            tcp_state,
            HostTcpState::Connecting(Pin::from(Box::new(join))),
        );

        Ok(())
    }

    fn finish_connect(
        &mut self,
        this: tcp::TcpSocket,
    ) -> Result<(InputStream, OutputStream), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let mut tcp_state = socket.tcp_state_write_lock();
        match &mut *tcp_state {
            HostTcpState::ConnectReady(_) => {}
            HostTcpState::Connecting(join) => match maybe_unwrap_future(join) {
                Some(joined) => joined.unwrap(),
                None => return Err(ErrorCode::WouldBlock.into()),
            },
            _ => return Err(ErrorCode::NotInProgress.into()),
        };

        let old_state = mem::replace(&mut *tcp_state, HostTcpState::Connected);

        // Extract the connection result.
        let result = match old_state {
            HostTcpState::ConnectReady(result) => result,
            _ => unreachable!(),
        };

        // Report errors, resetting the state if needed.
        match result {
            Ok(()) => {}
            Err(err) => {
                set_state(tcp_state, HostTcpState::Default);
                return Err(err.into());
            }
        }

        drop(tcp_state);

        let input_clone = socket.clone_inner();
        let output_clone = socket.clone_inner();

        let input_stream = self.table_mut().push_input_stream(Box::new(input_clone))?;
        let output_stream = self
            .table_mut()
            .push_output_stream(Box::new(output_clone))?;

        Ok((input_stream, output_stream))
    }

    fn start_listen(&mut self, this: tcp::TcpSocket) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let tcp_state = socket.tcp_state_write_lock();
        match &*tcp_state {
            HostTcpState::Bound => {}
            HostTcpState::ListenStarted => return Err(ErrorCode::AlreadyListening.into()),
            HostTcpState::Connected => return Err(ErrorCode::AlreadyConnected.into()),
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        socket.tcp_socket().listen(None)?;

        set_state(tcp_state, HostTcpState::ListenStarted);
        socket.notify();

        Ok(())
    }

    fn finish_listen(&mut self, this: tcp::TcpSocket) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let tcp_state = socket.tcp_state_write_lock();

        match &*tcp_state {
            HostTcpState::ListenStarted => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        let new_join = spawn_task_to_wait_for_connections(socket.clone_inner());
        set_state(
            tcp_state,
            HostTcpState::Listening(Pin::from(Box::new(new_join))),
        );

        Ok(())
    }

    fn accept(
        &mut self,
        this: tcp::TcpSocket,
    ) -> Result<(tcp::TcpSocket, InputStream, OutputStream), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let mut tcp_state = socket.tcp_state_write_lock();
        match &mut *tcp_state {
            HostTcpState::ListenReady(_) => {}
            HostTcpState::Listening(join) => match maybe_unwrap_future(join) {
                Some(joined) => joined.unwrap(),
                None => return Err(ErrorCode::WouldBlock.into()),
            },
            HostTcpState::Connected => return Err(ErrorCode::AlreadyConnected.into()),
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        let new_join = spawn_task_to_wait_for_connections(socket.clone_inner());
        set_state(
            tcp_state,
            HostTcpState::Listening(Pin::from(Box::new(new_join))),
        );

        // Do the OS accept call.
        let (connection, _addr) = socket.tcp_socket().accept_with(Blocking::No)?;
        let tcp_socket = HostTcpSocket::from_tcp_stream(connection)?;

        let input_clone = tcp_socket.clone_inner();
        let output_clone = tcp_socket.clone_inner();

        let tcp_socket = self.table_mut().push_tcp_socket(tcp_socket)?;
        let input_stream = self.table_mut().push_input_stream(Box::new(input_clone))?;
        let output_stream = self
            .table_mut()
            .push_output_stream(Box::new(output_clone))?;

        Ok((tcp_socket, input_stream, output_stream))
    }

    fn local_address(&mut self, this: tcp::TcpSocket) -> Result<IpSocketAddress, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        let addr = socket
            .tcp_socket()
            .as_socketlike_view::<std::net::TcpStream>()
            .local_addr()?;
        Ok(addr.into())
    }

    fn remote_address(&mut self, this: tcp::TcpSocket) -> Result<IpSocketAddress, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        let addr = socket
            .tcp_socket()
            .as_socketlike_view::<std::net::TcpStream>()
            .peer_addr()?;
        Ok(addr.into())
    }

    fn address_family(&mut self, this: tcp::TcpSocket) -> Result<IpAddressFamily, anyhow::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        // If `SO_DOMAIN` is available, use it.
        //
        // TODO: OpenBSD also supports this; upstream PRs are posted.
        #[cfg(not(any(
            windows,
            target_os = "ios",
            target_os = "macos",
            target_os = "netbsd",
            target_os = "openbsd"
        )))]
        {
            use rustix::net::AddressFamily;

            let family = sockopt::get_socket_domain(socket.tcp_socket())?;
            let family = match family {
                AddressFamily::INET => IpAddressFamily::Ipv4,
                AddressFamily::INET6 => IpAddressFamily::Ipv6,
                _ => return Err(ErrorCode::NotSupported.into()),
            };
            Ok(family)
        }

        // When `SO_DOMAIN` is not available, emulate it.
        #[cfg(any(
            windows,
            target_os = "ios",
            target_os = "macos",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            if let Ok(_) = sockopt::get_ipv6_unicast_hops(socket.tcp_socket()) {
                return Ok(IpAddressFamily::Ipv6);
            }
            if let Ok(_) = sockopt::get_ip_ttl(socket.tcp_socket()) {
                return Ok(IpAddressFamily::Ipv4);
            }
            Err(ErrorCode::NotSupported.into())
        }
    }

    fn ipv6_only(&mut self, this: tcp::TcpSocket) -> Result<bool, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::get_ipv6_v6only(socket.tcp_socket())?)
    }

    fn set_ipv6_only(&mut self, this: tcp::TcpSocket, value: bool) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::set_ipv6_v6only(socket.tcp_socket(), value)?)
    }

    fn set_listen_backlog_size(
        &mut self,
        this: tcp::TcpSocket,
        value: u64,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        let tcp_state = socket.tcp_state_read_lock();
        match &*tcp_state {
            HostTcpState::Listening(_) => {}
            _ => return Err(ErrorCode::NotInProgress.into()),
        }

        let value = value.try_into().map_err(|_| ErrorCode::OutOfMemory)?;
        Ok(rustix::net::listen(socket.tcp_socket(), value)?)
    }

    fn keep_alive(&mut self, this: tcp::TcpSocket) -> Result<bool, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::get_socket_keepalive(socket.tcp_socket())?)
    }

    fn set_keep_alive(&mut self, this: tcp::TcpSocket, value: bool) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::set_socket_keepalive(socket.tcp_socket(), value)?)
    }

    fn no_delay(&mut self, this: tcp::TcpSocket) -> Result<bool, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::get_tcp_nodelay(socket.tcp_socket())?)
    }

    fn set_no_delay(&mut self, this: tcp::TcpSocket, value: bool) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::set_tcp_nodelay(socket.tcp_socket(), value)?)
    }

    fn unicast_hop_limit(&mut self, this: tcp::TcpSocket) -> Result<u8, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        // We don't track whether the socket is IPv4 or IPv6 so try one and
        // fall back to the other.
        match sockopt::get_ipv6_unicast_hops(socket.tcp_socket()) {
            Ok(value) => Ok(value),
            Err(Errno::NOPROTOOPT) => {
                let value = sockopt::get_ip_ttl(socket.tcp_socket())?;
                let value = value.try_into().unwrap();
                Ok(value)
            }
            Err(err) => Err(err.into()),
        }
    }

    fn set_unicast_hop_limit(
        &mut self,
        this: tcp::TcpSocket,
        value: u8,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

        // We don't track whether the socket is IPv4 or IPv6 so try one and
        // fall back to the other.
        match sockopt::set_ipv6_unicast_hops(socket.tcp_socket(), Some(value)) {
            Ok(()) => Ok(()),
            Err(Errno::NOPROTOOPT) => Ok(sockopt::set_ip_ttl(socket.tcp_socket(), value.into())?),
            Err(err) => Err(err.into()),
        }
    }

    fn receive_buffer_size(&mut self, this: tcp::TcpSocket) -> Result<u64, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::get_socket_recv_buffer_size(socket.tcp_socket())? as u64)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: tcp::TcpSocket,
        value: u64,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        let value = value.try_into().map_err(|_| ErrorCode::OutOfMemory)?;
        Ok(sockopt::set_socket_recv_buffer_size(
            socket.tcp_socket(),
            value,
        )?)
    }

    fn send_buffer_size(&mut self, this: tcp::TcpSocket) -> Result<u64, network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        Ok(sockopt::get_socket_send_buffer_size(socket.tcp_socket())? as u64)
    }

    fn set_send_buffer_size(
        &mut self,
        this: tcp::TcpSocket,
        value: u64,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;
        let value = value.try_into().map_err(|_| ErrorCode::OutOfMemory)?;
        Ok(sockopt::set_socket_send_buffer_size(
            socket.tcp_socket(),
            value,
        )?)
    }

    fn subscribe(&mut self, this: tcp::TcpSocket) -> anyhow::Result<Pollable> {
        fn make_tcp_socket_future<'a>(stream: &'a mut dyn Any) -> PollableFuture<'a> {
            let socket = stream
                .downcast_mut::<HostTcpSocket>()
                .expect("downcast to HostTcpSocket failed");

            Box::pin(async {
                socket.receiver.changed().await.unwrap();
                Ok(())
            })
        }

        let pollable = HostPollable::TableEntry {
            index: this,
            make_future: make_tcp_socket_future,
        };

        Ok(self.table_mut().push_host_pollable(pollable)?)
    }

    fn shutdown(
        &mut self,
        this: tcp::TcpSocket,
        shutdown_type: ShutdownType,
    ) -> Result<(), network::Error> {
        let table = self.table();
        let socket = table.get_tcp_socket(this)?;

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

    fn drop_tcp_socket(&mut self, this: tcp::TcpSocket) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete_tcp_socket(this)?;

        // On non-Unix platforms, do a `shutdown` to wake up any `poll` calls
        // that are waiting.
        #[cfg(not(unix))]
        rustix::net::shutdown(&dropped.inner.tcp_socket, rustix::net::Shutdown::ReadWrite).unwrap();

        drop(dropped);

        Ok(())
    }
}

/// Spawn a task to monitor a socket for incoming connections that
/// can be `accept`ed.
fn spawn_task_to_wait_for_connections(socket: Arc<HostTcpSocketInner>) -> JoinHandle<()> {
    #[cfg(unix)]
    let join = spawn(async move {
        socket.tcp_socket.readable().await.unwrap().retain_ready();
        socket.set_state_and_notify(HostTcpState::ListenReady(Ok(())));
    });

    #[cfg(not(unix))]
    let join = spawn_blocking(move || {
        let result = match rustix::event::poll(
            &mut [rustix::event::PollFd::new(
                &socket.tcp_socket,
                rustix::event::PollFlags::IN,
            )],
            -1,
        ) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        };
        socket.set_state_and_notify(HostTcpState::ListenReady(result));
    });

    join
}

/// Set `*tcp_state` to `new_state` and consume `tcp_state`.
fn set_state(tcp_state: RwLockWriteGuard<HostTcpState>, new_state: HostTcpState) {
    let mut tcp_state = tcp_state;
    *tcp_state = new_state;
}

/// Given a future, return the finished value if it's already ready, or
/// `None` if it's not.
fn maybe_unwrap_future<F: std::future::Future + std::marker::Unpin>(
    future: &mut Pin<Box<F>>,
) -> Option<F::Output> {
    use std::ptr;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    // Create a no-op Waker. This is derived from [code in std] and can
    // be replaced with `std::task::Waker::noop()` when the "noop_waker"
    // feature is stablized.
    //
    // [code in std]: https://github.com/rust-lang/rust/blob/27fb598d51d4566a725e4868eaf5d2e15775193e/library/core/src/task/wake.rs#L349
    fn noop_waker() -> Waker {
        const VTABLE: RawWakerVTable = RawWakerVTable::new(
            // Cloning just returns a new no-op raw waker
            |_| RAW,
            // `wake` does nothing
            |_| {},
            // `wake_by_ref` does nothing
            |_| {},
            // Dropping does nothing as we don't allocate anything
            |_| {},
        );
        const RAW: RawWaker = RawWaker::new(ptr::null(), &VTABLE);

        unsafe { Waker::from_raw(RAW) }
    }

    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    match future.as_mut().poll(&mut cx) {
        Poll::Ready(val) => Some(val),
        Poll::Pending => None,
    }
}

// On POSIX, non-blocking TCP socket `connect` uses `EINPROGRESS`.
// <https://pubs.opengroup.org/onlinepubs/9699919799/functions/connect.html>
#[cfg(not(windows))]
const INPROGRESS: Errno = Errno::INPROGRESS;

// On Windows, non-blocking TCP socket `connect` uses `WSAEWOULDBLOCK`.
// <https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-connect>
#[cfg(windows)]
const INPROGRESS: Errno = Errno::WOULDBLOCK;

use crate::preview2::bindings::sockets::{
    network::{ErrorCode, IpAddressFamily, IpSocketAddress, Network},
    tcp::ShutdownType,
};
use crate::preview2::pipe::{AsyncReadStream, AsyncWriteStream};
use crate::preview2::tcp::{TcpReader, TcpSocket, TcpWriter};
use crate::preview2::{
    InputStream, OutputStream, Pollable, Preview2Future, SocketResult, Subscribe, WasiView,
};
use std::io;
use std::net::SocketAddr;
use std::task::{Context, Poll};
use std::time::Duration;
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
    Listening {
        pending_result: Option<io::Result<(Box<dyn TcpSocket>, TcpReader, TcpWriter)>>,
    },

    /// An outgoing connection is started via `start_connect`.
    Connecting {
        future: Preview2Future<io::Result<(TcpReader, TcpWriter)>>,
    },

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
    inner: Box<dyn TcpSocket>,

    /// The current state in the bind/listen/accept/connect progression.
    tcp_state: TcpState,
}

impl TcpSocketWrapper {
    fn new_input_stream(reader: TcpReader) -> InputStream {
        InputStream::Host(Box::new(AsyncReadStream::new(reader)))
    }

    fn new_output_stream(writer: TcpWriter) -> OutputStream {
        const SOCKET_READY_SIZE: usize = 1024 * 1024 * 1024;

        Box::new(AsyncWriteStream::new(SOCKET_READY_SIZE, writer))
    }
}

impl<T: WasiView> crate::preview2::bindings::sockets::tcp_create_socket::Host for T {
    fn create_tcp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<TcpSocketWrapper>> {
        let socket = self
            .ctx_mut()
            .network
            .new_tcp_socket(address_family.into())?;
        let wrapper = TcpSocketWrapper {
            inner: socket,
            tcp_state: TcpState::Default,
        };
        let socket = self.table_mut().push(wrapper)?;
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

        // At the moment, there's only one network handle (`instance-network`)
        // in existence. All we have to do here is validate that the caller indeed
        // has possesion of a valid handle and then we're good to go:
        let _network = table.get(&network)?;
        let socket = table.get_mut(&this)?;
        let local_address: SocketAddr = local_address.into();

        match socket.tcp_state {
            TcpState::Default => {}
            TcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        socket.inner.bind(local_address)?;
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
        let table = self.table_mut();

        // At the moment, there's only one network handle (`instance-network`)
        // in existence. All we have to do here is validate that the caller indeed
        // has possesion of a valid handle and then we're good to go:
        let _network = table.get(&network)?;
        let socket = table.get_mut(&this)?;
        let remote_address: SocketAddr = remote_address.into();

        match socket.tcp_state {
            TcpState::Default => {}
            TcpState::Connecting { .. } => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        let mut future = Preview2Future::new(socket.inner.connect(remote_address));

        // Attempt to return (validation) errors immediately:
        let future = match future.try_resolve() {
            Some(Err(e)) => return Err(e.into()),
            Some(Ok(r)) => Preview2Future::done(Ok(r)),
            None => future,
        };

        socket.tcp_state = TcpState::Connecting { future };
        Ok(())
    }

    fn finish_connect(
        &mut self,
        this: Resource<TcpSocketWrapper>,
    ) -> SocketResult<(Resource<InputStream>, Resource<OutputStream>)> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        let TcpState::Connecting { future } = &mut socket.tcp_state else {
            return Err(ErrorCode::NotInProgress.into());
        };

        match future.try_resolve() {
            Some(Ok((reader, writer))) => {
                socket.tcp_state = TcpState::Connected;

                let input = TcpSocketWrapper::new_input_stream(reader);
                let output = TcpSocketWrapper::new_output_stream(writer);

                let input_stream = self.table_mut().push_child(input, &this)?;
                let output_stream = self.table_mut().push_child(output, &this)?;

                Ok((input_stream, output_stream))
            }
            Some(Err(e)) => {
                socket.tcp_state = TcpState::Default;
                Err(e.into())
            }
            None => Err(ErrorCode::WouldBlock.into()),
        }
    }

    fn start_listen(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<()> {
        self.ctx().allowed_network_uses.check_allowed_tcp()?;
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        match socket.tcp_state {
            TcpState::Bound => {}
            TcpState::Default | TcpState::Connected | TcpState::Listening { .. } => {
                return Err(ErrorCode::InvalidState.into())
            }
            TcpState::ListenStarted | TcpState::Connecting { .. } | TcpState::BindStarted => {
                return Err(ErrorCode::ConcurrencyConflict.into())
            }
        }

        socket.inner.listen()?;
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

        socket.tcp_state = TcpState::Listening {
            pending_result: None,
        };

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
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        let TcpState::Listening { pending_result } = &mut socket.tcp_state else {
            return Err(ErrorCode::InvalidState.into());
        };

        let (client, reader, writer) = match pending_result.take() {
            Some(Ok(client)) => client,
            Some(Err(e)) => return Err(e.into()),
            None => {
                let mut cx = Context::from_waker(futures::task::noop_waker_ref());
                match socket.inner.poll_accept(&mut cx) {
                    Poll::Ready(Ok(client)) => client,
                    Poll::Ready(Err(e)) => return Err(e.into()),
                    Poll::Pending => return Err(ErrorCode::WouldBlock.into()),
                }
            }
        };

        let tcp_socket = TcpSocketWrapper {
            inner: client,
            tcp_state: TcpState::Connected,
        };

        let input = TcpSocketWrapper::new_input_stream(reader);
        let output = TcpSocketWrapper::new_output_stream(writer);

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

        Ok(socket.inner.local_address()?.into())
    }

    fn remote_address(
        &mut self,
        this: Resource<TcpSocketWrapper>,
    ) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.tcp_state {
            TcpState::Connected => {}
            TcpState::Connecting { .. } => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        Ok(socket.inner.remote_address()?.into())
    }

    fn is_listening(&mut self, this: Resource<TcpSocketWrapper>) -> Result<bool, anyhow::Error> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.tcp_state {
            TcpState::Listening { .. } => Ok(true),
            _ => Ok(false),
        }
    }

    fn address_family(
        &mut self,
        this: Resource<TcpSocketWrapper>,
    ) -> Result<IpAddressFamily, anyhow::Error> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(socket.inner.address_family().into())
    }

    fn ipv6_only(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<bool> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(socket.inner.ipv6_only()?)
    }

    fn set_ipv6_only(&mut self, this: Resource<TcpSocketWrapper>, value: bool) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        match socket.tcp_state {
            TcpState::Default => {}
            TcpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        socket.inner.set_ipv6_only(value)?;
        Ok(())
    }

    fn set_listen_backlog_size(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);
        Ok(socket.inner.set_listen_backlog_size(value)?)
    }

    fn keep_alive_enabled(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<bool> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(socket.inner.keep_alive_enabled()?)
    }

    fn set_keep_alive_enabled(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: bool,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;
        Ok(socket.inner.set_keep_alive_enabled(value)?)
    }

    fn keep_alive_idle_time(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;
        let duration = socket.inner.keep_alive_idle_time()?;
        Ok(duration.as_nanos() as u64)
    }

    fn set_keep_alive_idle_time(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;
        let duration = Duration::from_nanos(value);
        Ok(socket.inner.set_keep_alive_idle_time(duration)?)
    }

    fn keep_alive_interval(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;
        let duration = socket.inner.keep_alive_interval()?;
        Ok(duration.as_nanos() as u64)
    }

    fn set_keep_alive_interval(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;
        let duration = Duration::from_nanos(value);
        Ok(socket.inner.set_keep_alive_interval(duration)?)
    }

    fn keep_alive_count(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<u32> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(socket.inner.keep_alive_count()?)
    }

    fn set_keep_alive_count(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: u32,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;
        Ok(socket.inner.set_keep_alive_count(value)?)
    }

    fn hop_limit(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<u8> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(socket.inner.hop_limit()?)
    }

    fn set_hop_limit(&mut self, this: Resource<TcpSocketWrapper>, value: u8) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;
        Ok(socket.inner.set_hop_limit(value)?)
    }

    fn receive_buffer_size(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(socket.inner.receive_buffer_size()?.try_into().unwrap())
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);
        Ok(socket.inner.set_receive_buffer_size(value)?)
    }

    fn send_buffer_size(&mut self, this: Resource<TcpSocketWrapper>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;
        Ok(socket.inner.send_buffer_size()?.try_into().unwrap())
    }

    fn set_send_buffer_size(
        &mut self,
        this: Resource<TcpSocketWrapper>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);
        Ok(socket.inner.set_send_buffer_size(value)?)
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
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        match socket.tcp_state {
            TcpState::Connected => {}
            TcpState::Connecting { .. } => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        socket.inner.shutdown(match shutdown_type {
            ShutdownType::Receive => std::net::Shutdown::Read,
            ShutdownType::Send => std::net::Shutdown::Write,
            ShutdownType::Both => std::net::Shutdown::Both,
        })?;
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
        match &mut self.tcp_state {
            TcpState::Default
            | TcpState::BindStarted
            | TcpState::Bound
            | TcpState::ListenStarted
            | TcpState::Connected => {
                // No async operation in progress.
            }
            TcpState::Connecting { future } => future.ready().await,
            TcpState::Listening { pending_result } => match pending_result {
                Some(_) => {}
                None => {
                    let result = futures::future::poll_fn(|cx| self.inner.poll_accept(cx)).await;
                    *pending_result = Some(result);
                }
            },
        }
    }
}

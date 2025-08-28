use super::is_addr_allowed;
use crate::p3::bindings::sockets::types::{
    Duration, ErrorCode, HostTcpSocket, HostTcpSocketWithStore, IpAddressFamily, IpSocketAddress,
    TcpSocket,
};
use crate::p3::sockets::{SocketError, SocketResult, WasiSockets};
use crate::p3::{
    DEFAULT_BUFFER_CAPACITY, FutureOneshotProducer, FutureReadyProducer, StreamEmptyProducer,
    write_buffered_bytes,
};
use crate::sockets::{NonInheritedOptions, SocketAddrUse, SocketAddressFamily, WasiSocketsCtxView};
use anyhow::Context;
use bytes::BytesMut;
use io_lifetimes::AsSocketlike as _;
use std::io::Cursor;
use std::net::{Shutdown, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use wasmtime::AsContextMut as _;
use wasmtime::component::{
    Accessor, Destination, FutureReader, Resource, ResourceTable, Source, StreamConsumer,
    StreamProducer, StreamReader, StreamState,
};

fn get_socket<'a>(
    table: &'a ResourceTable,
    socket: &'a Resource<TcpSocket>,
) -> SocketResult<&'a TcpSocket> {
    table
        .get(socket)
        .context("failed to get socket resource from table")
        .map_err(SocketError::trap)
}

fn get_socket_mut<'a>(
    table: &'a mut ResourceTable,
    socket: &'a Resource<TcpSocket>,
) -> SocketResult<&'a mut TcpSocket> {
    table
        .get_mut(socket)
        .context("failed to get socket resource from table")
        .map_err(SocketError::trap)
}

struct ListenStreamProducer<T> {
    accepted: Option<std::io::Result<TcpStream>>,
    listener: Arc<TcpListener>,
    family: SocketAddressFamily,
    options: NonInheritedOptions,
    getter: for<'a> fn(&'a mut T) -> WasiSocketsCtxView<'a>,
}

impl<T> ListenStreamProducer<T> {
    async fn accept(&mut self) -> std::io::Result<TcpStream> {
        if let Some(res) = self.accepted.take() {
            return res;
        }
        let (stream, _) = self.listener.accept().await?;
        Ok(stream)
    }
}

impl<D> StreamProducer<D, Resource<TcpSocket>> for ListenStreamProducer<D>
where
    D: 'static,
{
    async fn produce(
        &mut self,
        store: &Accessor<D>,
        dst: &mut Destination<Resource<TcpSocket>>,
    ) -> wasmtime::Result<StreamState> {
        let res = self.accept().await;
        let socket = TcpSocket::new_accept(res, &self.options, self.family)
            .unwrap_or_else(|err| TcpSocket::new_error(err, self.family));
        let store = store.with_getter::<WasiSockets>(self.getter);
        let socket = store.with(|mut store| {
            store
                .get()
                .table
                .push(socket)
                .context("failed to push socket resource to table")
        })?;
        // FIXME: Handle cancellation
        if let Some(socket) = dst.write(&store, Some(socket)).await? {
            store.with(|mut store| {
                store
                    .get()
                    .table
                    .delete(socket)
                    .context("failed to delete socket resource from table")
            })?;
            return Ok(StreamState::Closed);
        }
        Ok(StreamState::Open)
    }

    async fn when_ready(&mut self, _: &Accessor<D>) -> wasmtime::Result<StreamState> {
        if self.accepted.is_none() {
            let res = self.accept().await;
            self.accepted = Some(res);
        }
        Ok(StreamState::Open)
    }
}

struct ReceiveStreamProducer {
    stream: Arc<TcpStream>,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
    buffer: Cursor<BytesMut>,
}

impl Drop for ReceiveStreamProducer {
    fn drop(&mut self) {
        self.close(Ok(()))
    }
}

impl ReceiveStreamProducer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let Some(tx) = self.result.take() {
            _ = self
                .stream
                .as_socketlike_view::<std::net::TcpStream>()
                .shutdown(Shutdown::Read);
            _ = tx.send(res);
        }
    }
}

impl<D> StreamProducer<D, u8> for ReceiveStreamProducer {
    async fn produce(
        &mut self,
        store: &Accessor<D>,
        dst: &mut Destination<u8>,
    ) -> wasmtime::Result<StreamState> {
        if !self.buffer.get_ref().is_empty() {
            write_buffered_bytes(store, &mut self.buffer, dst).await?;
            return Ok(StreamState::Open);
        }

        let res = 'result: loop {
            match store.with(|mut store| {
                if let Some(mut dst) = dst.as_guest_destination(store.as_context_mut()) {
                    let n = self.stream.try_read(dst.remaining())?;
                    if n > 0 {
                        dst.mark_written(n);
                    }
                    Ok(n)
                } else {
                    self.buffer.get_mut().reserve(DEFAULT_BUFFER_CAPACITY);
                    self.stream.try_read_buf(self.buffer.get_mut())
                }
            }) {
                Ok(0) => break 'result Ok(()),
                Ok(..) => {
                    if !self.buffer.get_ref().is_empty() {
                        // FIXME: `mem::take` rather than `clone` when we can ensure cancellation-safety
                        //let buf = mem::take(&mut self.buffer);
                        let buf = self.buffer.clone();
                        self.buffer = dst.write(store, buf).await?;
                        if self.buffer.position() as usize == self.buffer.get_ref().len() {
                            self.buffer.get_mut().clear();
                            self.buffer.set_position(0);
                        }
                    }
                    return Ok(StreamState::Open);
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    if let Err(err) = self.stream.readable().await {
                        break 'result Err(err.into());
                    }
                }
                Err(err) => break 'result Err(err.into()),
            }
        };
        self.close(res);
        Ok(StreamState::Closed)
    }

    async fn when_ready(&mut self, _: &Accessor<D>) -> wasmtime::Result<StreamState> {
        if self.buffer.get_ref().is_empty() {
            if let Err(err) = self.stream.readable().await {
                self.close(Err(err.into()));
                return Ok(StreamState::Closed);
            }
        }
        Ok(StreamState::Open)
    }
}

struct SendStreamConsumer {
    stream: Arc<TcpStream>,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
    buffer: BytesMut,
}

impl Drop for SendStreamConsumer {
    fn drop(&mut self) {
        self.close(Ok(()))
    }
}

impl SendStreamConsumer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let Some(tx) = self.result.take() {
            _ = self
                .stream
                .as_socketlike_view::<std::net::TcpStream>()
                .shutdown(Shutdown::Write);
            _ = tx.send(res);
        }
    }
}

impl<D> StreamConsumer<D, u8> for SendStreamConsumer {
    async fn consume(
        &mut self,
        store: &Accessor<D>,
        src: &mut Source<'_, u8>,
    ) -> wasmtime::Result<StreamState> {
        let res = 'result: loop {
            match store.with(|mut store| {
                let n = if let Some(mut src) = src.as_guest_source(store.as_context_mut()) {
                    let n = self.stream.try_write(src.remaining())?;
                    src.mark_read(n);
                    n
                } else {
                    // NOTE: The implementation might want to use Linux SIOCOUTQ ioctl or similar construct
                    // on other platforms to only read `min(socket_capacity, src.remaining())` and prevent
                    // short writes
                    self.buffer.reserve(src.remaining(&mut store));
                    if let Err(err) = src.read(&mut store, &mut self.buffer) {
                        return Ok(Err(err));
                    }
                    self.stream.try_write(&self.buffer)?
                };
                debug_assert!(n > 0);
                std::io::Result::Ok(Ok(n))
            }) {
                Ok(Ok(..)) if self.buffer.is_empty() => return Ok(StreamState::Open),
                Ok(Ok(n)) => {
                    let mut buf = &self.buffer[n..];
                    while !buf.is_empty() {
                        // FIXME: Handle cancellation
                        if let Err(err) = self.stream.writable().await {
                            break 'result Err(err.into());
                        }
                        match self.stream.try_write(buf) {
                            Ok(n) => buf = &buf[n..],
                            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => continue,
                            Err(err) => break 'result Err(err.into()),
                        }
                    }
                    self.buffer.clear();
                }
                Ok(Err(err)) => return Err(err),
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    if let Err(err) = self.stream.writable().await {
                        break 'result Err(err.into());
                    }
                }
                Err(err) => break 'result Err(err.into()),
            }
        };
        self.close(res);
        Ok(StreamState::Closed)
    }

    async fn when_ready(&mut self, _: &Accessor<D>) -> wasmtime::Result<StreamState> {
        if let Err(err) = self.stream.writable().await {
            self.close(Err(err.into()));
            return Ok(StreamState::Closed);
        }
        Ok(StreamState::Open)
    }
}

impl HostTcpSocketWithStore for WasiSockets {
    async fn bind<T>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let local_address = SocketAddr::from(local_address);
        if !is_addr_allowed(store, local_address, SocketAddrUse::TcpBind).await {
            return Err(ErrorCode::AccessDenied.into());
        }
        store.with(|mut store| {
            let socket = get_socket_mut(store.get().table, &socket)?;
            socket.start_bind(local_address)?;
            socket.finish_bind()?;
            Ok(())
        })
    }

    async fn connect<T>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
        remote_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let remote_address = SocketAddr::from(remote_address);
        if !is_addr_allowed(store, remote_address, SocketAddrUse::TcpConnect).await {
            return Err(ErrorCode::AccessDenied.into());
        }
        let sock = store.with(|mut store| {
            let socket = get_socket_mut(store.get().table, &socket)?;
            let socket = socket.start_connect(&remote_address)?;
            SocketResult::Ok(socket)
        })?;

        // FIXME: handle possible cancellation of the outer `connect`
        // https://github.com/bytecodealliance/wasmtime/pull/11291#discussion_r2223917986
        let res = sock.connect(remote_address).await;
        store.with(|mut store| {
            let socket = get_socket_mut(store.get().table, &socket)?;
            socket.finish_connect(res)?;
            Ok(())
        })
    }

    async fn listen<T: 'static>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
    ) -> SocketResult<StreamReader<Resource<TcpSocket>>> {
        let instance = store.instance();
        let getter = store.getter();
        store.with(|mut store| {
            let socket = get_socket_mut(store.get().table, &socket)?;
            socket.start_listen()?;
            socket.finish_listen()?;
            let listener = socket.tcp_listener_arc().unwrap().clone();
            let family = socket.address_family();
            let options = socket.non_inherited_options().clone();
            Ok(StreamReader::new(
                instance,
                &mut store,
                ListenStreamProducer {
                    accepted: None,
                    listener,
                    family,
                    options,
                    getter,
                },
            ))
        })
    }

    async fn send<T: 'static>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
        data: StreamReader<u8>,
    ) -> SocketResult<()> {
        let (result_tx, result_rx) = oneshot::channel();
        store.with(|mut store| {
            let sock = get_socket(store.get().table, &socket)?;
            let stream = sock.tcp_stream_arc()?;
            let stream = Arc::clone(stream);
            data.pipe(
                store,
                SendStreamConsumer {
                    stream,
                    result: Some(result_tx),
                    buffer: BytesMut::default(),
                },
            );
            SocketResult::Ok(())
        })?;
        result_rx
            .await
            .context("oneshot sender dropped")
            .map_err(SocketError::trap)??;
        Ok(())
    }

    async fn receive<T: 'static>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
        let instance = store.instance();
        store.with(|mut store| {
            let socket = get_socket_mut(store.get().table, &socket)?;
            match socket.start_receive() {
                Some(stream) => {
                    let stream = Arc::clone(stream);
                    let (result_tx, result_rx) = oneshot::channel();
                    Ok((
                        StreamReader::new(
                            instance,
                            &mut store,
                            ReceiveStreamProducer {
                                stream,
                                result: Some(result_tx),
                                buffer: Cursor::default(),
                            },
                        ),
                        FutureReader::new(instance, &mut store, FutureOneshotProducer(result_rx)),
                    ))
                }
                None => Ok((
                    StreamReader::new(instance, &mut store, StreamEmptyProducer),
                    FutureReader::new(
                        instance,
                        &mut store,
                        FutureReadyProducer(Err(ErrorCode::InvalidState)),
                    ),
                )),
            }
        })
    }
}

impl HostTcpSocket for WasiSocketsCtxView<'_> {
    fn create(&mut self, address_family: IpAddressFamily) -> SocketResult<Resource<TcpSocket>> {
        let family = address_family.into();
        let socket = TcpSocket::new(self.ctx, family)?;
        let resource = self
            .table
            .push(socket)
            .context("failed to push socket resource to table")
            .map_err(SocketError::trap)?;
        Ok(resource)
    }

    fn get_local_address(&mut self, socket: Resource<TcpSocket>) -> SocketResult<IpSocketAddress> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.local_address()?.into())
    }

    fn get_remote_address(&mut self, socket: Resource<TcpSocket>) -> SocketResult<IpSocketAddress> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.remote_address()?.into())
    }

    fn get_is_listening(&mut self, socket: Resource<TcpSocket>) -> wasmtime::Result<bool> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.is_listening())
    }

    fn get_address_family(
        &mut self,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<IpAddressFamily> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.address_family().into())
    }

    fn set_listen_backlog_size(
        &mut self,
        socket: Resource<TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let sock = get_socket_mut(self.table, &socket)?;
        sock.set_listen_backlog_size(value)?;
        Ok(())
    }

    fn get_keep_alive_enabled(&mut self, socket: Resource<TcpSocket>) -> SocketResult<bool> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.keep_alive_enabled()?)
    }

    fn set_keep_alive_enabled(
        &mut self,
        socket: Resource<TcpSocket>,
        value: bool,
    ) -> SocketResult<()> {
        let sock = get_socket(self.table, &socket)?;
        sock.set_keep_alive_enabled(value)?;
        Ok(())
    }

    fn get_keep_alive_idle_time(&mut self, socket: Resource<TcpSocket>) -> SocketResult<Duration> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.keep_alive_idle_time()?)
    }

    fn set_keep_alive_idle_time(
        &mut self,
        socket: Resource<TcpSocket>,
        value: Duration,
    ) -> SocketResult<()> {
        let sock = get_socket_mut(self.table, &socket)?;
        sock.set_keep_alive_idle_time(value)?;
        Ok(())
    }

    fn get_keep_alive_interval(&mut self, socket: Resource<TcpSocket>) -> SocketResult<Duration> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.keep_alive_interval()?)
    }

    fn set_keep_alive_interval(
        &mut self,
        socket: Resource<TcpSocket>,
        value: Duration,
    ) -> SocketResult<()> {
        let sock = get_socket(self.table, &socket)?;
        sock.set_keep_alive_interval(value)?;
        Ok(())
    }

    fn get_keep_alive_count(&mut self, socket: Resource<TcpSocket>) -> SocketResult<u32> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.keep_alive_count()?)
    }

    fn set_keep_alive_count(
        &mut self,
        socket: Resource<TcpSocket>,
        value: u32,
    ) -> SocketResult<()> {
        let sock = get_socket(self.table, &socket)?;
        sock.set_keep_alive_count(value)?;
        Ok(())
    }

    fn get_hop_limit(&mut self, socket: Resource<TcpSocket>) -> SocketResult<u8> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.hop_limit()?)
    }

    fn set_hop_limit(&mut self, socket: Resource<TcpSocket>, value: u8) -> SocketResult<()> {
        let sock = get_socket_mut(self.table, &socket)?;
        sock.set_hop_limit(value)?;
        Ok(())
    }

    fn get_receive_buffer_size(&mut self, socket: Resource<TcpSocket>) -> SocketResult<u64> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.receive_buffer_size()?)
    }

    fn set_receive_buffer_size(
        &mut self,
        socket: Resource<TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let sock = get_socket_mut(self.table, &socket)?;
        sock.set_receive_buffer_size(value)?;
        Ok(())
    }

    fn get_send_buffer_size(&mut self, socket: Resource<TcpSocket>) -> SocketResult<u64> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.send_buffer_size()?)
    }

    fn set_send_buffer_size(
        &mut self,
        socket: Resource<TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let sock = get_socket_mut(self.table, &socket)?;
        sock.set_send_buffer_size(value)?;
        Ok(())
    }

    fn drop(&mut self, sock: Resource<TcpSocket>) -> wasmtime::Result<()> {
        self.table
            .delete(sock)
            .context("failed to delete socket resource from table")?;
        Ok(())
    }
}

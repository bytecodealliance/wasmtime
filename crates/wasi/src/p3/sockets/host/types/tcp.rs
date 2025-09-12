use super::is_addr_allowed;
use crate::p3::DEFAULT_BUFFER_CAPACITY;
use crate::p3::bindings::sockets::types::{
    Duration, ErrorCode, HostTcpSocket, HostTcpSocketWithStore, IpAddressFamily, IpSocketAddress,
    TcpSocket,
};
use crate::p3::sockets::{SocketError, SocketResult, WasiSockets};
use crate::sockets::{NonInheritedOptions, SocketAddrUse, SocketAddressFamily, WasiSocketsCtxView};
use anyhow::Context as _;
use bytes::BytesMut;
use core::iter;
use core::pin::Pin;
use core::task::{Context, Poll};
use io_lifetimes::AsSocketlike as _;
use std::io::Cursor;
use std::net::{Shutdown, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use wasmtime::component::{
    Accessor, Destination, FutureReader, Resource, ResourceTable, Source, StreamConsumer,
    StreamProducer, StreamReader, StreamResult,
};
use wasmtime::{AsContextMut as _, StoreContextMut};

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
    listener: Arc<TcpListener>,
    family: SocketAddressFamily,
    options: NonInheritedOptions,
    getter: for<'a> fn(&'a mut T) -> WasiSocketsCtxView<'a>,
}

impl<D> StreamProducer<D> for ListenStreamProducer<D>
where
    D: 'static,
{
    type Item = Resource<TcpSocket>;
    type Buffer = Option<Self::Item>;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'a, D>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        // If the destination buffer is empty then this is a request on
        // behalf of the guest to wait for this socket to be ready to accept
        // without actually accepting something. The `TcpListener` in Tokio does
        // not have this capability so we're forced to lie here and say instead
        // "yes we're ready to accept" as a fallback.
        //
        // See WebAssembly/component-model#561 for some more information.
        if dst.remaining(&mut store) == Some(0) {
            return Poll::Ready(Ok(StreamResult::Completed));
        }
        let res = match self.listener.poll_accept(cx) {
            Poll::Ready(res) => res.map(|(stream, _)| stream),
            Poll::Pending if finish => return Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => return Poll::Pending,
        };
        let socket = TcpSocket::new_accept(res, &self.options, self.family)
            .unwrap_or_else(|err| TcpSocket::new_error(err, self.family));
        let WasiSocketsCtxView { table, .. } = (self.getter)(store.data_mut());
        let socket = table
            .push(socket)
            .context("failed to push socket resource to table")?;
        dst.set_buffer(Some(socket));
        Poll::Ready(Ok(StreamResult::Completed))
    }
}

struct ReceiveStreamProducer {
    stream: Arc<TcpStream>,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
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

impl<D> StreamProducer<D> for ReceiveStreamProducer {
    type Item = u8;
    type Buffer = Cursor<BytesMut>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'a, D>,
        dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let res = 'result: {
            // 0-length reads are an indication that we should wait for
            // readiness here, so use `poll_read_ready`.
            if dst.remaining(store.as_context_mut()) == Some(0) {
                return match self.stream.poll_read_ready(cx) {
                    Poll::Ready(Ok(())) => Poll::Ready(Ok(StreamResult::Completed)),
                    Poll::Ready(Err(err)) => break 'result Err(err.into()),
                    Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
                    Poll::Pending => Poll::Pending,
                };
            }

            let mut dst = dst.as_direct(store, DEFAULT_BUFFER_CAPACITY);
            let buf = dst.remaining();
            loop {
                match self.stream.try_read(buf) {
                    Ok(0) => break 'result Ok(()),
                    Ok(n) => {
                        dst.mark_written(n);
                        return Poll::Ready(Ok(StreamResult::Completed));
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        match self.stream.poll_read_ready(cx) {
                            Poll::Ready(Ok(())) => continue,
                            Poll::Ready(Err(err)) => break 'result Err(err.into()),
                            Poll::Pending if finish => {
                                return Poll::Ready(Ok(StreamResult::Cancelled));
                            }
                            Poll::Pending => return Poll::Pending,
                        }
                    }
                    Err(err) => break 'result Err(err.into()),
                }
            }
        };
        self.close(res);
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

struct SendStreamConsumer {
    stream: Arc<TcpStream>,
    result: Option<oneshot::Sender<Result<(), ErrorCode>>>,
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

impl<D> StreamConsumer<D> for SendStreamConsumer {
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let mut src = src.as_direct(store);
        let res = 'result: {
            // A 0-length write is a request to wait for readiness so use
            // `poll_write_ready` to wait for the underlying object to be ready.
            if src.remaining().is_empty() {
                return match self.stream.poll_write_ready(cx) {
                    Poll::Ready(Ok(())) => Poll::Ready(Ok(StreamResult::Completed)),
                    Poll::Ready(Err(err)) => break 'result Err(err.into()),
                    Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
                    Poll::Pending => Poll::Pending,
                };
            }
            loop {
                match self.stream.try_write(src.remaining()) {
                    Ok(n) => {
                        debug_assert!(n > 0);
                        src.mark_read(n);
                        return Poll::Ready(Ok(StreamResult::Completed));
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        match self.stream.poll_write_ready(cx) {
                            Poll::Ready(Ok(())) => continue,
                            Poll::Ready(Err(err)) => break 'result Err(err.into()),
                            Poll::Pending if finish => {
                                return Poll::Ready(Ok(StreamResult::Cancelled));
                            }
                            Poll::Pending => return Poll::Pending,
                        }
                    }
                    Err(err) => break 'result Err(err.into()),
                }
            }
        };
        self.close(res);
        Poll::Ready(Ok(StreamResult::Dropped))
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
                            },
                        ),
                        FutureReader::new(instance, &mut store, result_rx),
                    ))
                }
                None => Ok((
                    StreamReader::new(instance, &mut store, iter::empty()),
                    FutureReader::new(instance, &mut store, async {
                        anyhow::Ok(Err(ErrorCode::InvalidState))
                    }),
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

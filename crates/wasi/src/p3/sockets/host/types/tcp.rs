use crate::p3::DEFAULT_BUFFER_CAPACITY;
use crate::p3::bindings::sockets::types::{
    Duration, ErrorCode, HostTcpSocket, HostTcpSocketWithStore, IpAddressFamily, IpSocketAddress,
    TcpSocket,
};
use crate::p3::sockets::{SocketError, SocketResult, WasiSockets};
use crate::sockets::{TcpListenStream, TcpReceiveStream, TcpSendStream, WasiSocketsCtxView};
use bytes::BytesMut;
use core::iter;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::net::SocketAddr;
use tokio::sync::oneshot;
use wasmtime::component::{
    Access, Accessor, Destination, FutureReader, Resource, ResourceTable, Source, StreamConsumer,
    StreamProducer, StreamReader, StreamResult,
};
use wasmtime::error::Context as _;
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
    listener: TcpListenStream,
    getter: for<'a> fn(&'a mut T) -> WasiSocketsCtxView<'a>,
}

impl<D> StreamProducer<D> for ListenStreamProducer<D>
where
    D: 'static,
{
    type Item = Resource<TcpSocket>;
    type Buffer = Option<Self::Item>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'a, D>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        // If the destination buffer is empty then this is a readiness check.
        if dst.remaining(&mut store) == Some(0) {
            return match self.listener.poll_ready(cx) {
                Poll::Ready(()) => Poll::Ready(Ok(StreamResult::Completed)),
                Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
                Poll::Pending => Poll::Pending,
            };
        }

        let socket = match self.listener.poll_accept(cx) {
            Poll::Ready(socket) => socket,
            Poll::Pending if finish => return Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => return Poll::Pending,
        };
        let ctx = (self.getter)(store.data_mut());
        let socket = ctx
            .table
            .push(socket)
            .context("failed to push socket resource to table")?;
        dst.set_buffer(Some(socket));
        Poll::Ready(Ok(StreamResult::Completed))
    }
}

struct ReceiveStreamProducer(Option<(TcpReceiveStream, oneshot::Sender<Result<(), ErrorCode>>)>);

impl Drop for ReceiveStreamProducer {
    fn drop(&mut self) {
        self.close(Ok(()))
    }
}

impl ReceiveStreamProducer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let Some((_, tx)) = self.0.take() {
            _ = tx.send(res);
        }
    }
}

impl<D> StreamProducer<D> for ReceiveStreamProducer {
    type Item = u8;
    type Buffer = BytesMut;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'a, D>,
        dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let Some((stream, _)) = self.0.as_mut() else {
            return Poll::Ready(Ok(StreamResult::Dropped));
        };

        // 0-length read is a readiness check.
        if dst.remaining(store.as_context_mut()) == Some(0) {
            return match stream.poll_ready(cx) {
                Poll::Ready(()) => Poll::Ready(Ok(StreamResult::Completed)),
                Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
                Poll::Pending => Poll::Pending,
            };
        }

        let mut dst = dst.as_direct(store, DEFAULT_BUFFER_CAPACITY);
        let buf = dst.remaining();
        match stream.poll_read(cx, buf) {
            Poll::Ready(Ok(0)) => {
                self.close(Ok(()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Ready(Ok(n)) => {
                dst.mark_written(n);
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Poll::Ready(Err(err)) => {
                self.close(Err(err.into()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

struct SendStreamConsumer(Option<(TcpSendStream, oneshot::Sender<Result<(), ErrorCode>>)>);

impl Drop for SendStreamConsumer {
    fn drop(&mut self) {
        self.close(Ok(()))
    }
}

impl SendStreamConsumer {
    fn close(&mut self, res: Result<(), ErrorCode>) {
        if let Some((_, tx)) = self.0.take() {
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
        let Some((stream, _)) = self.0.as_mut() else {
            return Poll::Ready(Ok(StreamResult::Dropped));
        };

        let mut src = src.as_direct(store);

        // A 0-length write is a readiness check.
        if src.remaining().is_empty() {
            return match stream.poll_ready(cx) {
                Poll::Ready(()) => Poll::Ready(Ok(StreamResult::Completed)),
                Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
                Poll::Pending => Poll::Pending,
            };
        }

        match stream.poll_write(cx, src.remaining()) {
            Poll::Ready(Ok(n)) => {
                debug_assert!(n > 0);
                src.mark_read(n);
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Poll::Ready(Err(err)) => {
                self.close(Err(err.into()));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T: Send> HostTcpSocketWithStore<T> for WasiSockets {
    async fn connect(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
        remote_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let remote_address = SocketAddr::from(remote_address);

        store.with(|mut store| {
            let socket = get_socket_mut(store.get().table, &socket)?;
            let socket = socket.start_connect(remote_address)?;
            SocketResult::Ok(socket)
        })?;

        std::future::poll_fn(|cx| {
            store.with(|mut store| -> Poll<SocketResult<()>> {
                let socket = get_socket_mut(store.get().table, &socket)?;
                socket.poll_finish_connect(cx).map_err(SocketError::from)
            })
        })
        .await
    }

    async fn listen(
        mut store: Access<'_, T, Self>,
        socket_resource: Resource<TcpSocket>,
    ) -> SocketResult<StreamReader<Resource<TcpSocket>>> {
        let getter = store.getter();
        let socket = get_socket_mut(store.get().table, &socket_resource)?;

        let listener = socket.listen().await?;

        let ret = StreamReader::new(&mut store, ListenStreamProducer { listener, getter })
            .map_err(SocketError::trap)?;
        Ok(ret)
    }

    fn send(
        mut store: Access<'_, T, Self>,
        socket: Resource<TcpSocket>,
        mut data: StreamReader<u8>,
    ) -> wasmtime::Result<FutureReader<Result<(), ErrorCode>>> {
        let socket = get_socket_mut(store.get().table, &socket)?;
        match socket.take_send_stream() {
            Ok(stream) => {
                let (result_tx, result_rx) = oneshot::channel();
                data.pipe(&mut store, SendStreamConsumer(Some((stream, result_tx))))?;
                FutureReader::new(&mut store, result_rx)
            }
            Err(err) => {
                data.close(&mut store)?;
                FutureReader::new(
                    &mut store,
                    async move { wasmtime::error::Ok(Err(err.into())) },
                )
            }
        }
    }

    fn receive(
        mut store: Access<T, Self>,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
        let socket = get_socket_mut(store.get().table, &socket)?;
        match socket.take_receive_stream() {
            Ok(stream) => {
                let (result_tx, result_rx) = oneshot::channel();
                Ok((
                    StreamReader::new(
                        &mut store,
                        ReceiveStreamProducer(Some((stream, result_tx))),
                    )?,
                    FutureReader::new(&mut store, result_rx)?,
                ))
            }
            Err(err) => Ok((
                StreamReader::new(&mut store, iter::empty())?,
                FutureReader::new(
                    &mut store,
                    async move { wasmtime::error::Ok(Err(err.into())) },
                )?,
            )),
        }
    }
}

impl HostTcpSocket for WasiSocketsCtxView<'_> {
    async fn bind(
        &mut self,
        socket: Resource<TcpSocket>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let local_address = SocketAddr::from(local_address);
        let socket = get_socket_mut(self.table, &socket)?;
        socket.bind(local_address).await?;
        Ok(())
    }

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
        let sock = get_socket_mut(self.table, &socket)?;
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

use super::is_addr_allowed;
use crate::TrappableError;
use crate::p3::DEFAULT_BUFFER_CAPACITY;
use crate::p3::bindings::sockets::types::{
    Duration, ErrorCode, HostTcpSocket, HostTcpSocketWithStore, IpAddressFamily, IpSocketAddress,
    TcpSocket,
};
use crate::p3::sockets::{SocketResult, WasiSockets};
use crate::sockets::{NonInheritedOptions, SocketAddrUse, SocketAddressFamily, WasiSocketsCtxView};
use anyhow::Context;
use bytes::BytesMut;
use io_lifetimes::AsSocketlike as _;
use std::future::poll_fn;
use std::io::Cursor;
use std::net::{Shutdown, SocketAddr};
use std::pin::pin;
use std::sync::Arc;
use std::task::Poll;
use tokio::net::{TcpListener, TcpStream};
use wasmtime::component::{
    Accessor, AccessorTask, FutureReader, FutureWriter, GuardedFutureWriter, GuardedStreamWriter,
    Resource, ResourceTable, StreamReader, StreamWriter,
};

fn get_socket<'a>(
    table: &'a ResourceTable,
    socket: &'a Resource<TcpSocket>,
) -> SocketResult<&'a TcpSocket> {
    table
        .get(socket)
        .context("failed to get socket resource from table")
        .map_err(TrappableError::trap)
}

fn get_socket_mut<'a>(
    table: &'a mut ResourceTable,
    socket: &'a Resource<TcpSocket>,
) -> SocketResult<&'a mut TcpSocket> {
    table
        .get_mut(socket)
        .context("failed to get socket resource from table")
        .map_err(TrappableError::trap)
}

struct ListenTask {
    listener: Arc<TcpListener>,
    family: SocketAddressFamily,
    tx: StreamWriter<Resource<TcpSocket>>,
    options: NonInheritedOptions,
}

impl<T> AccessorTask<T, WasiSockets, wasmtime::Result<()>> for ListenTask {
    async fn run(self, store: &Accessor<T, WasiSockets>) -> wasmtime::Result<()> {
        let mut tx = GuardedStreamWriter::new(store, self.tx);
        while !tx.is_closed() {
            let Some(res) = ({
                let mut accept = pin!(self.listener.accept());
                let mut tx = pin!(tx.watch_reader());
                poll_fn(|cx| match tx.as_mut().poll(cx) {
                    Poll::Ready(()) => return Poll::Ready(None),
                    Poll::Pending => accept.as_mut().poll(cx).map(Some),
                })
                .await
            }) else {
                return Ok(());
            };
            let socket = TcpSocket::new_accept(res.map(|p| p.0), &self.options, self.family)
                .unwrap_or_else(|err| TcpSocket::new_error(err, self.family));
            let socket = store.with(|mut view| {
                view.get()
                    .table
                    .push(socket)
                    .context("failed to push socket resource to table")
            })?;
            if let Some(socket) = tx.write(Some(socket)).await {
                debug_assert!(tx.is_closed());
                store.with(|mut view| {
                    view.get()
                        .table
                        .delete(socket)
                        .context("failed to delete socket resource from table")
                })?;
                return Ok(());
            }
        }
        Ok(())
    }
}

struct ReceiveTask {
    stream: Arc<TcpStream>,
    data_tx: StreamWriter<u8>,
    result_tx: FutureWriter<Result<(), ErrorCode>>,
}

impl<T> AccessorTask<T, WasiSockets, wasmtime::Result<()>> for ReceiveTask {
    async fn run(self, store: &Accessor<T, WasiSockets>) -> wasmtime::Result<()> {
        let mut buf = BytesMut::with_capacity(DEFAULT_BUFFER_CAPACITY);
        let mut data_tx = GuardedStreamWriter::new(store, self.data_tx);
        let result_tx = GuardedFutureWriter::new(store, self.result_tx);
        let res = loop {
            match self.stream.try_read_buf(&mut buf) {
                Ok(0) => {
                    break Ok(());
                }
                Ok(..) => {
                    buf = data_tx.write_all(Cursor::new(buf)).await.into_inner();
                    if data_tx.is_closed() {
                        break Ok(());
                    }
                    buf.clear();
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    let Some(res) = ({
                        let mut readable = pin!(self.stream.readable());
                        let mut tx = pin!(data_tx.watch_reader());
                        poll_fn(|cx| match tx.as_mut().poll(cx) {
                            Poll::Ready(()) => return Poll::Ready(None),
                            Poll::Pending => readable.as_mut().poll(cx).map(Some),
                        })
                        .await
                    }) else {
                        break Ok(());
                    };
                    if let Err(err) = res {
                        break Err(err.into());
                    }
                }
                Err(err) => {
                    break Err(err.into());
                }
            }
        };
        _ = self
            .stream
            .as_socketlike_view::<std::net::TcpStream>()
            .shutdown(Shutdown::Read);
        drop(self.stream);
        drop(data_tx);
        result_tx.write(res).await;
        Ok(())
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
        store.with(|mut view| {
            let socket = get_socket_mut(view.get().table, &socket)?;
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
        let sock = store.with(|mut view| -> SocketResult<_> {
            let socket = get_socket_mut(view.get().table, &socket)?;
            Ok(socket.start_connect(&remote_address)?)
        })?;

        // FIXME: handle possible cancellation of the outer `connect`
        // https://github.com/bytecodealliance/wasmtime/pull/11291#discussion_r2223917986
        let res = sock.connect(remote_address).await;
        store.with(|mut view| -> SocketResult<_> {
            let socket = get_socket_mut(view.get().table, &socket)?;
            socket.finish_connect(res)?;
            Ok(())
        })
    }

    async fn listen<T: 'static>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
    ) -> SocketResult<StreamReader<Resource<TcpSocket>>> {
        store.with(|mut view| {
            let socket = get_socket_mut(view.get().table, &socket)?;
            socket.start_listen()?;
            socket.finish_listen()?;
            let listener = socket.tcp_listener_arc().unwrap().clone();
            let family = socket.address_family();
            let options = socket.non_inherited_options().clone();
            let (tx, rx) = view
                .instance()
                .stream(&mut view)
                .context("failed to create stream")
                .map_err(TrappableError::trap)?;
            let task = ListenTask {
                listener,
                family,
                tx,
                options,
            };
            view.spawn(task);
            Ok(rx)
        })
    }

    async fn send<T: 'static>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
        mut data: StreamReader<u8>,
    ) -> SocketResult<()> {
        let stream = store.with(|mut view| -> SocketResult<_> {
            let sock = get_socket(view.get().table, &socket)?;
            let stream = sock.tcp_stream_arc()?;
            Ok(Arc::clone(stream))
        })?;
        let mut buf = Vec::with_capacity(DEFAULT_BUFFER_CAPACITY);
        let mut result = Ok(());
        while !data.is_closed() {
            buf = data.read(store, buf).await;
            let mut slice = buf.as_slice();
            while !slice.is_empty() {
                match stream.try_write(&slice) {
                    Ok(n) => slice = &slice[n..],
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        if let Err(err) = stream.writable().await {
                            result = Err(ErrorCode::from(err).into());
                            break;
                        }
                    }
                    Err(err) => {
                        result = Err(ErrorCode::from(err).into());
                        break;
                    }
                }
            }
            buf.clear();
        }
        _ = stream
            .as_socketlike_view::<std::net::TcpStream>()
            .shutdown(Shutdown::Write);
        result
    }

    async fn receive<T: 'static>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
        store.with(|mut view| {
            let instance = view.instance();
            let (mut data_tx, data_rx) = instance
                .stream(&mut view)
                .context("failed to create stream")?;
            let socket = get_socket_mut(view.get().table, &socket)?;
            match socket.start_receive() {
                Some(stream) => {
                    let stream = stream.clone();
                    let (result_tx, result_rx) = instance
                        .future(&mut view, || unreachable!())
                        .context("failed to create future")?;
                    view.spawn(ReceiveTask {
                        stream,
                        data_tx,
                        result_tx,
                    });
                    Ok((data_rx, result_rx))
                }
                None => {
                    let (mut result_tx, result_rx) = instance
                        .future(&mut view, || Err(ErrorCode::InvalidState))
                        .context("failed to create future")?;
                    result_tx.close(&mut view);
                    data_tx.close(&mut view);
                    Ok((data_rx, result_rx))
                }
            }
        })
    }
}

impl HostTcpSocket for WasiSocketsCtxView<'_> {
    fn new(&mut self, address_family: IpAddressFamily) -> wasmtime::Result<Resource<TcpSocket>> {
        let family = address_family.into();
        let socket =
            TcpSocket::new(self.ctx, family).unwrap_or_else(|e| TcpSocket::new_error(e, family));
        self.table
            .push(socket)
            .context("failed to push socket resource to table")
    }

    fn local_address(&mut self, socket: Resource<TcpSocket>) -> SocketResult<IpSocketAddress> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.local_address()?.into())
    }

    fn remote_address(&mut self, socket: Resource<TcpSocket>) -> SocketResult<IpSocketAddress> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.remote_address()?.into())
    }

    fn is_listening(&mut self, socket: Resource<TcpSocket>) -> wasmtime::Result<bool> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.is_listening())
    }

    fn address_family(&mut self, socket: Resource<TcpSocket>) -> wasmtime::Result<IpAddressFamily> {
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

    fn keep_alive_enabled(&mut self, socket: Resource<TcpSocket>) -> SocketResult<bool> {
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

    fn keep_alive_idle_time(&mut self, socket: Resource<TcpSocket>) -> SocketResult<Duration> {
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

    fn keep_alive_interval(&mut self, socket: Resource<TcpSocket>) -> SocketResult<Duration> {
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

    fn keep_alive_count(&mut self, socket: Resource<TcpSocket>) -> SocketResult<u32> {
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

    fn hop_limit(&mut self, socket: Resource<TcpSocket>) -> SocketResult<u8> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.hop_limit()?)
    }

    fn set_hop_limit(&mut self, socket: Resource<TcpSocket>, value: u8) -> SocketResult<()> {
        let sock = get_socket_mut(self.table, &socket)?;
        sock.set_hop_limit(value)?;
        Ok(())
    }

    fn receive_buffer_size(&mut self, socket: Resource<TcpSocket>) -> SocketResult<u64> {
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

    fn send_buffer_size(&mut self, socket: Resource<TcpSocket>) -> SocketResult<u64> {
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

use core::future::poll_fn;
use core::mem;
use core::net::SocketAddr;
use core::pin::pin;
use core::task::Poll;

use std::io::Cursor;
use std::net::Shutdown;
use std::sync::Arc;

use anyhow::{Context as _, ensure};
use bytes::BytesMut;
use io_lifetimes::AsSocketlike as _;
use rustix::io::Errno;
use tokio::net::{TcpListener, TcpStream};
use wasmtime::component::{
    Accessor, AccessorTask, FutureReader, FutureWriter, GuardedFutureWriter, GuardedStreamWriter,
    Resource, ResourceTable, StreamReader, StreamWriter,
};

use crate::p3::DEFAULT_BUFFER_CAPACITY;
use crate::p3::bindings::sockets::types::{
    Duration, ErrorCode, HostTcpSocket, HostTcpSocketWithStore, IpAddressFamily, IpSocketAddress,
    TcpSocket,
};
use crate::p3::sockets::WasiSockets;
use crate::p3::sockets::tcp::{NonInheritedOptions, TcpState};
use crate::sockets::util::{
    is_valid_address_family, is_valid_remote_address, is_valid_unicast_address,
};
use crate::sockets::{SocketAddrUse, SocketAddressFamily, WasiSocketsCtxView};

use super::is_addr_allowed;

fn is_tcp_allowed<T>(store: &Accessor<T, WasiSockets>) -> bool {
    store.with(|mut view| view.get().ctx.allowed_network_uses.tcp)
}

fn get_socket<'a>(
    table: &'a ResourceTable,
    socket: &'a Resource<TcpSocket>,
) -> wasmtime::Result<&'a TcpSocket> {
    table
        .get(socket)
        .context("failed to get socket resource from table")
}

fn get_socket_mut<'a>(
    table: &'a mut ResourceTable,
    socket: &'a Resource<TcpSocket>,
) -> wasmtime::Result<&'a mut TcpSocket> {
    table
        .get_mut(socket)
        .context("failed to get socket resource from table")
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
            let state = match res {
                Ok((stream, _addr)) => {
                    self.options.apply(self.family, &stream);
                    TcpState::Connected(Arc::new(stream))
                }
                Err(err) => {
                    match Errno::from_io_error(&err) {
                        // From: https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-accept#:~:text=WSAEINPROGRESS
                        // > WSAEINPROGRESS: A blocking Windows Sockets 1.1 call is in progress,
                        // > or the service provider is still processing a callback function.
                        //
                        // wasi-sockets doesn't have an equivalent to the EINPROGRESS error,
                        // because in POSIX this error is only returned by a non-blocking
                        // `connect` and wasi-sockets has a different solution for that.
                        #[cfg(windows)]
                        Some(Errno::INPROGRESS) => TcpState::Error(ErrorCode::Unknown),

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
                        ) => TcpState::Error(ErrorCode::ConnectionAborted),
                        _ => TcpState::Error(err.into()),
                    }
                }
            };
            let socket = store.with(|mut view| {
                view.get()
                    .table
                    .push(TcpSocket::from_state(state, self.family))
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

struct ResultWriteTask {
    result: Result<(), ErrorCode>,
    result_tx: FutureWriter<Result<(), ErrorCode>>,
}

impl<T> AccessorTask<T, WasiSockets, wasmtime::Result<()>> for ResultWriteTask {
    async fn run(self, store: &Accessor<T, WasiSockets>) -> wasmtime::Result<()> {
        GuardedFutureWriter::new(store, self.result_tx)
            .write(self.result)
            .await;
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

        // Write the result async from a separate task to ensure that all resources used by this
        // task are freed
        store.spawn(ResultWriteTask {
            result: res,
            result_tx: result_tx.into(),
        });
        Ok(())
    }
}

impl HostTcpSocketWithStore for WasiSockets {
    async fn bind<T>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
        local_address: IpSocketAddress,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let local_address = SocketAddr::from(local_address);
        if !is_tcp_allowed(store)
            || !is_addr_allowed(store, local_address, SocketAddrUse::TcpBind).await
        {
            return Ok(Err(ErrorCode::AccessDenied));
        }
        store.with(|mut view| {
            let socket = get_socket_mut(view.get().table, &socket)?;
            Ok(socket.bind(local_address))
        })
    }

    async fn connect<T>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
        remote_address: IpSocketAddress,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let remote_address = SocketAddr::from(remote_address);
        if !is_tcp_allowed(store)
            || !is_addr_allowed(store, remote_address, SocketAddrUse::TcpConnect).await
        {
            return Ok(Err(ErrorCode::AccessDenied));
        }
        match store.with(|mut view| {
            let ip = remote_address.ip();
            let socket = get_socket_mut(view.get().table, &socket)?;
            if !is_valid_unicast_address(ip)
                || !is_valid_remote_address(remote_address)
                || !is_valid_address_family(ip, socket.family)
            {
                return anyhow::Ok(Err(ErrorCode::InvalidArgument));
            }
            match mem::replace(&mut socket.tcp_state, TcpState::Connecting) {
                TcpState::Default(sock) | TcpState::Bound(sock) => Ok(Ok(sock)),
                tcp_state => {
                    socket.tcp_state = tcp_state;
                    Ok(Err(ErrorCode::InvalidState))
                }
            }
        })? {
            Ok(sock) => {
                // FIXME: handle possible cancellation of the outer `connect`
                // https://github.com/bytecodealliance/wasmtime/pull/11291#discussion_r2223917986
                let res = sock.connect(remote_address).await;
                store.with(|mut view| {
                    let socket = get_socket_mut(view.get().table, &socket)?;
                    ensure!(
                        matches!(socket.tcp_state, TcpState::Connecting),
                        "corrupted socket state"
                    );
                    match res {
                        Ok(stream) => {
                            socket.tcp_state = TcpState::Connected(Arc::new(stream));
                            Ok(Ok(()))
                        }
                        Err(err) => {
                            socket.tcp_state = TcpState::Closed;
                            Ok(Err(err.into()))
                        }
                    }
                })
            }
            Err(err) => Ok(Err(err)),
        }
    }

    async fn listen<T: 'static>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<StreamReader<Resource<TcpSocket>>, ErrorCode>> {
        store.with(|mut view| {
            if !view.get().ctx.allowed_network_uses.tcp {
                return anyhow::Ok(Err(ErrorCode::AccessDenied));
            }
            let TcpSocket {
                tcp_state,
                listen_backlog_size,
                family,
                options,
            } = get_socket_mut(view.get().table, &socket)?;
            let sock = match mem::replace(tcp_state, TcpState::Closed) {
                TcpState::Default(sock) | TcpState::Bound(sock) => sock,
                prev => {
                    *tcp_state = prev;
                    return Ok(Err(ErrorCode::InvalidState));
                }
            };
            let listener = match sock.listen(*listen_backlog_size) {
                Ok(listener) => listener,
                Err(err) => {
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
                        Some(Errno::MFILE) => return Ok(Err(ErrorCode::OutOfMemory)),

                        _ => return Ok(Err(err.into())),
                    }
                }
            };
            let listener = Arc::new(listener);
            *tcp_state = TcpState::Listening(Arc::clone(&listener));
            let family = *family;
            let options = options.clone();
            let (tx, rx) = view
                .instance()
                .stream(&mut view)
                .context("failed to create stream")?;
            let task = ListenTask {
                listener,
                family,
                tx,
                options,
            };
            view.spawn(task);
            Ok(Ok(rx))
        })
    }

    async fn send<T: 'static>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
        data: StreamReader<u8>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let (stream, mut data) = match store.with(|mut view| -> wasmtime::Result<_> {
            let sock = get_socket(view.get().table, &socket)?;
            if let TcpState::Connected(stream) | TcpState::Receiving(stream) = &sock.tcp_state {
                Ok(Ok((Arc::clone(&stream), data)))
            } else {
                Ok(Err(ErrorCode::InvalidState))
            }
        })? {
            Ok((stream, data)) => (stream, data),
            Err(err) => return Ok(Err(err)),
        };
        let mut buf = Vec::with_capacity(8096);
        let mut result = Ok(());
        while !data.is_closed() {
            buf = data.read(store, buf).await;
            let mut slice = buf.as_slice();
            while !slice.is_empty() {
                match stream.try_write(&slice) {
                    Ok(n) => slice = &slice[n..],
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        if let Err(err) = stream.writable().await {
                            result = Err(err.into());
                            break;
                        }
                    }
                    Err(err) => {
                        result = Err(err.into());
                        break;
                    }
                }
            }
            buf.clear();
        }
        _ = stream
            .as_socketlike_view::<std::net::TcpStream>()
            .shutdown(Shutdown::Write);
        Ok(result)
    }

    async fn receive<T: 'static>(
        store: &Accessor<T, Self>,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
        store.with(|mut view| {
            let instance = view.instance();
            let (data_tx, data_rx) = instance
                .stream(&mut view)
                .context("failed to create stream")?;
            let TcpSocket { tcp_state, .. } = get_socket_mut(view.get().table, &socket)?;
            match mem::replace(tcp_state, TcpState::Closed) {
                TcpState::Connected(stream) => {
                    *tcp_state = TcpState::Receiving(Arc::clone(&stream));
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
                prev => {
                    *tcp_state = prev;
                    let (result_tx, result_rx) = instance
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
        let socket = TcpSocket::new(address_family.into()).context("failed to create socket")?;
        self.table
            .push(socket)
            .context("failed to push socket resource to table")
    }

    fn local_address(
        &mut self,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<IpSocketAddress, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.local_address())
    }

    fn remote_address(
        &mut self,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<IpSocketAddress, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.remote_address())
    }

    fn is_listening(&mut self, socket: Resource<TcpSocket>) -> wasmtime::Result<bool> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.is_listening())
    }

    fn address_family(&mut self, socket: Resource<TcpSocket>) -> wasmtime::Result<IpAddressFamily> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.address_family())
    }

    fn set_listen_backlog_size(
        &mut self,
        socket: Resource<TcpSocket>,
        value: u64,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let sock = get_socket_mut(self.table, &socket)?;
        Ok(sock.set_listen_backlog_size(value))
    }

    fn keep_alive_enabled(
        &mut self,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<bool, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.keep_alive_enabled())
    }

    fn set_keep_alive_enabled(
        &mut self,
        socket: Resource<TcpSocket>,
        value: bool,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.set_keep_alive_enabled(value))
    }

    fn keep_alive_idle_time(
        &mut self,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<Duration, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.keep_alive_idle_time())
    }

    fn set_keep_alive_idle_time(
        &mut self,
        socket: Resource<TcpSocket>,
        value: Duration,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let sock = get_socket_mut(self.table, &socket)?;
        Ok(sock.set_keep_alive_idle_time(value))
    }

    fn keep_alive_interval(
        &mut self,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<Duration, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.keep_alive_interval())
    }

    fn set_keep_alive_interval(
        &mut self,
        socket: Resource<TcpSocket>,
        value: Duration,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.set_keep_alive_interval(value))
    }

    fn keep_alive_count(
        &mut self,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<u32, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.keep_alive_count())
    }

    fn set_keep_alive_count(
        &mut self,
        socket: Resource<TcpSocket>,
        value: u32,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.set_keep_alive_count(value))
    }

    fn hop_limit(
        &mut self,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<u8, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.hop_limit())
    }

    fn set_hop_limit(
        &mut self,
        socket: Resource<TcpSocket>,
        value: u8,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let sock = get_socket_mut(self.table, &socket)?;
        Ok(sock.set_hop_limit(value))
    }

    fn receive_buffer_size(
        &mut self,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<u64, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.receive_buffer_size())
    }

    fn set_receive_buffer_size(
        &mut self,
        socket: Resource<TcpSocket>,
        value: u64,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let sock = get_socket_mut(self.table, &socket)?;
        Ok(sock.set_receive_buffer_size(value))
    }

    fn send_buffer_size(
        &mut self,
        socket: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<u64, ErrorCode>> {
        let sock = get_socket(self.table, &socket)?;
        Ok(sock.send_buffer_size())
    }

    fn set_send_buffer_size(
        &mut self,
        socket: Resource<TcpSocket>,
        value: u64,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let sock = get_socket_mut(self.table, &socket)?;
        Ok(sock.set_send_buffer_size(value))
    }

    fn drop(&mut self, sock: Resource<TcpSocket>) -> wasmtime::Result<()> {
        self.table
            .delete(sock)
            .context("failed to delete socket resource from table")?;
        Ok(())
    }
}

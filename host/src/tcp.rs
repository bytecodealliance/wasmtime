#![allow(unused_variables)]

use crate::{
    wasi_poll::{InputStream, OutputStream},
    wasi_tcp::{
        Connection, ConnectionFlags, Errno, IoSize, IpSocketAddress, Ipv4SocketAddress,
        Ipv6SocketAddress, Listener, ListenerFlags, Network, TcpListener, WasiTcp,
    },
    HostResult, WasiCtx,
};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::ops::BitAnd;
use wasi_common::listener::TableListenerExt;
use wasi_common::tcp_listener::TableTcpListenerExt;

/// TODO: Remove once wasmtime #5589 lands.
fn contains<T: BitAnd<Output = T> + Eq + Copy>(flags: T, flag: T) -> bool {
    (flags & flag) == flag
}

fn convert(error: wasi_common::Error) -> anyhow::Error {
    if let Some(errno) = error.downcast_ref() {
        use wasi_common::Errno::*;

        match errno {
            Acces => Errno::Access,
            Again => Errno::Again,
            Already => Errno::Already,
            Badf => Errno::Badf,
            Busy => Errno::Busy,
            Ilseq => Errno::Ilseq,
            Inprogress => Errno::Inprogress,
            Intr => Errno::Intr,
            Inval => Errno::Inval,
            Io => Errno::Io,
            Msgsize => Errno::Msgsize,
            Nametoolong => Errno::Nametoolong,
            Noent => Errno::Noent,
            Nomem => Errno::Nomem,
            Nosys => Errno::Nosys,
            Notrecoverable => Errno::Notrecoverable,
            Notsup => Errno::Notsup,
            Overflow => Errno::Overflow,
            Perm => Errno::Perm,
            Addrinuse => Errno::Addrinuse,
            Addrnotavail => Errno::Addrnotavail,
            Afnosupport => Errno::Afnosupport,
            Connaborted => Errno::ConnectionAborted,
            Connrefused => Errno::ConnectionRefused,
            Connreset => Errno::ConnectionReset,
            Destaddrreq => Errno::Destaddrreq,
            Hostunreach => Errno::HostUnreachable,
            Isconn => Errno::Isconn,
            Multihop => Errno::Multihop,
            Netreset => Errno::NetworkReset,
            Netdown => Errno::NetworkDown,
            Netunreach => Errno::NetworkUnreachable,
            Nobufs => Errno::Nobufs,
            Noprotoopt => Errno::Noprotoopt,
            Timedout => Errno::Timedout,
            _ => {
                panic!("Unexpected errno: {:?}", errno);
            }
        }
        .into()
    } else {
        error.into()
    }
}

#[async_trait::async_trait]
impl WasiTcp for WasiCtx {
    async fn listen(
        &mut self,
        network: Network,
        address: IpSocketAddress,
        backlog: Option<u32>,
        flags: ListenerFlags,
    ) -> HostResult<TcpListener, Errno> {
        todo!()
    }

    async fn accept(
        &mut self,
        listener: Listener,
        flags: ConnectionFlags,
    ) -> HostResult<(Connection, InputStream, OutputStream), Errno> {
        let table = self.table_mut();
        let l = table.get_listener_mut(listener)?;

        let nonblocking = contains(flags, ConnectionFlags::NONBLOCK);

        if contains(flags, ConnectionFlags::KEEPALIVE) || contains(flags, ConnectionFlags::NODELAY)
        {
            todo!()
        }

        let (connection, input_stream, output_stream) = l.accept(nonblocking).await?;

        let connection = table.push(Box::new(connection)).map_err(convert)?;
        let input_stream = table.push(Box::new(input_stream)).map_err(convert)?;
        let output_stream = table.push(Box::new(output_stream)).map_err(convert)?;

        Ok(Ok((connection, input_stream, output_stream)))
    }

    async fn accept_tcp(
        &mut self,
        listener: TcpListener,
        flags: ConnectionFlags,
    ) -> HostResult<(Connection, InputStream, OutputStream, IpSocketAddress), Errno> {
        let table = self.table_mut();
        let l = table.get_tcp_listener_mut(listener)?;

        let nonblocking = contains(flags, ConnectionFlags::NONBLOCK);

        if contains(flags, ConnectionFlags::KEEPALIVE) || contains(flags, ConnectionFlags::NODELAY)
        {
            todo!()
        }

        let (connection, input_stream, output_stream, addr) = l.accept(nonblocking).await?;

        let connection = table.push(Box::new(connection)).map_err(convert)?;
        let input_stream = table.push(Box::new(input_stream)).map_err(convert)?;
        let output_stream = table.push(Box::new(output_stream)).map_err(convert)?;

        Ok(Ok((connection, input_stream, output_stream, addr.into())))
    }

    async fn connect(
        &mut self,
        network: Network,
        local_address: IpSocketAddress,
        remote_address: IpSocketAddress,
        flags: ConnectionFlags,
    ) -> HostResult<(Connection, InputStream, OutputStream), Errno> {
        todo!()
    }

    async fn send(&mut self, connection: Connection, bytes: Vec<u8>) -> HostResult<IoSize, Errno> {
        todo!()
    }

    async fn receive(
        &mut self,
        connection: Connection,
        length: IoSize,
    ) -> HostResult<(Vec<u8>, bool), Errno> {
        todo!()
    }

    async fn is_connected(&mut self, connection: Connection) -> anyhow::Result<bool> {
        // This should ultimately call `getpeername` and test whether it
        // gets a `ENOTCONN` error indicating not-connected.
        todo!()
    }

    async fn get_flags(&mut self, connection: Connection) -> HostResult<ConnectionFlags, Errno> {
        todo!()
    }

    async fn set_flags(
        &mut self,
        connection: Connection,
        flags: ConnectionFlags,
    ) -> HostResult<(), Errno> {
        todo!()
    }

    async fn get_receive_buffer_size(
        &mut self,
        connection: Connection,
    ) -> HostResult<IoSize, Errno> {
        todo!()
    }

    async fn set_receive_buffer_size(
        &mut self,
        connection: Connection,
        value: IoSize,
    ) -> HostResult<(), Errno> {
        todo!()
    }

    async fn get_send_buffer_size(&mut self, connection: Connection) -> HostResult<IoSize, Errno> {
        todo!()
    }

    async fn set_send_buffer_size(
        &mut self,
        connection: Connection,
        value: IoSize,
    ) -> HostResult<(), Errno> {
        todo!()
    }

    async fn bytes_readable(&mut self, socket: Connection) -> HostResult<(IoSize, bool), Errno> {
        drop(socket);
        todo!()
    }

    async fn bytes_writable(&mut self, socket: Connection) -> HostResult<(IoSize, bool), Errno> {
        drop(socket);
        todo!()
    }

    async fn close_tcp_listener(&mut self, listener: TcpListener) -> anyhow::Result<()> {
        drop(listener);
        todo!()
    }

    async fn close_connection(&mut self, connection: Connection) -> anyhow::Result<()> {
        drop(connection);
        todo!()
    }
}

impl From<SocketAddr> for IpSocketAddress {
    fn from(addr: SocketAddr) -> Self {
        match addr {
            SocketAddr::V4(v4) => Self::Ipv4(v4.into()),
            SocketAddr::V6(v6) => Self::Ipv6(v6.into()),
        }
    }
}

impl From<SocketAddrV4> for Ipv4SocketAddress {
    fn from(addr: SocketAddrV4) -> Self {
        Self {
            address: MyIpv4Addr::from(addr.ip()).0,
            port: addr.port(),
        }
    }
}

impl From<SocketAddrV6> for Ipv6SocketAddress {
    fn from(addr: SocketAddrV6) -> Self {
        Self {
            address: MyIpv6Addr::from(addr.ip()).0,
            port: addr.port(),
            flow_info: addr.flowinfo(),
            scope_id: addr.scope_id(),
        }
    }
}

// Newtypes to guide conversions.
struct MyIpv4Addr((u8, u8, u8, u8));
struct MyIpv6Addr((u16, u16, u16, u16, u16, u16, u16, u16));

impl From<&Ipv4Addr> for MyIpv4Addr {
    fn from(addr: &Ipv4Addr) -> Self {
        let octets = addr.octets();
        Self((octets[0], octets[1], octets[2], octets[3]))
    }
}

impl From<&Ipv6Addr> for MyIpv6Addr {
    fn from(addr: &Ipv6Addr) -> Self {
        let segments = addr.segments();
        Self((
            segments[0],
            segments[1],
            segments[2],
            segments[3],
            segments[4],
            segments[5],
            segments[6],
            segments[7],
        ))
    }
}

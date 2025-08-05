use crate::p2::bindings::sockets::network::{ErrorCode, IpAddressFamily, IpSocketAddress, Network};
use crate::p2::bindings::sockets::udp;
use crate::p2::udp::{IncomingDatagramStream, OutgoingDatagramStream, SendState};
use crate::p2::{Pollable, SocketError, SocketResult};
use crate::sockets::util::{is_valid_address_family, is_valid_remote_address};
use crate::sockets::{
    MAX_UDP_DATAGRAM_SIZE, SocketAddrUse, SocketAddressFamily, UdpSocket, WasiSocketsCtxView,
};
use anyhow::anyhow;
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::io::Interest;
use wasmtime::component::Resource;
use wasmtime_wasi_io::poll::DynPollable;

impl udp::Host for WasiSocketsCtxView<'_> {}

impl udp::HostUdpSocket for WasiSocketsCtxView<'_> {
    async fn start_bind(
        &mut self,
        this: Resource<udp::UdpSocket>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let local_address = SocketAddr::from(local_address);
        let check = self.table.get(&network)?.socket_addr_check.clone();
        check.check(local_address, SocketAddrUse::UdpBind).await?;

        let socket = self.table.get_mut(&this)?;
        socket.bind(local_address)?;
        socket.set_socket_addr_check(Some(check));

        Ok(())
    }

    fn finish_bind(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<()> {
        self.table.get_mut(&this)?.finish_bind()?;
        Ok(())
    }

    async fn stream(
        &mut self,
        this: Resource<udp::UdpSocket>,
        remote_address: Option<IpSocketAddress>,
    ) -> SocketResult<(
        Resource<udp::IncomingDatagramStream>,
        Resource<udp::OutgoingDatagramStream>,
    )> {
        let has_active_streams = self
            .table
            .iter_children(&this)?
            .any(|c| c.is::<IncomingDatagramStream>() || c.is::<OutgoingDatagramStream>());

        if has_active_streams {
            return Err(SocketError::trap(anyhow!("UDP streams not dropped yet")));
        }

        let socket = self.table.get_mut(&this)?;
        let remote_address = remote_address.map(SocketAddr::from);

        if !socket.is_bound() {
            return Err(ErrorCode::InvalidState.into());
        }

        // We disconnect & (re)connect in two distinct steps for two reasons:
        // - To leave our socket instance in a consistent state in case the
        //   connect fails.
        // - When reconnecting to a different address, Linux sometimes fails
        //   if there isn't a disconnect in between.

        // Step #1: Disconnect
        if socket.is_connected() {
            socket.disconnect()?;
        }

        // Step #2: (Re)connect
        if let Some(connect_addr) = remote_address {
            let Some(check) = socket.socket_addr_check() else {
                return Err(ErrorCode::InvalidState.into());
            };
            check.check(connect_addr, SocketAddrUse::UdpConnect).await?;
            socket.connect(connect_addr)?;
        }

        let incoming_stream = IncomingDatagramStream {
            inner: socket.socket().clone(),
            remote_address,
        };
        let outgoing_stream = OutgoingDatagramStream {
            inner: socket.socket().clone(),
            remote_address,
            family: socket.address_family(),
            send_state: SendState::Idle,
            socket_addr_check: socket.socket_addr_check().cloned(),
        };

        Ok((
            self.table.push_child(incoming_stream, &this)?,
            self.table.push_child(outgoing_stream, &this)?,
        ))
    }

    fn local_address(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<IpSocketAddress> {
        let socket = self.table.get(&this)?;
        Ok(socket.local_address()?.into())
    }

    fn remote_address(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<IpSocketAddress> {
        let socket = self.table.get(&this)?;
        Ok(socket.remote_address()?.into())
    }

    fn address_family(
        &mut self,
        this: Resource<udp::UdpSocket>,
    ) -> Result<IpAddressFamily, anyhow::Error> {
        let socket = self.table.get(&this)?;
        Ok(socket.address_family().into())
    }

    fn unicast_hop_limit(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<u8> {
        let socket = self.table.get(&this)?;
        Ok(socket.unicast_hop_limit()?)
    }

    fn set_unicast_hop_limit(
        &mut self,
        this: Resource<udp::UdpSocket>,
        value: u8,
    ) -> SocketResult<()> {
        let socket = self.table.get(&this)?;
        socket.set_unicast_hop_limit(value)?;
        Ok(())
    }

    fn receive_buffer_size(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<u64> {
        let socket = self.table.get(&this)?;
        Ok(socket.receive_buffer_size()?)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<udp::UdpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let socket = self.table.get(&this)?;
        socket.set_receive_buffer_size(value)?;
        Ok(())
    }

    fn send_buffer_size(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<u64> {
        let socket = self.table.get(&this)?;
        Ok(socket.send_buffer_size()?)
    }

    fn set_send_buffer_size(&mut self, this: Resource<UdpSocket>, value: u64) -> SocketResult<()> {
        let socket = self.table.get(&this)?;
        socket.set_send_buffer_size(value)?;
        Ok(())
    }

    fn subscribe(&mut self, this: Resource<UdpSocket>) -> anyhow::Result<Resource<DynPollable>> {
        wasmtime_wasi_io::poll::subscribe(self.table, this)
    }

    fn drop(&mut self, this: Resource<udp::UdpSocket>) -> Result<(), anyhow::Error> {
        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = self.table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

#[async_trait]
impl Pollable for UdpSocket {
    async fn ready(&mut self) {
        // None of the socket-level operations block natively
    }
}

impl udp::HostIncomingDatagramStream for WasiSocketsCtxView<'_> {
    fn receive(
        &mut self,
        this: Resource<udp::IncomingDatagramStream>,
        max_results: u64,
    ) -> SocketResult<Vec<udp::IncomingDatagram>> {
        // Returns Ok(None) when the message was dropped.
        fn recv_one(
            stream: &IncomingDatagramStream,
        ) -> SocketResult<Option<udp::IncomingDatagram>> {
            let mut buf = [0; MAX_UDP_DATAGRAM_SIZE];
            let (size, received_addr) = stream.inner.try_recv_from(&mut buf)?;
            debug_assert!(size <= buf.len());

            match stream.remote_address {
                Some(connected_addr) if connected_addr != received_addr => {
                    // Normally, this should have already been checked for us by the OS.
                    return Ok(None);
                }
                _ => {}
            }

            Ok(Some(udp::IncomingDatagram {
                data: buf[..size].into(),
                remote_address: received_addr.into(),
            }))
        }

        let stream = self.table.get(&this)?;
        let max_results: usize = max_results.try_into().unwrap_or(usize::MAX);

        if max_results == 0 {
            return Ok(vec![]);
        }

        let mut datagrams = vec![];

        while datagrams.len() < max_results {
            match recv_one(stream) {
                Ok(Some(datagram)) => {
                    datagrams.push(datagram);
                }
                Ok(None) => {
                    // Message was dropped
                }
                Err(_) if datagrams.len() > 0 => {
                    return Ok(datagrams);
                }
                Err(e) if matches!(e.downcast_ref(), Some(ErrorCode::WouldBlock)) => {
                    return Ok(datagrams);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(datagrams)
    }

    fn subscribe(
        &mut self,
        this: Resource<udp::IncomingDatagramStream>,
    ) -> anyhow::Result<Resource<DynPollable>> {
        wasmtime_wasi_io::poll::subscribe(self.table, this)
    }

    fn drop(&mut self, this: Resource<udp::IncomingDatagramStream>) -> Result<(), anyhow::Error> {
        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = self.table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

#[async_trait]
impl Pollable for IncomingDatagramStream {
    async fn ready(&mut self) {
        // FIXME: Add `Interest::ERROR` when we update to tokio 1.32.
        self.inner
            .ready(Interest::READABLE)
            .await
            .expect("failed to await UDP socket readiness");
    }
}

impl udp::HostOutgoingDatagramStream for WasiSocketsCtxView<'_> {
    fn check_send(&mut self, this: Resource<udp::OutgoingDatagramStream>) -> SocketResult<u64> {
        let stream = self.table.get_mut(&this)?;

        let permit = match stream.send_state {
            SendState::Idle => {
                const PERMIT: usize = 16;
                stream.send_state = SendState::Permitted(PERMIT);
                PERMIT
            }
            SendState::Permitted(n) => n,
            SendState::Waiting => 0,
        };

        Ok(permit.try_into().unwrap())
    }

    async fn send(
        &mut self,
        this: Resource<udp::OutgoingDatagramStream>,
        datagrams: Vec<udp::OutgoingDatagram>,
    ) -> SocketResult<u64> {
        async fn send_one(
            stream: &OutgoingDatagramStream,
            datagram: &udp::OutgoingDatagram,
        ) -> SocketResult<()> {
            if datagram.data.len() > MAX_UDP_DATAGRAM_SIZE {
                return Err(ErrorCode::DatagramTooLarge.into());
            }

            let provided_addr = datagram.remote_address.map(SocketAddr::from);
            let addr = match (stream.remote_address, provided_addr) {
                (None, Some(addr)) => {
                    let Some(check) = stream.socket_addr_check.as_ref() else {
                        return Err(ErrorCode::InvalidState.into());
                    };
                    check
                        .check(addr, SocketAddrUse::UdpOutgoingDatagram)
                        .await?;
                    addr
                }
                (Some(addr), None) => addr,
                (Some(connected_addr), Some(provided_addr)) if connected_addr == provided_addr => {
                    connected_addr
                }
                _ => return Err(ErrorCode::InvalidArgument.into()),
            };

            if !is_valid_remote_address(addr) || !is_valid_address_family(addr.ip(), stream.family)
            {
                return Err(ErrorCode::InvalidArgument.into());
            }

            if stream.remote_address == Some(addr) {
                stream.inner.try_send(&datagram.data)?;
            } else {
                stream.inner.try_send_to(&datagram.data, addr)?;
            }

            Ok(())
        }

        let stream = self.table.get_mut(&this)?;

        match stream.send_state {
            SendState::Permitted(n) if n >= datagrams.len() => {
                stream.send_state = SendState::Idle;
            }
            SendState::Permitted(_) => {
                return Err(SocketError::trap(anyhow::anyhow!(
                    "unpermitted: argument exceeds permitted size"
                )));
            }
            SendState::Idle | SendState::Waiting => {
                return Err(SocketError::trap(anyhow::anyhow!(
                    "unpermitted: must call check-send first"
                )));
            }
        }

        if datagrams.is_empty() {
            return Ok(0);
        }

        let mut count = 0;

        for datagram in datagrams {
            match send_one(stream, &datagram).await {
                Ok(_) => count += 1,
                Err(_) if count > 0 => {
                    // WIT: "If at least one datagram has been sent successfully, this function never returns an error."
                    return Ok(count);
                }
                Err(e) if matches!(e.downcast_ref(), Some(ErrorCode::WouldBlock)) => {
                    stream.send_state = SendState::Waiting;
                    return Ok(count);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(count)
    }

    fn subscribe(
        &mut self,
        this: Resource<udp::OutgoingDatagramStream>,
    ) -> anyhow::Result<Resource<DynPollable>> {
        wasmtime_wasi_io::poll::subscribe(self.table, this)
    }

    fn drop(&mut self, this: Resource<udp::OutgoingDatagramStream>) -> Result<(), anyhow::Error> {
        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = self.table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

#[async_trait]
impl Pollable for OutgoingDatagramStream {
    async fn ready(&mut self) {
        match self.send_state {
            SendState::Idle | SendState::Permitted(_) => {}
            SendState::Waiting => {
                // FIXME: Add `Interest::ERROR` when we update to tokio 1.32.
                self.inner
                    .ready(Interest::WRITABLE)
                    .await
                    .expect("failed to await UDP socket readiness");
                self.send_state = SendState::Idle;
            }
        }
    }
}

impl From<SocketAddressFamily> for IpAddressFamily {
    fn from(family: SocketAddressFamily) -> IpAddressFamily {
        match family {
            SocketAddressFamily::Ipv4 => IpAddressFamily::Ipv4,
            SocketAddressFamily::Ipv6 => IpAddressFamily::Ipv6,
        }
    }
}

pub mod sync {
    use wasmtime::component::Resource;

    use crate::p2::{
        SocketError,
        bindings::{
            sockets::{
                network::Network,
                udp::{
                    self as async_udp,
                    HostIncomingDatagramStream as AsyncHostIncomingDatagramStream,
                    HostOutgoingDatagramStream as AsyncHostOutgoingDatagramStream,
                    HostUdpSocket as AsyncHostUdpSocket, IncomingDatagramStream,
                    OutgoingDatagramStream,
                },
            },
            sync::sockets::udp::{
                self, HostIncomingDatagramStream, HostOutgoingDatagramStream, HostUdpSocket,
                IncomingDatagram, IpAddressFamily, IpSocketAddress, OutgoingDatagram, Pollable,
                UdpSocket,
            },
        },
    };
    use crate::runtime::in_tokio;
    use crate::sockets::WasiSocketsCtxView;

    impl udp::Host for WasiSocketsCtxView<'_> {}

    impl HostUdpSocket for WasiSocketsCtxView<'_> {
        fn start_bind(
            &mut self,
            self_: Resource<UdpSocket>,
            network: Resource<Network>,
            local_address: IpSocketAddress,
        ) -> Result<(), SocketError> {
            in_tokio(async {
                AsyncHostUdpSocket::start_bind(self, self_, network, local_address).await
            })
        }

        fn finish_bind(&mut self, self_: Resource<UdpSocket>) -> Result<(), SocketError> {
            AsyncHostUdpSocket::finish_bind(self, self_)
        }

        fn stream(
            &mut self,
            self_: Resource<UdpSocket>,
            remote_address: Option<IpSocketAddress>,
        ) -> Result<
            (
                Resource<IncomingDatagramStream>,
                Resource<OutgoingDatagramStream>,
            ),
            SocketError,
        > {
            in_tokio(async { AsyncHostUdpSocket::stream(self, self_, remote_address).await })
        }

        fn local_address(
            &mut self,
            self_: Resource<UdpSocket>,
        ) -> Result<IpSocketAddress, SocketError> {
            AsyncHostUdpSocket::local_address(self, self_)
        }

        fn remote_address(
            &mut self,
            self_: Resource<UdpSocket>,
        ) -> Result<IpSocketAddress, SocketError> {
            AsyncHostUdpSocket::remote_address(self, self_)
        }

        fn address_family(
            &mut self,
            self_: Resource<UdpSocket>,
        ) -> wasmtime::Result<IpAddressFamily> {
            AsyncHostUdpSocket::address_family(self, self_)
        }

        fn unicast_hop_limit(&mut self, self_: Resource<UdpSocket>) -> Result<u8, SocketError> {
            AsyncHostUdpSocket::unicast_hop_limit(self, self_)
        }

        fn set_unicast_hop_limit(
            &mut self,
            self_: Resource<UdpSocket>,
            value: u8,
        ) -> Result<(), SocketError> {
            AsyncHostUdpSocket::set_unicast_hop_limit(self, self_, value)
        }

        fn receive_buffer_size(&mut self, self_: Resource<UdpSocket>) -> Result<u64, SocketError> {
            AsyncHostUdpSocket::receive_buffer_size(self, self_)
        }

        fn set_receive_buffer_size(
            &mut self,
            self_: Resource<UdpSocket>,
            value: u64,
        ) -> Result<(), SocketError> {
            AsyncHostUdpSocket::set_receive_buffer_size(self, self_, value)
        }

        fn send_buffer_size(&mut self, self_: Resource<UdpSocket>) -> Result<u64, SocketError> {
            AsyncHostUdpSocket::send_buffer_size(self, self_)
        }

        fn set_send_buffer_size(
            &mut self,
            self_: Resource<UdpSocket>,
            value: u64,
        ) -> Result<(), SocketError> {
            AsyncHostUdpSocket::set_send_buffer_size(self, self_, value)
        }

        fn subscribe(
            &mut self,
            self_: Resource<UdpSocket>,
        ) -> wasmtime::Result<Resource<Pollable>> {
            AsyncHostUdpSocket::subscribe(self, self_)
        }

        fn drop(&mut self, rep: Resource<UdpSocket>) -> wasmtime::Result<()> {
            AsyncHostUdpSocket::drop(self, rep)
        }
    }

    impl HostIncomingDatagramStream for WasiSocketsCtxView<'_> {
        fn receive(
            &mut self,
            self_: Resource<IncomingDatagramStream>,
            max_results: u64,
        ) -> Result<Vec<IncomingDatagram>, SocketError> {
            Ok(
                AsyncHostIncomingDatagramStream::receive(self, self_, max_results)?
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            )
        }

        fn subscribe(
            &mut self,
            self_: Resource<IncomingDatagramStream>,
        ) -> wasmtime::Result<Resource<Pollable>> {
            AsyncHostIncomingDatagramStream::subscribe(self, self_)
        }

        fn drop(&mut self, rep: Resource<IncomingDatagramStream>) -> wasmtime::Result<()> {
            AsyncHostIncomingDatagramStream::drop(self, rep)
        }
    }

    impl From<async_udp::IncomingDatagram> for IncomingDatagram {
        fn from(other: async_udp::IncomingDatagram) -> Self {
            let async_udp::IncomingDatagram {
                data,
                remote_address,
            } = other;
            Self {
                data,
                remote_address,
            }
        }
    }

    impl HostOutgoingDatagramStream for WasiSocketsCtxView<'_> {
        fn check_send(
            &mut self,
            self_: Resource<OutgoingDatagramStream>,
        ) -> Result<u64, SocketError> {
            AsyncHostOutgoingDatagramStream::check_send(self, self_)
        }

        fn send(
            &mut self,
            self_: Resource<OutgoingDatagramStream>,
            datagrams: Vec<OutgoingDatagram>,
        ) -> Result<u64, SocketError> {
            let datagrams = datagrams.into_iter().map(Into::into).collect();
            in_tokio(async { AsyncHostOutgoingDatagramStream::send(self, self_, datagrams).await })
        }

        fn subscribe(
            &mut self,
            self_: Resource<OutgoingDatagramStream>,
        ) -> wasmtime::Result<Resource<Pollable>> {
            AsyncHostOutgoingDatagramStream::subscribe(self, self_)
        }

        fn drop(&mut self, rep: Resource<OutgoingDatagramStream>) -> wasmtime::Result<()> {
            AsyncHostOutgoingDatagramStream::drop(self, rep)
        }
    }

    impl From<OutgoingDatagram> for async_udp::OutgoingDatagram {
        fn from(other: OutgoingDatagram) -> Self {
            let OutgoingDatagram {
                data,
                remote_address,
            } = other;
            Self {
                data,
                remote_address,
            }
        }
    }
}

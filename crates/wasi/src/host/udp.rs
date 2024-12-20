use crate::host::network::util;
use crate::network::{SocketAddrUse, SocketAddressFamily};
use crate::{
    bindings::{
        sockets::network::{ErrorCode, IpAddressFamily, IpSocketAddress, Network},
        sockets::udp,
    },
    udp::{IncomingDatagramStream, OutgoingDatagramStream, SendState, UdpState},
    Subscribe,
};
use crate::{Pollable, SocketError, SocketResult, WasiImpl, WasiView};
use anyhow::anyhow;
use async_trait::async_trait;
use io_lifetimes::AsSocketlike;
use rustix::io::Errno;
use std::net::SocketAddr;
use tokio::io::Interest;
use wasmtime::component::Resource;

/// Theoretical maximum byte size of a UDP datagram, the real limit is lower,
/// but we do not account for e.g. the transport layer here for simplicity.
/// In practice, datagrams are typically less than 1500 bytes.
const MAX_UDP_DATAGRAM_SIZE: usize = u16::MAX as usize;

impl<T> udp::Host for WasiImpl<T> where T: WasiView {}

impl<T> udp::HostUdpSocket for WasiImpl<T>
where
    T: WasiView,
{
    async fn start_bind(
        &mut self,
        this: Resource<udp::UdpSocket>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        self.ctx().allowed_network_uses.check_allowed_udp()?;
        let table = self.table();

        match table.get(&this)?.udp_state {
            UdpState::Default => {}
            UdpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            UdpState::Bound | UdpState::Connected => return Err(ErrorCode::InvalidState.into()),
        }

        // Set the socket addr check on the socket so later functions have access to it through the socket handle
        let check = table.get(&network)?.socket_addr_check.clone();
        table
            .get_mut(&this)?
            .socket_addr_check
            .replace(check.clone());

        let socket = table.get(&this)?;
        let local_address: SocketAddr = local_address.into();

        util::validate_address_family(&local_address, &socket.family)?;

        {
            check.check(local_address, SocketAddrUse::UdpBind).await?;

            // Perform the OS bind call.
            util::udp_bind(socket.udp_socket(), &local_address).map_err(|error| match error {
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
        socket.udp_state = UdpState::BindStarted;

        Ok(())
    }

    fn finish_bind(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get_mut(&this)?;

        match socket.udp_state {
            UdpState::BindStarted => {
                socket.udp_state = UdpState::Bound;
                Ok(())
            }
            _ => Err(ErrorCode::NotInProgress.into()),
        }
    }

    async fn stream(
        &mut self,
        this: Resource<udp::UdpSocket>,
        remote_address: Option<IpSocketAddress>,
    ) -> SocketResult<(
        Resource<udp::IncomingDatagramStream>,
        Resource<udp::OutgoingDatagramStream>,
    )> {
        let table = self.table();

        let has_active_streams = table
            .iter_children(&this)?
            .any(|c| c.is::<IncomingDatagramStream>() || c.is::<OutgoingDatagramStream>());

        if has_active_streams {
            return Err(SocketError::trap(anyhow!("UDP streams not dropped yet")));
        }

        let socket = table.get_mut(&this)?;
        let remote_address = remote_address.map(SocketAddr::from);

        match socket.udp_state {
            UdpState::Bound | UdpState::Connected => {}
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        // We disconnect & (re)connect in two distinct steps for two reasons:
        // - To leave our socket instance in a consistent state in case the
        //   connect fails.
        // - When reconnecting to a different address, Linux sometimes fails
        //   if there isn't a disconnect in between.

        // Step #1: Disconnect
        if let UdpState::Connected = socket.udp_state {
            util::udp_disconnect(socket.udp_socket())?;
            socket.udp_state = UdpState::Bound;
        }

        // Step #2: (Re)connect
        if let Some(connect_addr) = remote_address {
            let Some(check) = socket.socket_addr_check.as_ref() else {
                return Err(ErrorCode::InvalidState.into());
            };
            util::validate_remote_address(&connect_addr)?;
            util::validate_address_family(&connect_addr, &socket.family)?;
            check.check(connect_addr, SocketAddrUse::UdpConnect).await?;

            rustix::net::connect(socket.udp_socket(), &connect_addr).map_err(
                |error| match error {
                    Errno::AFNOSUPPORT => ErrorCode::InvalidArgument, // See `bind` implementation.
                    Errno::INPROGRESS => {
                        tracing::debug!(
                            "UDP connect returned EINPROGRESS, which should never happen"
                        );
                        ErrorCode::Unknown
                    }
                    _ => ErrorCode::from(error),
                },
            )?;
            socket.udp_state = UdpState::Connected;
        }

        let incoming_stream = IncomingDatagramStream {
            inner: socket.inner.clone(),
            remote_address,
        };
        let outgoing_stream = OutgoingDatagramStream {
            inner: socket.inner.clone(),
            remote_address,
            family: socket.family,
            send_state: SendState::Idle,
            socket_addr_check: socket.socket_addr_check.clone(),
        };

        Ok((
            self.table().push_child(incoming_stream, &this)?,
            self.table().push_child(outgoing_stream, &this)?,
        ))
    }

    fn local_address(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.udp_state {
            UdpState::Default => return Err(ErrorCode::InvalidState.into()),
            UdpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => {}
        }

        let addr = socket
            .udp_socket()
            .as_socketlike_view::<std::net::UdpSocket>()
            .local_addr()?;
        Ok(addr.into())
    }

    fn remote_address(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.udp_state {
            UdpState::Connected => {}
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        let addr = socket
            .udp_socket()
            .as_socketlike_view::<std::net::UdpSocket>()
            .peer_addr()?;
        Ok(addr.into())
    }

    fn address_family(
        &mut self,
        this: Resource<udp::UdpSocket>,
    ) -> Result<IpAddressFamily, anyhow::Error> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.family {
            SocketAddressFamily::Ipv4 => Ok(IpAddressFamily::Ipv4),
            SocketAddressFamily::Ipv6 => Ok(IpAddressFamily::Ipv6),
        }
    }

    fn unicast_hop_limit(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<u8> {
        let table = self.table();
        let socket = table.get(&this)?;

        let ttl = match socket.family {
            SocketAddressFamily::Ipv4 => util::get_ip_ttl(socket.udp_socket())?,
            SocketAddressFamily::Ipv6 => util::get_ipv6_unicast_hops(socket.udp_socket())?,
        };

        Ok(ttl)
    }

    fn set_unicast_hop_limit(
        &mut self,
        this: Resource<udp::UdpSocket>,
        value: u8,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.family {
            SocketAddressFamily::Ipv4 => util::set_ip_ttl(socket.udp_socket(), value)?,
            SocketAddressFamily::Ipv6 => util::set_ipv6_unicast_hops(socket.udp_socket(), value)?,
        }

        Ok(())
    }

    fn receive_buffer_size(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;

        let value = util::get_socket_recv_buffer_size(socket.udp_socket())?;
        Ok(value as u64)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<udp::UdpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);

        util::set_socket_recv_buffer_size(socket.udp_socket(), value)?;
        Ok(())
    }

    fn send_buffer_size(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;

        let value = util::get_socket_send_buffer_size(socket.udp_socket())?;
        Ok(value as u64)
    }

    fn set_send_buffer_size(
        &mut self,
        this: Resource<udp::UdpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);

        util::set_socket_send_buffer_size(socket.udp_socket(), value)?;
        Ok(())
    }

    fn subscribe(&mut self, this: Resource<udp::UdpSocket>) -> anyhow::Result<Resource<Pollable>> {
        crate::poll::subscribe(self.table(), this)
    }

    fn drop(&mut self, this: Resource<udp::UdpSocket>) -> Result<(), anyhow::Error> {
        let table = self.table();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

impl<T> udp::HostIncomingDatagramStream for WasiImpl<T>
where
    T: WasiView,
{
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

        let table = self.table();
        let stream = table.get(&this)?;
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
    ) -> anyhow::Result<Resource<Pollable>> {
        crate::poll::subscribe(self.table(), this)
    }

    fn drop(&mut self, this: Resource<udp::IncomingDatagramStream>) -> Result<(), anyhow::Error> {
        let table = self.table();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

#[async_trait]
impl Subscribe for IncomingDatagramStream {
    async fn ready(&mut self) {
        // FIXME: Add `Interest::ERROR` when we update to tokio 1.32.
        self.inner
            .ready(Interest::READABLE)
            .await
            .expect("failed to await UDP socket readiness");
    }
}

impl<T> udp::HostOutgoingDatagramStream for WasiImpl<T>
where
    T: WasiView,
{
    fn check_send(&mut self, this: Resource<udp::OutgoingDatagramStream>) -> SocketResult<u64> {
        let table = self.table();
        let stream = table.get_mut(&this)?;

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

            util::validate_remote_address(&addr)?;
            util::validate_address_family(&addr, &stream.family)?;

            if stream.remote_address == Some(addr) {
                stream.inner.try_send(&datagram.data)?;
            } else {
                stream.inner.try_send_to(&datagram.data, addr)?;
            }

            Ok(())
        }

        let table = self.table();
        let stream = table.get_mut(&this)?;

        match stream.send_state {
            SendState::Permitted(n) if n >= datagrams.len() => {
                stream.send_state = SendState::Idle;
            }
            SendState::Permitted(_) => {
                return Err(SocketError::trap(anyhow::anyhow!(
                    "unpermitted: argument exceeds permitted size"
                )))
            }
            SendState::Idle | SendState::Waiting => {
                return Err(SocketError::trap(anyhow::anyhow!(
                    "unpermitted: must call check-send first"
                )))
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
    ) -> anyhow::Result<Resource<Pollable>> {
        crate::poll::subscribe(self.table(), this)
    }

    fn drop(&mut self, this: Resource<udp::OutgoingDatagramStream>) -> Result<(), anyhow::Error> {
        let table = self.table();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

#[async_trait]
impl Subscribe for OutgoingDatagramStream {
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

pub mod sync {
    use wasmtime::component::Resource;

    use crate::{
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
        runtime::in_tokio,
        SocketError, WasiImpl, WasiView,
    };

    impl<T> udp::Host for WasiImpl<T> where T: WasiView {}

    impl<T> HostUdpSocket for WasiImpl<T>
    where
        T: WasiView,
    {
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

    impl<T> HostIncomingDatagramStream for WasiImpl<T>
    where
        T: WasiView,
    {
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

    impl<T> HostOutgoingDatagramStream for WasiImpl<T>
    where
        T: WasiView,
    {
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

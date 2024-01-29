use crate::preview2::udp::{IncomingDatagramStream, OutgoingDatagramStream, UdpSocket};
use crate::preview2::{
    bindings::{
        sockets::network::{ErrorCode, IpAddressFamily, IpSocketAddress, Network},
        sockets::udp,
        sockets::udp_create_socket,
    },
    udp::UdpState,
    Subscribe,
};
use crate::preview2::{Pollable, SocketError, SocketResult, WasiView};
use anyhow::anyhow;
use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use wasmtime::component::Resource;

/// A `wasi:sockets/udp::udp-socket` instance.
/// This is mostly glue code translating between WASI types and concepts (Tables,
/// Resources, Pollables, ...) to their idiomatic Rust equivalents.
pub struct UdpSocketResource {
    inner: Arc<dyn UdpSocket + Send + Sync>,
    udp_state: UdpState,
}

#[async_trait]
impl Subscribe for UdpSocketResource {
    async fn ready(&mut self) {
        // None of the socket-level operations block natively
    }
}

/// Theoretical maximum byte size of a UDP datagram, the real limit is lower,
/// but we do not account for e.g. the transport layer here for simplicity.
/// In practice, datagrams are typically less than 1500 bytes.
const MAX_UDP_DATAGRAM_SIZE: usize = u16::MAX as usize;

impl<T: WasiView> udp::Host for T {}

impl<T: WasiView> udp_create_socket::Host for T {
    fn create_udp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> SocketResult<Resource<UdpSocketResource>> {
        let socket = self
            .ctx_mut()
            .network
            .new_udp_socket(address_family.into())?;
        let resource = UdpSocketResource {
            inner: socket.into(),
            udp_state: UdpState::Default,
        };
        let socket = self.table_mut().push(resource)?;
        Ok(socket)
    }
}

impl<T: WasiView> udp::HostUdpSocket for T {
    fn start_bind(
        &mut self,
        this: Resource<UdpSocketResource>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let table = self.table_mut();
        table.get(&network)?.check_access()?;

        match table.get(&this)?.udp_state {
            UdpState::Default => {}
            UdpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            UdpState::Bound | UdpState::Connected => return Err(ErrorCode::InvalidState.into()),
        }

        let socket = table.get(&this)?;
        let local_address: SocketAddr = local_address.into();
        socket.inner.bind(local_address)?;

        let socket = table.get_mut(&this)?;
        socket.udp_state = UdpState::BindStarted;

        Ok(())
    }

    fn finish_bind(&mut self, this: Resource<UdpSocketResource>) -> SocketResult<()> {
        let table = self.table_mut();
        let socket = table.get_mut(&this)?;

        match socket.udp_state {
            UdpState::BindStarted => {
                socket.udp_state = UdpState::Bound;
                Ok(())
            }
            _ => Err(ErrorCode::NotInProgress.into()),
        }
    }

    fn stream(
        &mut self,
        this: Resource<UdpSocketResource>,
        remote_address: Option<IpSocketAddress>,
    ) -> SocketResult<(
        Resource<udp::IncomingDatagramStream>,
        Resource<udp::OutgoingDatagramStream>,
    )> {
        let table = self.table_mut();

        let has_active_streams = table
            .iter_children(&this)?
            .any(|c| c.is::<IncomingStreamResource>() || c.is::<OutgoingStreamResource>());

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
            socket.inner.disconnect()?;
            socket.udp_state = UdpState::Bound;
        }

        // Step #2: (Re)connect
        if let Some(connect_addr) = remote_address {
            socket.inner.connect(connect_addr)?;
            socket.udp_state = UdpState::Connected;
        }

        let (incoming_stream, outgoing_stream) = socket.inner.streams();
        let incoming_stream = IncomingStreamResource::new(incoming_stream, remote_address);
        let outgoing_stream = OutgoingStreamResource::new(outgoing_stream, remote_address);

        Ok((
            self.table_mut().push_child(incoming_stream, &this)?,
            self.table_mut().push_child(outgoing_stream, &this)?,
        ))
    }

    fn local_address(
        &mut self,
        this: Resource<UdpSocketResource>,
    ) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.udp_state {
            UdpState::Default => return Err(ErrorCode::InvalidState.into()),
            UdpState::BindStarted => return Err(ErrorCode::ConcurrencyConflict.into()),
            _ => {}
        }

        let addr = socket.inner.local_address()?;
        Ok(addr.into())
    }

    fn remote_address(
        &mut self,
        this: Resource<UdpSocketResource>,
    ) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?;

        match socket.udp_state {
            UdpState::Connected => {}
            _ => return Err(ErrorCode::InvalidState.into()),
        }

        let addr = socket.inner.remote_address()?;
        Ok(addr.into())
    }

    fn address_family(
        &mut self,
        this: Resource<UdpSocketResource>,
    ) -> Result<IpAddressFamily, anyhow::Error> {
        let table = self.table();
        let socket = table.get(&this)?;

        Ok(socket.inner.address_family().into())
    }

    fn unicast_hop_limit(&mut self, this: Resource<UdpSocketResource>) -> SocketResult<u8> {
        let table = self.table();
        let socket = table.get(&this)?;

        let ttl = socket.inner.hop_limit()?;

        Ok(ttl)
    }

    fn set_unicast_hop_limit(
        &mut self,
        this: Resource<UdpSocketResource>,
        value: u8,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;

        socket.inner.set_hop_limit(value)?;

        Ok(())
    }

    fn receive_buffer_size(&mut self, this: Resource<UdpSocketResource>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;

        let value = socket.inner.receive_buffer_size()?;
        Ok(value as u64)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<UdpSocketResource>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);

        socket.inner.set_receive_buffer_size(value)?;
        Ok(())
    }

    fn send_buffer_size(&mut self, this: Resource<UdpSocketResource>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?;

        let value = socket.inner.send_buffer_size()?;
        Ok(value as u64)
    }

    fn set_send_buffer_size(
        &mut self,
        this: Resource<UdpSocketResource>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?;
        let value = value.try_into().unwrap_or(usize::MAX);

        socket.inner.set_send_buffer_size(value)?;
        Ok(())
    }

    fn subscribe(
        &mut self,
        this: Resource<UdpSocketResource>,
    ) -> anyhow::Result<Resource<Pollable>> {
        crate::preview2::poll::subscribe(self.table_mut(), this)
    }

    fn drop(&mut self, this: Resource<UdpSocketResource>) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

impl<T: WasiView> udp::HostIncomingDatagramStream for T {
    fn receive(
        &mut self,
        this: Resource<udp::IncomingDatagramStream>,
        max_results: u64,
    ) -> SocketResult<Vec<udp::IncomingDatagram>> {
        // Returns Ok(None) when the message was dropped.
        fn recv_one(
            stream: &IncomingStreamResource,
        ) -> SocketResult<Option<udp::IncomingDatagram>> {
            let mut buf = [0; MAX_UDP_DATAGRAM_SIZE];
            let (size, received_addr) = stream.inner.recv(&mut buf)?;
            debug_assert!(size <= buf.len());

            if matches!(stream.remote_address, Some(remote_address) if remote_address != received_addr)
            {
                // Normally, this should have already been checked for us by the OS.
                return Ok(None);
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
        crate::preview2::poll::subscribe(self.table_mut(), this)
    }

    fn drop(&mut self, this: Resource<udp::IncomingDatagramStream>) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

pub struct OutgoingStreamResource {
    pub(crate) inner: OutgoingDatagramStream,

    /// If this has a value, the stream is "connected".
    pub(crate) remote_address: Option<SocketAddr>,

    pub(crate) send_state: SendState,
}

pub(crate) enum SendState {
    /// Waiting for the API consumer to call `check-send`.
    Idle,
    /// Ready to send up to x datagrams.
    Permitted(usize),
    /// Waiting for the OS.
    Waiting,
}

impl OutgoingStreamResource {
    fn new(inner: OutgoingDatagramStream, remote_address: Option<SocketAddr>) -> Self {
        Self {
            inner,
            remote_address,
            send_state: SendState::Idle,
        }
    }
}

#[async_trait]
impl Subscribe for OutgoingStreamResource {
    async fn ready(&mut self) {
        if let SendState::Waiting = self.send_state {
            self.inner.ready().await;
            self.send_state = SendState::Idle;
        }
    }
}

impl<T: WasiView> udp::HostOutgoingDatagramStream for T {
    fn check_send(&mut self, this: Resource<udp::OutgoingDatagramStream>) -> SocketResult<u64> {
        let table = self.table_mut();
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

    fn send(
        &mut self,
        this: Resource<udp::OutgoingDatagramStream>,
        datagrams: Vec<udp::OutgoingDatagram>,
    ) -> SocketResult<u64> {
        fn send_one(
            stream: &OutgoingStreamResource,
            datagram: &udp::OutgoingDatagram,
        ) -> SocketResult<()> {
            if datagram.data.len() > MAX_UDP_DATAGRAM_SIZE {
                return Err(ErrorCode::DatagramTooLarge.into());
            }

            let provided_addr = datagram.remote_address.map(SocketAddr::from);
            match (stream.remote_address, provided_addr) {
                (None, Some(target)) => {
                    stream.inner.send(&datagram.data, target)?;
                }
                (Some(target), None) => {
                    stream.inner.send(&datagram.data, target)?;
                }
                (Some(connected_addr), Some(provided_addr)) if connected_addr == provided_addr => {
                    stream.inner.send(&datagram.data, provided_addr)?;
                }
                _ => return Err(ErrorCode::InvalidArgument.into()),
            }

            Ok(())
        }

        let table = self.table_mut();
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
            match send_one(stream, &datagram) {
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
        crate::preview2::poll::subscribe(self.table_mut(), this)
    }

    fn drop(&mut self, this: Resource<udp::OutgoingDatagramStream>) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

pub struct IncomingStreamResource {
    inner: IncomingDatagramStream,
    /// If this has a value, the stream is "connected".
    pub(crate) remote_address: Option<SocketAddr>,
}

impl IncomingStreamResource {
    fn new(inner: IncomingDatagramStream, remote_address: Option<SocketAddr>) -> Self {
        Self {
            inner,
            remote_address,
        }
    }
}

#[async_trait]
impl Subscribe for IncomingStreamResource {
    async fn ready(&mut self) {
        self.inner.ready().await;
    }
}

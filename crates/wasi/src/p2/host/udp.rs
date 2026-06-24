use crate::p2::bindings::sockets::network::{ErrorCode, IpAddressFamily, IpSocketAddress, Network};
use crate::p2::bindings::sockets::udp;
use crate::p2::udp::{AsyncOperation, IncomingDatagramStream, OutgoingDatagramStream};
use crate::p2::{Pollable, SocketError, SocketResult, UdpSocket};
use crate::sockets::{SocketAddressFamily, WasiSocketsCtxView};
use async_trait::async_trait;
use std::future::poll_fn;
use std::net::SocketAddr;
use std::task::{Context, Poll, Waker};
use wasmtime::component::Resource;
use wasmtime::format_err;
use wasmtime_wasi_io::poll::DynPollable;

const MAX_DATAGRAMS: usize = 16;

impl udp::Host for WasiSocketsCtxView<'_> {}

impl udp::HostUdpSocket for WasiSocketsCtxView<'_> {
    async fn start_bind(
        &mut self,
        this: Resource<UdpSocket>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        // The network resource itself represents the capability to use this
        // method, so we need to check its validity. Other than that, we have no
        // use for it.
        _ = self.table.get(&network)?;

        let local_address = SocketAddr::from(local_address);
        let socket = self.table.get_mut(&this)?;
        if socket.in_progress_operation.is_some() {
            return Err(ErrorCode::ConcurrencyConflict.into());
        }

        socket
            .get_mut()
            .ok_or(ErrorCode::InvalidState)?
            .bind(local_address)
            .await?;

        socket.in_progress_operation = Some(AsyncOperation::Bind);
        Ok(())
    }

    fn finish_bind(&mut self, this: Resource<UdpSocket>) -> SocketResult<()> {
        let socket = self.table.get_mut(&this)?;
        if socket.in_progress_operation != Some(AsyncOperation::Bind) {
            return Err(ErrorCode::NotInProgress.into());
        };
        socket.in_progress_operation = None;
        Ok(())
    }

    async fn stream(
        &mut self,
        this: Resource<UdpSocket>,
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
            return Err(SocketError::trap(format_err!(
                "UDP streams not dropped yet"
            )));
        }

        let socket = self.table.get_mut(&this)?;
        let inner = socket
            .get_mut()
            .ok_or(SocketError::trap(wasmtime::error::format_err!(
                "`connect` needs exclusive access"
            )))?;

        if !inner.is_bound() {
            // In WASI 0.2, sockets had to be explicitly bound before connecting.
            return Err(ErrorCode::InvalidState.into());
        }

        if let Some(connect_addr) = remote_address {
            inner.connect(connect_addr.into()).await?;
        } else if inner.is_connected() {
            inner.disconnect()?;
        }
        let incoming_stream = IncomingDatagramStream::new(socket.inner.clone());
        let outgoing_stream = OutgoingDatagramStream::new(socket.inner.clone());
        Ok((
            self.table.push_child(incoming_stream, &this)?,
            self.table.push_child(outgoing_stream, &this)?,
        ))
    }

    fn local_address(&mut self, this: Resource<UdpSocket>) -> SocketResult<IpSocketAddress> {
        let mut socket = self.table.get(&this)?.lock();
        Ok(socket.local_address()?.into())
    }

    fn remote_address(&mut self, this: Resource<UdpSocket>) -> SocketResult<IpSocketAddress> {
        let mut socket = self.table.get(&this)?.lock();
        Ok(socket.remote_address()?.into())
    }

    fn address_family(
        &mut self,
        this: Resource<UdpSocket>,
    ) -> Result<IpAddressFamily, wasmtime::Error> {
        let socket = self.table.get(&this)?.lock();
        Ok(socket.address_family().into())
    }

    fn unicast_hop_limit(&mut self, this: Resource<UdpSocket>) -> SocketResult<u8> {
        let socket = self.table.get(&this)?.lock();
        Ok(socket.unicast_hop_limit()?)
    }

    fn set_unicast_hop_limit(&mut self, this: Resource<UdpSocket>, value: u8) -> SocketResult<()> {
        let socket = self.table.get(&this)?.lock();
        socket.set_unicast_hop_limit(value)?;
        Ok(())
    }

    fn receive_buffer_size(&mut self, this: Resource<UdpSocket>) -> SocketResult<u64> {
        let socket = self.table.get(&this)?.lock();
        Ok(socket.receive_buffer_size()?)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<UdpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let socket = self.table.get(&this)?.lock();
        socket.set_receive_buffer_size(value)?;
        Ok(())
    }

    fn send_buffer_size(&mut self, this: Resource<UdpSocket>) -> SocketResult<u64> {
        let socket = self.table.get(&this)?.lock();
        Ok(socket.send_buffer_size()?)
    }

    fn set_send_buffer_size(&mut self, this: Resource<UdpSocket>, value: u64) -> SocketResult<()> {
        let socket = self.table.get(&this)?.lock();
        socket.set_send_buffer_size(value)?;
        Ok(())
    }

    fn subscribe(&mut self, this: Resource<UdpSocket>) -> wasmtime::Result<Resource<DynPollable>> {
        wasmtime_wasi_io::poll::subscribe(self.table, this)
    }

    fn drop(&mut self, this: Resource<UdpSocket>) -> Result<(), wasmtime::Error> {
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
        let stream = self.table.get_mut(&this)?;
        let max_results: usize = max_results
            .try_into()
            .unwrap_or(usize::MAX)
            .min(MAX_DATAGRAMS);
        if max_results == 0 {
            return Ok(vec![]);
        }

        let mut datagrams = vec![];
        let mut sum = 0;

        while datagrams.len() < max_results && sum < crate::MAX_READ_SIZE_ALLOC {
            match stream.try_recv() {
                Err(ErrorCode::WouldBlock) => break,
                Ok((data, remote_addr)) => {
                    sum += 1 + data.len();
                    datagrams.push(udp::IncomingDatagram {
                        data,
                        remote_address: remote_addr.into(),
                    });
                }
                Err(_) if datagrams.len() > 0 => break,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        Ok(datagrams)
    }

    fn subscribe(
        &mut self,
        this: Resource<udp::IncomingDatagramStream>,
    ) -> wasmtime::Result<Resource<DynPollable>> {
        wasmtime_wasi_io::poll::subscribe(self.table, this)
    }

    fn drop(&mut self, this: Resource<udp::IncomingDatagramStream>) -> Result<(), wasmtime::Error> {
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
        poll_fn(|cx| self.poll_recv_ready(cx)).await
    }
}

impl udp::HostOutgoingDatagramStream for WasiSocketsCtxView<'_> {
    fn check_send(&mut self, this: Resource<udp::OutgoingDatagramStream>) -> SocketResult<u64> {
        let stream = self.table.get_mut(&this)?;

        let count = if let Poll::Ready(()) =
            stream.poll_send_ready(&mut Context::from_waker(Waker::noop()))
        {
            // We don't know how many Tokio will accept, so we make up a
            // reasonable number here.  If we're wrong and `send` returns
            // `Ok(0)`, the guest will just have to deal with that, e.g. by
            // looping or returning `EWOULDBLOCK`.
            MAX_DATAGRAMS
        } else {
            0
        };

        stream.check_send_permit_count = count;

        Ok(count.try_into().unwrap())
    }

    fn send(
        &mut self,
        this: Resource<udp::OutgoingDatagramStream>,
        datagrams: Vec<udp::OutgoingDatagram>,
    ) -> SocketResult<u64> {
        let stream = self.table.get_mut(&this)?;

        if datagrams.is_empty() {
            return Ok(0);
        }

        if datagrams.len() > stream.check_send_permit_count {
            return Err(SocketError::trap(wasmtime::format_err!(
                "unpermitted: argument exceeds permitted size"
            )));
        }

        // Reset permit. From the WIT spec:
        // > Each call to `send` must be permitted by a preceding `check-send`.
        stream.check_send_permit_count = 0;

        let mut count = 0;

        for datagram in datagrams {
            match stream.try_send(datagram.data, datagram.remote_address.map(SocketAddr::from)) {
                Err(ErrorCode::WouldBlock) => break,
                Ok(()) => count += 1,
                Err(_) if count > 0 => {
                    // WIT: "If at least one datagram has been sent successfully, this function never returns an error."
                    break;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        Ok(count)
    }

    fn subscribe(
        &mut self,
        this: Resource<udp::OutgoingDatagramStream>,
    ) -> wasmtime::Result<Resource<DynPollable>> {
        wasmtime_wasi_io::poll::subscribe(self.table, this)
    }

    async fn drop(
        &mut self,
        this: Resource<udp::OutgoingDatagramStream>,
    ) -> Result<(), wasmtime::Error> {
        let mut stream = self.table.delete(this)?;
        // Prevent silently dropping already-acknowledged sends by waiting for
        // any in-progress background send to complete. This may block
        // the guest, but that's the price we pay for implementing the
        // readiness-based P2 API in terms of the completion-based P3 API.
        std::future::poll_fn(|cx| stream.poll_send_ready(cx)).await;
        drop(stream);
        Ok(())
    }
}

#[async_trait]
impl Pollable for OutgoingDatagramStream {
    async fn ready(&mut self) {
        poll_fn(|cx| self.poll_send_ready(cx)).await
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
        SocketError, UdpSocket,
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
            AsyncHostOutgoingDatagramStream::send(self, self_, datagrams)
        }

        fn subscribe(
            &mut self,
            self_: Resource<OutgoingDatagramStream>,
        ) -> wasmtime::Result<Resource<Pollable>> {
            AsyncHostOutgoingDatagramStream::subscribe(self, self_)
        }

        fn drop(&mut self, rep: Resource<OutgoingDatagramStream>) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHostOutgoingDatagramStream::drop(self, rep).await })
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

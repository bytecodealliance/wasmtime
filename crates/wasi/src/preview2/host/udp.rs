use std::net::SocketAddr;

use crate::preview2::{
    bindings::{
        sockets::network::{ErrorCode, IpAddressFamily, IpSocketAddress, Network},
        sockets::udp,
    },
    udp::{UdpSocketInner, UdpState},
};
use crate::preview2::{Pollable, SocketResult, WasiView};
use cap_net_ext::{AddressFamily, PoolExt};
use io_lifetimes::AsSocketlike;
use rustix::io::Errno;
use rustix::net::sockopt;
use wasmtime::component::Resource;

/// Theoretical maximum byte size of a UDP datagram, the real limit is lower,
/// but we do not account for e.g. the transport layer here for simplicity.
/// In practice, datagrams are typically less than 1500 bytes.
const MAX_UDP_DATAGRAM_SIZE: usize = 65535;

impl<T: WasiView> udp::Host for T {}

impl<T: WasiView> udp::HostUdpSocket for T {
    fn start_bind(
        &mut self,
        this: Resource<udp::UdpSocket>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let table = self.table();
        let mut socket = table.get(&this)?.inner.lock().unwrap();
        let network = table.get(&network)?;
        let local_address: SocketAddr = local_address.into();

        match socket.udp_state {
            UdpState::Default => {}
            UdpState::BindStarted | UdpState::Connecting(..) => {
                return Err(ErrorCode::ConcurrencyConflict.into())
            }
            UdpState::Bound | UdpState::Connected(..) => return Err(ErrorCode::InvalidState.into()),
        }

        let binder = network.pool.udp_binder(local_address)?;

        // Perform the OS bind call.
        binder.bind_existing_udp_socket(
            &*socket
                .udp_socket()
                .as_socketlike_view::<cap_std::net::UdpSocket>(),
        )?;

        socket.udp_state = UdpState::BindStarted;

        Ok(())
    }

    fn finish_bind(
        &mut self,
        this: Resource<udp::UdpSocket>,
    ) -> SocketResult<(
        Resource<udp::InboundDatagramStream>,
        Resource<udp::OutboundDatagramStream>,
    )> {
        let table = self.table_mut();
        let outer = table.get(&this)?;
        {
            let mut socket = outer.inner.lock().unwrap();

            match socket.udp_state {
                UdpState::BindStarted => {}
                _ => return Err(ErrorCode::NotInProgress.into()),
            }

            socket.udp_state = UdpState::Bound;
        }

        let inbound_stream = outer.new_inbound_stream();
        let outbound_stream = outer.new_outbound_stream();

        Ok((
            self.table_mut().push_child(inbound_stream, &this)?,
            self.table_mut().push_child(outbound_stream, &this)?,
        ))
    }

    fn start_connect(
        &mut self,
        this: Resource<udp::UdpSocket>,
        remote_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let table = self.table();
        let mut socket = table.get(&this)?.inner.lock().unwrap();
        let remote_address: SocketAddr = remote_address.into();

        match socket.udp_state {
            UdpState::Default | UdpState::Bound => {}
            UdpState::BindStarted | UdpState::Connecting(..) => {
                return Err(ErrorCode::ConcurrencyConflict.into())
            }
            UdpState::Connected(..) => return Err(ErrorCode::InvalidState.into()),
        }

        rustix::net::connect(socket.udp_socket(), &remote_address)?;

        socket.udp_state = UdpState::Connecting(remote_address);
        Ok(())
    }

    fn finish_connect(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<()> {
        let table = self.table();
        let mut socket = table.get(&this)?.inner.lock().unwrap();

        match socket.udp_state {
            UdpState::Connecting(addr) => {
                socket.udp_state = UdpState::Connected(addr);
                Ok(())
            }
            _ => Err(ErrorCode::NotInProgress.into()),
        }
    }

    fn local_address(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?.inner.lock().unwrap();
        let addr = socket
            .udp_socket()
            .as_socketlike_view::<std::net::UdpSocket>()
            .local_addr()?;
        Ok(addr.into())
    }

    fn remote_address(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<IpSocketAddress> {
        let table = self.table();
        let socket = table.get(&this)?.inner.lock().unwrap();
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
        let socket = table.get(&this)?.inner.lock().unwrap();
        match socket.family {
            AddressFamily::Ipv4 => Ok(IpAddressFamily::Ipv4),
            AddressFamily::Ipv6 => Ok(IpAddressFamily::Ipv6),
        }
    }

    fn ipv6_only(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<bool> {
        let table = self.table();
        let socket = table.get(&this)?.inner.lock().unwrap();
        Ok(sockopt::get_ipv6_v6only(socket.udp_socket())?)
    }

    fn set_ipv6_only(&mut self, this: Resource<udp::UdpSocket>, value: bool) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?.inner.lock().unwrap();
        Ok(sockopt::set_ipv6_v6only(socket.udp_socket(), value)?)
    }

    fn unicast_hop_limit(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<u8> {
        let table = self.table();
        let socket = table.get(&this)?.inner.lock().unwrap();

        // We don't track whether the socket is IPv4 or IPv6 so try one and
        // fall back to the other.
        match sockopt::get_ipv6_unicast_hops(socket.udp_socket()) {
            Ok(value) => Ok(value),
            Err(Errno::NOPROTOOPT) => {
                let value = sockopt::get_ip_ttl(socket.udp_socket())?;
                let value = value.try_into().unwrap();
                Ok(value)
            }
            Err(err) => Err(err.into()),
        }
    }

    fn set_unicast_hop_limit(
        &mut self,
        this: Resource<udp::UdpSocket>,
        value: u8,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?.inner.lock().unwrap();

        // We don't track whether the socket is IPv4 or IPv6 so try one and
        // fall back to the other.
        match sockopt::set_ipv6_unicast_hops(socket.udp_socket(), Some(value)) {
            Ok(()) => Ok(()),
            Err(Errno::NOPROTOOPT) => Ok(sockopt::set_ip_ttl(socket.udp_socket(), value.into())?),
            Err(err) => Err(err.into()),
        }
    }

    fn receive_buffer_size(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?.inner.lock().unwrap();
        Ok(sockopt::get_socket_recv_buffer_size(socket.udp_socket())? as u64)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<udp::UdpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?.inner.lock().unwrap();
        let value = value.try_into().map_err(|_| ErrorCode::OutOfMemory)?;
        Ok(sockopt::set_socket_recv_buffer_size(
            socket.udp_socket(),
            value,
        )?)
    }

    fn send_buffer_size(&mut self, this: Resource<udp::UdpSocket>) -> SocketResult<u64> {
        let table = self.table();
        let socket = table.get(&this)?.inner.lock().unwrap();
        Ok(sockopt::get_socket_send_buffer_size(socket.udp_socket())? as u64)
    }

    fn set_send_buffer_size(
        &mut self,
        this: Resource<udp::UdpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let table = self.table();
        let socket = table.get(&this)?.inner.lock().unwrap();
        let value = value.try_into().map_err(|_| ErrorCode::OutOfMemory)?;
        Ok(sockopt::set_socket_send_buffer_size(
            socket.udp_socket(),
            value,
        )?)
    }

    fn subscribe(&mut self, this: Resource<udp::UdpSocket>) -> anyhow::Result<Resource<Pollable>> {
        crate::preview2::poll::subscribe(self.table_mut(), this)
    }

    fn drop(&mut self, this: Resource<udp::UdpSocket>) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

impl<T: WasiView> udp::HostInboundDatagramStream for T {
    fn receive(
        &mut self,
        this: Resource<udp::InboundDatagramStream>,
        max_results: u64,
    ) -> SocketResult<Vec<udp::InboundDatagram>> {
        fn recv_one(socket: &UdpSocketInner) -> SocketResult<udp::InboundDatagram> {
            let mut buf = [0; MAX_UDP_DATAGRAM_SIZE];
            let (size, received_addr) = socket.udp_socket().try_recv_from(&mut buf)?;

            match socket.remote_address() {
                Some(connected_addr) if connected_addr != received_addr => {
                    // Normally, this should have already been checked for us by the OS.
                    // Drop message...
                    return Err(ErrorCode::WouldBlock.into());
                }
                _ => {}
            }

            // FIXME: check permission to receive from `received_addr`.
            Ok(udp::InboundDatagram {
                data: buf[..size].into(),
                remote_address: received_addr.into(),
            })
        }

        let table = self.table();
        let socket = table.get(&this)?.inner.lock().unwrap();

        if max_results == 0 {
            return Ok(vec![]);
        }

        let mut datagrams = vec![];

        for _ in 0..max_results {
            match recv_one(&socket) {
                Ok(datagram) => {
                    datagrams.push(datagram);
                }
                Err(_e) if datagrams.len() > 0 => {
                    return Ok(datagrams);
                }
                Err(e) => return Err(e),
            }
        }

        Ok(datagrams)
    }

    fn subscribe(
        &mut self,
        this: Resource<udp::InboundDatagramStream>,
    ) -> anyhow::Result<Resource<Pollable>> {
        crate::preview2::poll::subscribe(self.table_mut(), this)
    }

    fn drop(&mut self, this: Resource<udp::InboundDatagramStream>) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

impl<T: WasiView> udp::HostOutboundDatagramStream for T {
    fn send(
        &mut self,
        this: Resource<udp::OutboundDatagramStream>,
        datagrams: Vec<udp::OutboundDatagram>,
    ) -> SocketResult<u64> {
        fn send_one(socket: &UdpSocketInner, datagram: &udp::OutboundDatagram) -> SocketResult<()> {
            let provided_addr = datagram.remote_address.map(SocketAddr::from);
            let addr = match (socket.remote_address(), provided_addr) {
                (None, Some(addr)) => addr,
                (Some(addr), None) => addr,
                (Some(connected_addr), Some(provided_addr)) if connected_addr == provided_addr => {
                    connected_addr
                }
                _ => return Err(ErrorCode::InvalidArgument.into()),
            };

            // FIXME: check permission to send to `addr`.
            socket.udp_socket().try_send_to(&datagram.data, addr)?;

            Ok(())
        }

        let table = self.table();
        let socket = table.get(&this)?.inner.lock().unwrap();

        if datagrams.is_empty() {
            return Ok(0);
        }

        let mut count = 0;

        for datagram in datagrams {
            match send_one(&socket, &datagram) {
                Ok(_size) => count += 1,
                Err(_e) if count > 0 => {
                    // WIT: "If at least one datagram has been sent successfully, this function never returns an error."
                    return Ok(count);
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(count)
    }

    fn subscribe(
        &mut self,
        this: Resource<udp::OutboundDatagramStream>,
    ) -> anyhow::Result<Resource<Pollable>> {
        crate::preview2::poll::subscribe(self.table_mut(), this)
    }

    fn drop(&mut self, this: Resource<udp::OutboundDatagramStream>) -> Result<(), anyhow::Error> {
        let table = self.table_mut();

        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

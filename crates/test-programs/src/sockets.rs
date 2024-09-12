use crate::wasi::clocks::monotonic_clock;
use crate::wasi::io::poll::{self, Pollable};
use crate::wasi::io::streams::{InputStream, OutputStream, StreamError};
use crate::wasi::random;
use crate::wasi::sockets::instance_network;
use crate::wasi::sockets::ip_name_lookup;
use crate::wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
    Network,
};
use crate::wasi::sockets::tcp::TcpSocket;
use crate::wasi::sockets::udp::{
    IncomingDatagram, IncomingDatagramStream, OutgoingDatagram, OutgoingDatagramStream, UdpSocket,
};
use crate::wasi::sockets::{tcp_create_socket, udp_create_socket};
use std::ops::Range;

const TIMEOUT_NS: u64 = 1_000_000_000;

impl Pollable {
    pub fn block_until(&self, timeout: &Pollable) -> Result<(), ErrorCode> {
        let ready = poll::poll(&[self, timeout]);
        assert!(ready.len() > 0);
        match ready[0] {
            0 => Ok(()),
            1 => Err(ErrorCode::Timeout),
            _ => unreachable!(),
        }
    }
}

impl InputStream {
    pub fn blocking_read_to_end(&self) -> Result<Vec<u8>, crate::wasi::io::error::Error> {
        let mut data = vec![];
        loop {
            match self.blocking_read(1024 * 1024) {
                Ok(chunk) => data.extend(chunk),
                Err(StreamError::Closed) => return Ok(data),
                Err(StreamError::LastOperationFailed(e)) => return Err(e),
            }
        }
    }
}

impl OutputStream {
    pub fn blocking_write_util(&self, mut bytes: &[u8]) -> Result<(), StreamError> {
        let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
        let pollable = self.subscribe();

        while !bytes.is_empty() {
            pollable.block_until(&timeout).expect("write timed out");

            let permit = self.check_write()?;

            let len = bytes.len().min(permit as usize);
            let (chunk, rest) = bytes.split_at(len);

            self.write(chunk)?;

            self.blocking_flush()?;

            bytes = rest;
        }
        Ok(())
    }
}

impl Network {
    pub fn default() -> Network {
        instance_network::instance_network()
    }

    pub fn blocking_resolve_addresses(&self, name: &str) -> Result<Vec<IpAddress>, ErrorCode> {
        let stream = ip_name_lookup::resolve_addresses(&self, name)?;

        let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
        let pollable = stream.subscribe();

        let mut addresses = vec![];

        loop {
            match stream.resolve_next_address() {
                Ok(Some(addr)) => {
                    addresses.push(addr);
                }
                Ok(None) => match addresses[..] {
                    [] => return Err(ErrorCode::NameUnresolvable),
                    _ => return Ok(addresses),
                },
                Err(ErrorCode::WouldBlock) => {
                    pollable.block_until(&timeout)?;
                }
                Err(err) => return Err(err),
            }
        }
    }

    /// Same as `Network::blocking_resolve_addresses` but ignores post validation errors
    ///
    /// The ignored error codes signal that the input passed validation
    /// and a lookup was actually attempted, but failed. These are ignored to
    /// make the CI tests less flaky.
    pub fn permissive_blocking_resolve_addresses(
        &self,
        name: &str,
    ) -> Result<Vec<IpAddress>, ErrorCode> {
        match self.blocking_resolve_addresses(name) {
            Err(ErrorCode::NameUnresolvable | ErrorCode::TemporaryResolverFailure) => Ok(vec![]),
            r => r,
        }
    }
}

impl TcpSocket {
    pub fn new(address_family: IpAddressFamily) -> Result<TcpSocket, ErrorCode> {
        tcp_create_socket::create_tcp_socket(address_family)
    }

    pub fn blocking_bind(
        &self,
        network: &Network,
        local_address: IpSocketAddress,
    ) -> Result<(), ErrorCode> {
        let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
        let sub = self.subscribe();

        self.start_bind(&network, local_address)?;

        loop {
            match self.finish_bind() {
                Err(ErrorCode::WouldBlock) => sub.block_until(&timeout)?,
                result => return result,
            }
        }
    }

    pub fn blocking_listen(&self) -> Result<(), ErrorCode> {
        let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
        let sub = self.subscribe();

        self.start_listen()?;

        loop {
            match self.finish_listen() {
                Err(ErrorCode::WouldBlock) => sub.block_until(&timeout)?,
                result => return result,
            }
        }
    }

    pub fn blocking_connect(
        &self,
        network: &Network,
        remote_address: IpSocketAddress,
    ) -> Result<(InputStream, OutputStream), ErrorCode> {
        let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
        let sub = self.subscribe();

        self.start_connect(&network, remote_address)?;

        loop {
            match self.finish_connect() {
                Err(ErrorCode::WouldBlock) => sub.block_until(&timeout)?,
                result => return result,
            }
        }
    }

    pub fn blocking_accept(&self) -> Result<(TcpSocket, InputStream, OutputStream), ErrorCode> {
        let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
        let sub = self.subscribe();

        loop {
            match self.accept() {
                Err(ErrorCode::WouldBlock) => sub.block_until(&timeout)?,
                result => return result,
            }
        }
    }
}

impl UdpSocket {
    pub fn new(address_family: IpAddressFamily) -> Result<UdpSocket, ErrorCode> {
        udp_create_socket::create_udp_socket(address_family)
    }

    pub fn blocking_bind(
        &self,
        network: &Network,
        local_address: IpSocketAddress,
    ) -> Result<(), ErrorCode> {
        let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
        let sub = self.subscribe();

        self.start_bind(&network, local_address)?;

        loop {
            match self.finish_bind() {
                Err(ErrorCode::WouldBlock) => sub.block_until(&timeout)?,
                result => return result,
            }
        }
    }

    pub fn blocking_bind_unspecified(&self, network: &Network) -> Result<(), ErrorCode> {
        let ip = IpAddress::new_unspecified(self.address_family());
        let port = 0;

        self.blocking_bind(network, IpSocketAddress::new(ip, port))
    }
}

impl OutgoingDatagramStream {
    fn blocking_check_send(&self, timeout: &Pollable) -> Result<u64, ErrorCode> {
        let sub = self.subscribe();

        loop {
            match self.check_send() {
                Ok(0) => sub.block_until(timeout)?,
                result => return result,
            }
        }
    }

    pub fn blocking_send(&self, mut datagrams: &[OutgoingDatagram]) -> Result<(), ErrorCode> {
        let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);

        while !datagrams.is_empty() {
            let permit = self.blocking_check_send(&timeout)?;
            let chunk_len = datagrams.len().min(permit as usize);
            match self.send(&datagrams[..chunk_len]) {
                Ok(0) => {}
                Ok(packets_sent) => {
                    let packets_sent = packets_sent as usize;
                    datagrams = &datagrams[packets_sent..];
                }
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }
}

impl IncomingDatagramStream {
    pub fn blocking_receive(&self, count: Range<u64>) -> Result<Vec<IncomingDatagram>, ErrorCode> {
        let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
        let pollable = self.subscribe();
        let mut datagrams = vec![];

        loop {
            match self.receive(count.end - datagrams.len() as u64) {
                Ok(mut chunk) => {
                    datagrams.append(&mut chunk);

                    if datagrams.len() >= count.start as usize {
                        return Ok(datagrams);
                    } else {
                        pollable.block_until(&timeout)?;
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }
}

impl IpAddress {
    pub const IPV4_BROADCAST: IpAddress = IpAddress::Ipv4((255, 255, 255, 255));

    pub const IPV4_LOOPBACK: IpAddress = IpAddress::Ipv4((127, 0, 0, 1));
    pub const IPV6_LOOPBACK: IpAddress = IpAddress::Ipv6((0, 0, 0, 0, 0, 0, 0, 1));

    pub const IPV4_UNSPECIFIED: IpAddress = IpAddress::Ipv4((0, 0, 0, 0));
    pub const IPV6_UNSPECIFIED: IpAddress = IpAddress::Ipv6((0, 0, 0, 0, 0, 0, 0, 0));

    pub const IPV4_MAPPED_LOOPBACK: IpAddress =
        IpAddress::Ipv6((0, 0, 0, 0, 0, 0xFFFF, 0x7F00, 0x0001));

    pub const fn new_loopback(family: IpAddressFamily) -> IpAddress {
        match family {
            IpAddressFamily::Ipv4 => Self::IPV4_LOOPBACK,
            IpAddressFamily::Ipv6 => Self::IPV6_LOOPBACK,
        }
    }

    pub const fn new_unspecified(family: IpAddressFamily) -> IpAddress {
        match family {
            IpAddressFamily::Ipv4 => Self::IPV4_UNSPECIFIED,
            IpAddressFamily::Ipv6 => Self::IPV6_UNSPECIFIED,
        }
    }

    pub const fn family(&self) -> IpAddressFamily {
        match self {
            IpAddress::Ipv4(_) => IpAddressFamily::Ipv4,
            IpAddress::Ipv6(_) => IpAddressFamily::Ipv6,
        }
    }
}

impl PartialEq for IpAddress {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ipv4(left), Self::Ipv4(right)) => left == right,
            (Self::Ipv6(left), Self::Ipv6(right)) => left == right,
            _ => false,
        }
    }
}

impl IpSocketAddress {
    pub const fn new(ip: IpAddress, port: u16) -> IpSocketAddress {
        match ip {
            IpAddress::Ipv4(addr) => IpSocketAddress::Ipv4(Ipv4SocketAddress {
                port: port,
                address: addr,
            }),
            IpAddress::Ipv6(addr) => IpSocketAddress::Ipv6(Ipv6SocketAddress {
                port: port,
                address: addr,
                flow_info: 0,
                scope_id: 0,
            }),
        }
    }

    pub const fn ip(&self) -> IpAddress {
        match self {
            IpSocketAddress::Ipv4(addr) => IpAddress::Ipv4(addr.address),
            IpSocketAddress::Ipv6(addr) => IpAddress::Ipv6(addr.address),
        }
    }

    pub const fn port(&self) -> u16 {
        match self {
            IpSocketAddress::Ipv4(addr) => addr.port,
            IpSocketAddress::Ipv6(addr) => addr.port,
        }
    }

    pub const fn family(&self) -> IpAddressFamily {
        match self {
            IpSocketAddress::Ipv4(_) => IpAddressFamily::Ipv4,
            IpSocketAddress::Ipv6(_) => IpAddressFamily::Ipv6,
        }
    }
}

impl PartialEq for Ipv4SocketAddress {
    fn eq(&self, other: &Self) -> bool {
        self.port == other.port && self.address == other.address
    }
}

impl PartialEq for Ipv6SocketAddress {
    fn eq(&self, other: &Self) -> bool {
        self.port == other.port
            && self.flow_info == other.flow_info
            && self.address == other.address
            && self.scope_id == other.scope_id
    }
}

impl PartialEq for IpSocketAddress {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ipv4(l0), Self::Ipv4(r0)) => l0 == r0,
            (Self::Ipv6(l0), Self::Ipv6(r0)) => l0 == r0,
            _ => false,
        }
    }
}

fn generate_random_u16(range: Range<u16>) -> u16 {
    let start = range.start as u64;
    let end = range.end as u64;
    let port = start + (random::random::get_random_u64() % (end - start));
    port as u16
}

/// Execute the inner function with a randomly generated port.
/// To prevent random failures, we make a few attempts before giving up.
pub fn attempt_random_port<F>(
    local_address: IpAddress,
    mut f: F,
) -> Result<IpSocketAddress, ErrorCode>
where
    F: FnMut(IpSocketAddress) -> Result<(), ErrorCode>,
{
    const MAX_ATTEMPTS: u32 = 10;
    let mut i = 0;
    loop {
        i += 1;

        let port: u16 = generate_random_u16(1024..u16::MAX);
        let sock_addr = IpSocketAddress::new(local_address, port);

        match f(sock_addr) {
            Ok(_) => return Ok(sock_addr),
            Err(e) if i >= MAX_ATTEMPTS => return Err(e),
            // Try again if the port is already taken. This can sometimes show up as `AccessDenied` on Windows.
            Err(ErrorCode::AddressInUse | ErrorCode::AccessDenied) => {}
            Err(e) => return Err(e),
        }
    }
}

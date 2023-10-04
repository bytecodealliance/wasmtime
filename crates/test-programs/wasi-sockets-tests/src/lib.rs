wit_bindgen::generate!("test-command-with-sockets" in "../../wasi/wit");

use wasi::io::poll::{self, Pollable};
use wasi::io::streams::{InputStream, OutputStream, StreamError};
use wasi::sockets::instance_network;
use wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
    Network,
};
use wasi::sockets::tcp::TcpSocket;
use wasi::sockets::tcp_create_socket;

impl Pollable {
    pub fn wait(&self) {
        poll::poll_one(self);
    }
}

impl OutputStream {
    pub fn blocking_write_util(&self, mut bytes: &[u8]) -> Result<(), StreamError> {
        let pollable = self.subscribe();

        while !bytes.is_empty() {
            pollable.wait();

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
        let sub = self.subscribe();

        self.start_bind(&network, local_address)?;

        loop {
            match self.finish_bind() {
                Err(ErrorCode::WouldBlock) => sub.wait(),
                result => return result,
            }
        }
    }

    pub fn blocking_listen(&self) -> Result<(), ErrorCode> {
        let sub = self.subscribe();

        self.start_listen()?;

        loop {
            match self.finish_listen() {
                Err(ErrorCode::WouldBlock) => sub.wait(),
                result => return result,
            }
        }
    }

    pub fn blocking_connect(
        &self,
        network: &Network,
        remote_address: IpSocketAddress,
    ) -> Result<(InputStream, OutputStream), ErrorCode> {
        let sub = self.subscribe();

        self.start_connect(&network, remote_address)?;

        loop {
            match self.finish_connect() {
                Err(ErrorCode::WouldBlock) => sub.wait(),
                result => return result,
            }
        }
    }

    pub fn blocking_accept(&self) -> Result<(TcpSocket, InputStream, OutputStream), ErrorCode> {
        let sub = self.subscribe();

        loop {
            match self.accept() {
                Err(ErrorCode::WouldBlock) => sub.wait(),
                result => return result,
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

wit_bindgen::generate!("test-command-with-sockets" in "../../wasi/wit");

use wasi::io::poll::{self, Pollable};
use wasi::io::streams::{InputStream, OutputStream, StreamError};
use wasi::sockets::instance_network;
use wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
    Network,
};
use wasi::sockets::tcp::TcpSocket;
use wasi::sockets::{network, tcp_create_socket, udp, udp_create_socket};

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

pub fn example_body_udp(net: udp::Network, sock: udp::UdpSocket, family: network::IpAddressFamily) {
    let first_message = b"Hello, world!";
    let second_message = b"Greetings, planet!";

    let sub = sock.subscribe();

    let addr = sock.local_address().unwrap();

    let client = udp_create_socket::create_udp_socket(family).unwrap();
    let client_sub = client.subscribe();

    client.start_connect(&net, addr).unwrap();
    poll::poll_one(&client_sub);
    client.finish_connect().unwrap();

    let _client_addr = client.local_address().unwrap();

    let n = client
        .send(&[
            udp::Datagram {
                data: vec![],
                remote_address: addr,
            },
            udp::Datagram {
                data: first_message.to_vec(),
                remote_address: addr,
            },
        ])
        .unwrap();
    assert_eq!(n, 2);

    drop(client_sub);
    drop(client);

    poll::poll_one(&sub);
    let datagrams = sock.receive(2).unwrap();
    let mut datagrams = datagrams.into_iter();
    let (first, second) = match (datagrams.next(), datagrams.next(), datagrams.next()) {
        (Some(first), Some(second), None) => (first, second),
        (Some(_first), None, None) => panic!("only one datagram received"),
        (None, None, None) => panic!("no datagrams received"),
        _ => panic!("invalid datagram sequence received"),
    };

    assert!(first.data.is_empty());

    // TODO: Verify the `remote_address`
    //assert_eq!(first.remote_address, client_addr);

    // Check that we sent and recieved our message!
    assert_eq!(second.data, first_message); // Not guaranteed to work but should work in practice.

    // TODO: Verify the `remote_address`
    //assert_eq!(second.remote_address, client_addr);

    // Another client
    let client = udp_create_socket::create_udp_socket(family).unwrap();
    let client_sub = client.subscribe();

    client.start_connect(&net, addr).unwrap();
    poll::poll_one(&client_sub);
    client.finish_connect().unwrap();

    let n = client
        .send(&[udp::Datagram {
            data: second_message.to_vec(),
            remote_address: addr,
        }])
        .unwrap();
    assert_eq!(n, 1);

    drop(client_sub);
    drop(client);

    poll::poll_one(&sub);
    let datagrams = sock.receive(2).unwrap();
    let mut datagrams = datagrams.into_iter();
    let first = match (datagrams.next(), datagrams.next()) {
        (Some(first), None) => first,
        (None, None) => panic!("no datagrams received"),
        _ => panic!("invalid datagram sequence received"),
    };

    // Check that we sent and recieved our message!
    assert_eq!(first.data, second_message); // Not guaranteed to work but should work in practice.
}

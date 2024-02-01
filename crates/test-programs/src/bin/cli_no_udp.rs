//! This test assumes that it will be run without udp support enabled
use test_programs::wasi::sockets::{
    network::IpAddress,
    udp::{ErrorCode, IpAddressFamily, IpSocketAddress, Network, UdpSocket},
};

fn main() {
    let net = Network::default();
    let family = IpAddressFamily::Ipv4;
    let remote1 = IpSocketAddress::new(IpAddress::new_loopback(family), 4321);
    let sock = UdpSocket::new(family).unwrap();

    let bind = sock.blocking_bind(&net, remote1);
    eprintln!("Result of binding: {bind:?}");
    assert!(matches!(bind, Err(ErrorCode::AccessDenied)));
}

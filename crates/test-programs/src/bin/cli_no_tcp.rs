//! This test assumes that it will be run without tcp support enabled
use test_programs::wasi::sockets::{
    network::IpAddress,
    tcp::{ErrorCode, IpAddressFamily, IpSocketAddress, Network, TcpSocket},
};

fn main() {
    let net = Network::default();
    let family = IpAddressFamily::Ipv4;
    let remote1 = IpSocketAddress::new(IpAddress::new_loopback(family), 4321);
    let sock = TcpSocket::new(family).unwrap();

    let bind = sock.blocking_bind(&net, remote1);
    eprintln!("Result of binding: {bind:?}");
    assert!(matches!(bind, Err(ErrorCode::AccessDenied)));

    let listen = sock.blocking_listen();
    eprintln!("Result of listen: {listen:?}");
    assert!(matches!(listen, Err(ErrorCode::AccessDenied)));

    let connect = sock.blocking_connect(&net, remote1);
    eprintln!("Result of connect: {connect:?}");
    assert!(matches!(connect, Err(ErrorCode::AccessDenied)));

    let accept = sock.blocking_accept();
    eprintln!("Result of accept: {accept:?}");
    assert!(matches!(accept, Err(ErrorCode::AccessDenied)));
}

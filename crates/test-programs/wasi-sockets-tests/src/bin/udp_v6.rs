//! Like udp_v4.rs, but with IPv6.

use wasi::io::poll;
use wasi::sockets::network::{IpAddressFamily, IpSocketAddress, Ipv6SocketAddress};
use wasi::sockets::{instance_network, udp_create_socket};
use wasi_sockets_tests::*;

fn main() {
    let net = instance_network::instance_network();

    let sock = udp_create_socket::create_udp_socket(IpAddressFamily::Ipv6).unwrap();

    let addr = IpSocketAddress::Ipv6(Ipv6SocketAddress {
        port: 0,                           // use any free port
        address: (0, 0, 0, 0, 0, 0, 0, 1), // localhost
        flow_info: 0,
        scope_id: 0,
    });

    let sub = sock.subscribe();

    sock.start_bind(&net, addr).unwrap();

    poll::poll_one(&sub);
    drop(sub);

    sock.finish_bind().unwrap();

    example_body_udp(net, sock, IpAddressFamily::Ipv6)
}

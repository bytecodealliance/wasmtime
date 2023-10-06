//! Like v4.rs, but with IPv6.

use test_programs::wasi::io::poll;
use test_programs::wasi::sockets::network::{IpAddressFamily, IpSocketAddress, Ipv6SocketAddress};
use test_programs::wasi::sockets::{instance_network, tcp_create_socket};

fn main() {
    let net = instance_network::instance_network();

    let sock = tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv6).unwrap();

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

    test_programs::sockets::example_body(net, sock, IpAddressFamily::Ipv6)
}

//! A simple TCP testcase, using IPv4.

use test_programs::wasi::io::poll;
use test_programs::wasi::sockets::network::{IpAddressFamily, IpSocketAddress, Ipv4SocketAddress};
use test_programs::wasi::sockets::{instance_network, tcp_create_socket};

fn main() {
    let net = instance_network::instance_network();

    let sock = tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv4).unwrap();

    let addr = IpSocketAddress::Ipv4(Ipv4SocketAddress {
        port: 0,                 // use any free port
        address: (127, 0, 0, 1), // localhost
    });

    let sub = sock.subscribe();

    sock.start_bind(&net, addr).unwrap();

    poll::poll_one(&sub);
    drop(sub);

    sock.finish_bind().unwrap();

    test_programs::sockets::example_body(net, sock, IpAddressFamily::Ipv4)
}

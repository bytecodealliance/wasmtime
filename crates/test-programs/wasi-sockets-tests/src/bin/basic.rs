use wasi::io::streams;
use wasi::poll::poll;
use wasi::sockets::network::{IpAddressFamily, IpSocketAddress, Ipv4SocketAddress};
use wasi::sockets::{instance_network, tcp, tcp_create_socket};
use wasi_sockets_tests::*;

fn wait(sub: poll::Pollable) {
    loop {
        let wait = poll::poll_oneoff(&[sub]);
        if wait[0] {
            break;
        }
    }
}

fn main() {
    let message = b"Hello, world!";

    let net = instance_network::instance_network();

    let sock = tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv4).unwrap();

    let addr = IpSocketAddress::Ipv4(Ipv4SocketAddress {
        port: 0,
        address: (127, 0, 0, 1),
    });

    let sub = tcp::subscribe(sock);

    tcp::start_bind(sock, net, addr).unwrap();
    wait(sub);
    tcp::finish_bind(sock).unwrap();

    tcp::start_listen(sock, net).unwrap();
    wait(sub);
    tcp::finish_listen(sock).unwrap();

    let addr = tcp::local_address(sock).unwrap();

    let client = tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv4).unwrap();
    let client_sub = tcp::subscribe(client);
    tcp::start_connect(client, net, addr).unwrap();
    wait(client_sub);
    let (client_input, client_output) = tcp::finish_connect(client).unwrap();

    let (n, _status) = streams::write(client_output, message).unwrap();
    assert_eq!(n, message.len() as u64); // Not guaranteed to work but should work in practice.

    streams::drop_input_stream(client_input);
    streams::drop_output_stream(client_output);
    poll::drop_pollable(client_sub);
    tcp::drop_tcp_socket(client);

    wait(sub);
    let (_accepted, input, _output) = tcp::accept(sock).unwrap();
    let (data, _status) = streams::read(input, message.len() as u64).unwrap();

    // Check that we sent and recieved our message!
    assert_eq!(data, message); // Not guaranteed to work but should work in practice.
}

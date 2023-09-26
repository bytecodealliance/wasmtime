use wasi_sockets_tests::*;
use wasi::poll::poll;
use wasi::io::streams;
use wasi::sockets::network::{
    self, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
};
use wasi::sockets::{instance_network, tcp, tcp_create_socket};


fn test_sample_application(family: network::IpAddressFamily, bind_address: IpSocketAddress) {
    let first_message = b"Hello, world!";
    let second_message = b"Greetings, planet!";

	let net = instance_network::instance_network();
    let sock = tcp_create_socket::create_tcp_socket(family).unwrap();

    let sub = tcp::subscribe(sock);

    tcp::start_bind(sock, net, bind_address).unwrap();

    wait(sub);
    wasi::poll::poll::drop_pollable(sub);

    tcp::finish_bind(sock).unwrap();

    let sub = tcp::subscribe(sock);

    tcp::start_listen(sock).unwrap();
    wait(sub);
    tcp::finish_listen(sock).unwrap();

    let addr = tcp::local_address(sock).unwrap();

    let client = tcp_create_socket::create_tcp_socket(family).unwrap();
    let client_sub = tcp::subscribe(client);

    tcp::start_connect(client, net, addr).unwrap();
    wait(client_sub);
    let (client_input, client_output) = tcp::finish_connect(client).unwrap();

    let (n, status) = write(client_output, &[]);
    assert_eq!(n, 0);
    assert_eq!(status, streams::StreamStatus::Open);

    let (n, status) = write(client_output, first_message);
    assert_eq!(n, first_message.len());
    assert_eq!(status, streams::StreamStatus::Open);

    streams::drop_input_stream(client_input);
    streams::drop_output_stream(client_output);
    poll::drop_pollable(client_sub);
    tcp::drop_tcp_socket(client);

    wait(sub);
    let (accepted, input, output) = tcp::accept(sock).unwrap();

    let (empty_data, status) = streams::read(input, 0).unwrap();
    assert!(empty_data.is_empty());
    assert_eq!(status, streams::StreamStatus::Open);

    let (data, status) = streams::blocking_read(input, first_message.len() as u64).unwrap();
    assert_eq!(status, streams::StreamStatus::Open);

    streams::drop_input_stream(input);
    streams::drop_output_stream(output);
    tcp::drop_tcp_socket(accepted);

    // Check that we sent and recieved our message!
    assert_eq!(data, first_message); // Not guaranteed to work but should work in practice.

    // Another client
    let client = tcp_create_socket::create_tcp_socket(family).unwrap();
    let client_sub = tcp::subscribe(client);

    tcp::start_connect(client, net, addr).unwrap();
    wait(client_sub);
    let (client_input, client_output) = tcp::finish_connect(client).unwrap();

    let (n, status) = write(client_output, second_message);
    assert_eq!(n, second_message.len());
    assert_eq!(status, streams::StreamStatus::Open);

    streams::drop_input_stream(client_input);
    streams::drop_output_stream(client_output);
    poll::drop_pollable(client_sub);
    tcp::drop_tcp_socket(client);

    wait(sub);
    let (accepted, input, output) = tcp::accept(sock).unwrap();
    let (data, status) = streams::blocking_read(input, second_message.len() as u64).unwrap();
    assert_eq!(status, streams::StreamStatus::Open);

    streams::drop_input_stream(input);
    streams::drop_output_stream(output);
    tcp::drop_tcp_socket(accepted);

    // Check that we sent and recieved our message!
    assert_eq!(data, second_message); // Not guaranteed to work but should work in practice.

    poll::drop_pollable(sub);
    tcp::drop_tcp_socket(sock);
    network::drop_network(net);
}

fn main() {
    test_sample_application(
        IpAddressFamily::Ipv4,
        IpSocketAddress::Ipv4(Ipv4SocketAddress {
            port: 0,                 // use any free port
            address: (127, 0, 0, 1), // localhost
        }),
    );
    test_sample_application(
        IpAddressFamily::Ipv6,
        IpSocketAddress::Ipv6(Ipv6SocketAddress {
            port: 0,                           // use any free port
            address: (0, 0, 0, 0, 0, 0, 0, 1), // localhost
            flow_info: 0,
            scope_id: 0,
        }),
    );
}
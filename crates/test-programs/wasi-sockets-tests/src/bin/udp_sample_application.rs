use wasi::io::poll;
use wasi::sockets::network::{
    IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
};
use wasi::sockets::{instance_network, udp, udp_create_socket};
use wasi_sockets_tests::*;

fn test_sample_application(family: IpAddressFamily, bind_address: IpSocketAddress) {
    let first_message = b"Hello, world!";
    let second_message = b"Greetings, planet!";

    let net = instance_network::instance_network();

    let sock = udp_create_socket::create_udp_socket(family).unwrap();

    let sub = sock.subscribe();

    sock.start_bind(&net, bind_address).unwrap();

    poll::poll_one(&sub);
    drop(sub);

    sock.finish_bind().unwrap();

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

use test_programs::wasi::sockets::network::{
    IpAddress, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress, Network,
};
use test_programs::wasi::sockets::udp::{OutgoingDatagram, UdpSocket};

fn test_udp_sample_application(family: IpAddressFamily, bind_address: IpSocketAddress) {
    let unspecified_addr = IpSocketAddress::new(IpAddress::new_unspecified(family), 0);

    let first_message = &[];
    let second_message = b"Hello, world!";
    let third_message = b"Greetings, planet!";

    let net = Network::default();

    let server = UdpSocket::new(family).unwrap();

    server.blocking_bind(&net, bind_address).unwrap();
    let (server_incoming, _) = server.stream(None).unwrap();
    let addr = server.local_address().unwrap();

    let client_addr = {
        let client = UdpSocket::new(family).unwrap();
        client.blocking_bind(&net, unspecified_addr).unwrap();
        let (_, client_outgoing) = client.stream(Some(addr)).unwrap();

        let datagrams = [
            OutgoingDatagram {
                data: first_message.to_vec(),
                remote_address: None,
            },
            OutgoingDatagram {
                data: second_message.to_vec(),
                remote_address: Some(addr),
            },
        ];
        client_outgoing.blocking_send(&datagrams).unwrap();

        client.local_address().unwrap()
    };

    {
        // Check that we've received our sent messages.
        let datagrams = server_incoming.blocking_receive(2..100).unwrap();
        assert_eq!(datagrams.len(), 2);

        assert_eq!(datagrams[0].data, first_message);
        assert_eq!(datagrams[0].remote_address, client_addr);

        assert_eq!(datagrams[1].data, second_message);
        assert_eq!(datagrams[1].remote_address, client_addr);
    }

    // Another client
    {
        let client = UdpSocket::new(family).unwrap();
        client.blocking_bind(&net, unspecified_addr).unwrap();
        let (_, client_outgoing) = client.stream(None).unwrap();

        let datagrams = [OutgoingDatagram {
            data: third_message.to_vec(),
            remote_address: Some(addr),
        }];
        client_outgoing.blocking_send(&datagrams).unwrap();
    }

    {
        // Check that we sent and received our message!
        let datagrams = server_incoming.blocking_receive(1..100).unwrap();
        assert_eq!(datagrams.len(), 1);

        assert_eq!(datagrams[0].data, third_message);
    }
}

fn main() {
    test_udp_sample_application(
        IpAddressFamily::Ipv4,
        IpSocketAddress::Ipv4(Ipv4SocketAddress {
            port: 0,                 // use any free port
            address: (127, 0, 0, 1), // localhost
        }),
    );
    test_udp_sample_application(
        IpAddressFamily::Ipv6,
        IpSocketAddress::Ipv6(Ipv6SocketAddress {
            port: 0,                           // use any free port
            address: (0, 0, 0, 0, 0, 0, 0, 1), // localhost
            flow_info: 0,
            scope_id: 0,
        }),
    );
}

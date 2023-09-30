use wasi::io::streams;
use wasi::sockets::network::{
    IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress, Network,
};
use wasi::sockets::tcp::TcpSocket;
use wasi_sockets_tests::*;

fn test_sample_application(family: IpAddressFamily, bind_address: IpSocketAddress) {
    let first_message = b"Hello, world!";
    let second_message = b"Greetings, planet!";

    let net = Network::default();
    let listener = TcpSocket::new(family).unwrap();

    listener.blocking_bind(&net, bind_address).unwrap();
    listener.blocking_listen().unwrap();

    let addr = listener.local_address().unwrap();

    {
        let client = TcpSocket::new(family).unwrap();
        let (_client_input, client_output) = client.blocking_connect(&net, addr).unwrap();

        let (n, status) = client_output.blocking_write_util(&[]);
        assert_eq!(n, 0);
        assert_eq!(status, streams::StreamStatus::Open);

        let (n, status) = client_output.blocking_write_util(first_message);
        assert_eq!(n, first_message.len());
        assert_eq!(status, streams::StreamStatus::Open);
    }

    {
        let (_accepted, input, _output) = listener.accept().unwrap();

        let (empty_data, status) = input.read(0).unwrap();
        assert!(empty_data.is_empty());
        assert_eq!(status, streams::StreamStatus::Open);

        let (data, status) = input.blocking_read(first_message.len() as u64).unwrap();
        assert_eq!(status, streams::StreamStatus::Open);

        // Check that we sent and recieved our message!
        assert_eq!(data, first_message); // Not guaranteed to work but should work in practice.
    }

    // Another client
    {
        let client = TcpSocket::new(family).unwrap();
        let (_client_input, client_output) = client.blocking_connect(&net, addr).unwrap();

        let (n, status) = client_output.blocking_write_util(second_message);
        assert_eq!(n, second_message.len());
        assert_eq!(status, streams::StreamStatus::Open);
    }

    {
        let (_accepted, input, _output) = listener.accept().unwrap();
        let (data, status) = input.blocking_read(second_message.len() as u64).unwrap();
        assert_eq!(status, streams::StreamStatus::Open);

        // Check that we sent and recieved our message!
        assert_eq!(data, second_message); // Not guaranteed to work but should work in practice.
    }
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

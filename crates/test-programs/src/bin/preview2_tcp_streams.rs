use test_programs::wasi::io::streams::StreamError;
use test_programs::wasi::sockets::network::{IpAddress, IpAddressFamily, IpSocketAddress, Network};
use test_programs::wasi::sockets::tcp::{ShutdownType, TcpSocket};

/// InputStream::read should return `StreamError::Closed` after the connection has been shut down by the server.
fn test_tcp_read_from_closed_input_stream(net: &Network, family: IpAddressFamily) {
    // Set up server & client sockets:
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let listener = TcpSocket::new(family).unwrap();
    listener.blocking_bind(&net, bind_address).unwrap();
    listener.blocking_listen().unwrap();
    let bound_address = listener.local_address().unwrap();
    let client = TcpSocket::new(family).unwrap();
    let (connected_input, connected_output) = client.blocking_connect(net, bound_address).unwrap();
    let (accepted, accepted_input, accepted_output) = listener.blocking_accept().unwrap();

    // Shut down the connection from the server side:
    accepted.shutdown(ShutdownType::Both).unwrap();
    drop(accepted_input);
    drop(accepted_output);
    drop(accepted);

    // Wait for the shutdown signal to reach the client:
    connected_input.subscribe().block();

    // And now the actual test:

    // The input stream should immediately signal StreamError::Closed.
    // Notably, it should _not_ return an empty list (the wasi-io equivalent of EWOULDBLOCK)
    // See: https://github.com/bytecodealliance/wasmtime/pull/8968
    assert!(matches!(connected_input.read(10), Err(StreamError::Closed)));

    // Stream should still be closed, even when requesting 0 bytes:
    assert!(matches!(connected_input.read(0), Err(StreamError::Closed)));

    drop(connected_input);
    drop(connected_output);
    drop(client);
}

fn main() {
    let net = Network::default();

    test_tcp_read_from_closed_input_stream(&net, IpAddressFamily::Ipv4);
    test_tcp_read_from_closed_input_stream(&net, IpAddressFamily::Ipv6);
}

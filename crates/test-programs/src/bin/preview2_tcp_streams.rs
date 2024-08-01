use test_programs::wasi::io::streams::{InputStream, OutputStream, StreamError};
use test_programs::wasi::sockets::network::{IpAddress, IpAddressFamily, IpSocketAddress, Network};
use test_programs::wasi::sockets::tcp::{ShutdownType, TcpSocket};

/// InputStream::read should return `StreamError::Closed` after the connection has been shut down by the server.
fn test_tcp_input_stream_should_be_closed_by_remote_shutdown(
    net: &Network,
    family: IpAddressFamily,
) {
    setup(net, family, |server, client| {
        // Shut down the connection from the server side:
        server.socket.shutdown(ShutdownType::Both).unwrap();
        drop(server);

        // Wait for the shutdown signal to reach the client:
        client.input.subscribe().block();

        // The input stream should immediately signal StreamError::Closed.
        // Notably, it should _not_ return an empty list (the wasi-io equivalent of EWOULDBLOCK)
        // See: https://github.com/bytecodealliance/wasmtime/pull/8968
        assert!(matches!(client.input.read(10), Err(StreamError::Closed)));

        // Stream should still be closed, even when requesting 0 bytes:
        assert!(matches!(client.input.read(0), Err(StreamError::Closed)));
    });
}

/// OutputStream should return `StreamError::Closed` after the connection has been locally shut down for sending.
fn test_tcp_output_stream_should_be_closed_by_local_shutdown(
    net: &Network,
    family: IpAddressFamily,
) {
    setup(net, family, |_server, client| {
        let message = b"Hi!";

        // The stream should be writable:
        assert!(client.output.check_write().unwrap() as usize >= message.len());

        // Perform the shutdown
        client.socket.shutdown(ShutdownType::Send).unwrap();

        // Stream should be closed:
        assert!(matches!(
            client.output.write(message),
            Err(StreamError::Closed)
        ));

        // The stream should remain closed:
        assert!(matches!(
            client.output.check_write(),
            Err(StreamError::Closed)
        ));
        assert!(matches!(client.output.flush(), Err(StreamError::Closed)));
    });
}

fn main() {
    let net = Network::default();

    test_tcp_input_stream_should_be_closed_by_remote_shutdown(&net, IpAddressFamily::Ipv4);
    test_tcp_input_stream_should_be_closed_by_remote_shutdown(&net, IpAddressFamily::Ipv6);

    test_tcp_output_stream_should_be_closed_by_local_shutdown(&net, IpAddressFamily::Ipv4);
    test_tcp_output_stream_should_be_closed_by_local_shutdown(&net, IpAddressFamily::Ipv6);
}

struct Connection {
    input: InputStream,
    output: OutputStream,
    socket: TcpSocket,
}

/// Set up a connected pair of sockets
fn setup(net: &Network, family: IpAddressFamily, body: impl FnOnce(Connection, Connection)) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let listener = TcpSocket::new(family).unwrap();
    listener.blocking_bind(&net, bind_address).unwrap();
    listener.blocking_listen().unwrap();
    let bound_address = listener.local_address().unwrap();
    let client_socket = TcpSocket::new(family).unwrap();
    let (client_input, client_output) = client_socket.blocking_connect(net, bound_address).unwrap();
    let (accepted_socket, accepted_input, accepted_output) = listener.blocking_accept().unwrap();

    body(
        Connection {
            input: accepted_input,
            output: accepted_output,
            socket: accepted_socket,
        },
        Connection {
            input: client_input,
            output: client_output,
            socket: client_socket,
        },
    );
}

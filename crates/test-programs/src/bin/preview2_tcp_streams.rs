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

/// InputStream::read should return `StreamError::Closed` after the connection has been shut down locally.
fn test_tcp_input_stream_should_be_closed_by_local_shutdown(
    net: &Network,
    family: IpAddressFamily,
) {
    setup(net, family, |server, client| {
        // On Linux, `recv` continues to work even after `shutdown(sock, SHUT_RD)`
        // has been called. To properly test that this behavior doesn't happen in
        // WASI, we make sure there's some data to read by the client:
        server.output.blocking_write_util(b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.").unwrap();

        // Wait for the data to reach the client:
        client.input.subscribe().block();

        // Shut down socket locally:
        client.socket.shutdown(ShutdownType::Receive).unwrap();

        // The input stream should immediately signal StreamError::Closed.
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

/// Calling `shutdown` while the OutputStream is in the middle of a background write should not cause that write to be lost.
fn test_tcp_shutdown_should_not_lose_data(net: &Network, family: IpAddressFamily) {
    setup(net, family, |server, client| {
        // Minimize the local send buffer:
        client.socket.set_send_buffer_size(1024).unwrap();
        let small_buffer_size = client.socket.send_buffer_size().unwrap();

        // Create a significantly bigger buffer, so that we can be pretty sure the `write` won't finish immediately:
        let big_buffer_size = client
            .output
            .check_write()
            .unwrap()
            .min(100 * small_buffer_size);
        assert!(big_buffer_size > small_buffer_size);
        let outgoing_data = vec![0; big_buffer_size as usize];

        // Submit the oversized buffer and immediately initiate the shutdown:
        client.output.write(&outgoing_data).unwrap();
        client.socket.shutdown(ShutdownType::Send).unwrap();

        // The peer should receive _all_ data:
        let incoming_data = server.input.blocking_read_to_end().unwrap();
        assert_eq!(
            outgoing_data.len(),
            incoming_data.len(),
            "Received data should match the sent data"
        );
    });
}

/// InputStream::subscribe should not wake up if there is no data to read.
fn test_tcp_input_stream_should_not_wake_on_empty_data(net: &Network, family: IpAddressFamily) {
    setup(net, family, |server, client| {
        use test_programs::wasi::clocks::monotonic_clock::subscribe_duration;
        let timeout_100ms = 100_000_000;

        // Send some data to the server
        client.output.blocking_write_and_flush(b"Hi!").unwrap();

        server.input.subscribe().block();
        let res = server.input.read(512).unwrap();
        assert_eq!(res, b"Hi!", "Expected to receive data");

        // Don't send any data

        let res = server
            .input
            .subscribe()
            .block_until(&subscribe_duration(timeout_100ms));
        assert!(res.is_err(), "Expected to time out cause no data was sent");
    });
}

fn main() {
    let net = Network::default();

    test_tcp_input_stream_should_be_closed_by_remote_shutdown(&net, IpAddressFamily::Ipv4);
    test_tcp_input_stream_should_be_closed_by_remote_shutdown(&net, IpAddressFamily::Ipv6);

    test_tcp_input_stream_should_be_closed_by_local_shutdown(&net, IpAddressFamily::Ipv4);
    test_tcp_input_stream_should_be_closed_by_local_shutdown(&net, IpAddressFamily::Ipv6);

    test_tcp_output_stream_should_be_closed_by_local_shutdown(&net, IpAddressFamily::Ipv4);
    test_tcp_output_stream_should_be_closed_by_local_shutdown(&net, IpAddressFamily::Ipv6);

    test_tcp_shutdown_should_not_lose_data(&net, IpAddressFamily::Ipv4);
    test_tcp_shutdown_should_not_lose_data(&net, IpAddressFamily::Ipv6);

    test_tcp_input_stream_should_not_wake_on_empty_data(&net, IpAddressFamily::Ipv4);
    test_tcp_input_stream_should_not_wake_on_empty_data(&net, IpAddressFamily::Ipv6);
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

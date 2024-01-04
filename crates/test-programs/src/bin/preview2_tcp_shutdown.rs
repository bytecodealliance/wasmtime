use test_programs::wasi::io::streams::{InputStream, OutputStream, StreamError};
use test_programs::wasi::sockets::network::{IpAddress, IpAddressFamily, IpSocketAddress, Network};
use test_programs::wasi::sockets::tcp::{ShutdownType, TcpSocket};

/// InputStream::read should return `StreamError::Closed` after the connection has been shut down for receiving.
fn test_tcp_shutdown_input_stream_read(net: &Network, family: IpAddressFamily) {
    setup(net, family, |client, input, _output| {
        // The stream should be readable:
        assert_eq!(input.read(10).unwrap().len(), 10);

        // Also test the 0 input length edge case:
        assert_eq!(input.read(0).unwrap().len(), 0);

        // Perform the shutdown
        client.shutdown(ShutdownType::Receive).unwrap();

        // Stream should be closed:
        assert!(matches!(input.read(10), Err(StreamError::Closed)));

        // Stream should still be closed, even when requesting 0 bytes:
        assert!(matches!(input.read(0), Err(StreamError::Closed)));
    });
}

/// OutputStream::check_write methods should return `StreamError::Closed` after the connection has been shut down for sending.
fn test_tcp_shutdown_output_stream_check_write(net: &Network, family: IpAddressFamily) {
    setup(net, family, |client, _input, output| {
        // The stream should be writable:
        assert!(output.check_write().unwrap() > 0);

        // Perform the shutdown
        client.shutdown(ShutdownType::Send).unwrap();

        // Stream should be closed:
        assert!(matches!(output.check_write(), Err(StreamError::Closed)));
    });
}

/// OutputStream::check_write methods should return `StreamError::Closed` after the connection has been shut down for sending.
fn test_tcp_shutdown_output_stream_write(net: &Network, family: IpAddressFamily) {
    setup(net, family, |client, _input, output| {
        let message = b"Hi!";

        // The stream should be writable:
        assert!(output.check_write().unwrap() as usize > message.len());

        // Perform the shutdown
        client.shutdown(ShutdownType::Send).unwrap();

        // Stream should be closed:
        assert!(matches!(output.write(message), Err(StreamError::Closed)));
    });
}

/// OutputStream::check_write methods should return `StreamError::Closed` after the connection has been shut down for sending.
fn test_tcp_shutdown_output_stream_flush(net: &Network, family: IpAddressFamily) {
    setup(net, family, |client, _input, output| {
        // The stream should be writable:
        assert!(output.check_write().unwrap() > 0);

        // Perform the shutdown
        client.shutdown(ShutdownType::Send).unwrap();

        // Stream should be closed:
        assert!(matches!(output.flush(), Err(StreamError::Closed)));
    });
}

fn main() {
    let net = Network::default();

    test_tcp_shutdown_input_stream_read(&net, IpAddressFamily::Ipv4);
    test_tcp_shutdown_input_stream_read(&net, IpAddressFamily::Ipv6);

    test_tcp_shutdown_output_stream_check_write(&net, IpAddressFamily::Ipv4);
    test_tcp_shutdown_output_stream_check_write(&net, IpAddressFamily::Ipv6);
    test_tcp_shutdown_output_stream_write(&net, IpAddressFamily::Ipv4);
    test_tcp_shutdown_output_stream_write(&net, IpAddressFamily::Ipv6);
    test_tcp_shutdown_output_stream_flush(&net, IpAddressFamily::Ipv4);
    test_tcp_shutdown_output_stream_flush(&net, IpAddressFamily::Ipv6);
}

fn setup(
    net: &Network,
    family: IpAddressFamily,
    body: impl FnOnce(&TcpSocket, &InputStream, &OutputStream),
) {
    // Set up a connected TCP client:
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let listener = TcpSocket::new(family).unwrap();
    listener.blocking_bind(&net, bind_address).unwrap();
    listener.blocking_listen().unwrap();
    let bound_address = listener.local_address().unwrap();
    let client = TcpSocket::new(family).unwrap();
    let (input, output) = client.blocking_connect(net, bound_address).unwrap();
    let (accepted, i, o) = listener.blocking_accept().unwrap();

    // On Linux, `recv` continues to work even after `shutdown(sock, SHUT_RD)`
    // has been called. To properly test that this behavior doesn't happen in
    // WASI, we make sure there's some data to read by the client:
    o.blocking_write_util(b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.").unwrap();

    body(&client, &input, &output);

    drop(i);
    drop(o);
    drop(accepted);
}

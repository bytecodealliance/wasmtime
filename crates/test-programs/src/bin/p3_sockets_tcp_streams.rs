use core::future::Future;

use futures::join;
use test_programs::p3::wasi::sockets::types::{
    IpAddress, IpAddressFamily, IpSocketAddress, TcpSocket,
};
use test_programs::p3::wit_stream;

struct Component;

test_programs::p3::export!(Component);

/// InputStream::read should return `StreamError::Closed` after the connection has been shut down by the server.
async fn test_tcp_input_stream_should_be_closed_by_remote_shutdown(family: IpAddressFamily) {
    setup(family, |server, client| async move {
        drop(server);

        let (mut client_rx, client_fut) = client.receive();

        // The input stream should immediately signal StreamError::Closed.
        // Notably, it should _not_ return an empty list (the wasi-io equivalent of EWOULDBLOCK)
        // See: https://github.com/bytecodealliance/wasmtime/pull/8968

        // Wait for the shutdown signal to reach the client:
        assert!(client_rx.next().await.is_none());
        assert_eq!(client_fut.await, Ok(()));
    })
    .await;
}

/// InputStream::read should return `StreamError::Closed` after the connection has been shut down locally.
async fn test_tcp_input_stream_should_be_closed_by_local_shutdown(family: IpAddressFamily) {
    setup(family, |server, client| async move {
        let (mut server_tx, server_rx) = wit_stream::new();
        join!(
            async {
                server.send(server_rx).await.unwrap();
            },
            async {
                // On Linux, `recv` continues to work even after `shutdown(sock, SHUT_RD)`
                // has been called. To properly test that this behavior doesn't happen in
                // WASI, we make sure there's some data to read by the client:
                let rest = server_tx.write_all(b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.".into()).await;
                assert!(rest.is_empty());
                drop(server_tx);
            },
        );

        let (client_rx, client_fut) = client.receive();

        // Shut down socket locally:
        drop(client_rx);
        // Wait for the shutdown signal to reach the client:
        assert_eq!(client_fut.await, Ok(()));
    }).await;
}

/// StreamWriter should return `StreamError::Closed` after the connection has been locally shut down for sending.
async fn test_tcp_output_stream_should_be_closed_by_local_shutdown(family: IpAddressFamily) {
    setup(family, |_server, client| async move {
        let (client_tx, client_rx) = wit_stream::new();
        join!(
            async {
                client.send(client_rx).await.unwrap();
            },
            async {
                // TODO: Verify if send on the stream should return an error
                //assert!(client_tx.send(b"Hi!".into()).await.is_err());
                drop(client_tx);
            }
        );
    })
    .await;
}

/// Calling `shutdown` while the StreamWriter is in the middle of a background write should not cause that write to be lost.
async fn test_tcp_shutdown_should_not_lose_data(family: IpAddressFamily) {
    setup(family, |server, client| async move {
        // Minimize the local send buffer:
        client.set_send_buffer_size(1024).unwrap();
        let small_buffer_size = client.send_buffer_size().unwrap();

        // Create a significantly bigger buffer, so that we can be pretty sure the `write` won't finish immediately:
        let big_buffer_size = 100 * small_buffer_size;
        assert!(big_buffer_size > small_buffer_size);
        let outgoing_data = vec![0; big_buffer_size as usize];

        // Submit the oversized buffer and immediately initiate the shutdown:
        let (mut client_tx, client_rx) = wit_stream::new();
        join!(
            async {
                client.send(client_rx).await.unwrap();
            },
            async {
                let ret = client_tx.write_all(outgoing_data.clone()).await;
                assert!(ret.is_empty());
                drop(client_tx);
            },
            async {
                // The peer should receive _all_ data:
                let (server_rx, server_fut) = server.receive();
                let incoming_data = server_rx.collect().await;
                assert_eq!(
                    outgoing_data, incoming_data,
                    "Received data should match the sent data"
                );
                server_fut.await.unwrap();
            },
        );
    })
    .await;
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_tcp_input_stream_should_be_closed_by_remote_shutdown(IpAddressFamily::Ipv4).await;
        test_tcp_input_stream_should_be_closed_by_remote_shutdown(IpAddressFamily::Ipv6).await;

        test_tcp_input_stream_should_be_closed_by_local_shutdown(IpAddressFamily::Ipv4).await;
        test_tcp_input_stream_should_be_closed_by_local_shutdown(IpAddressFamily::Ipv6).await;

        test_tcp_output_stream_should_be_closed_by_local_shutdown(IpAddressFamily::Ipv4).await;
        test_tcp_output_stream_should_be_closed_by_local_shutdown(IpAddressFamily::Ipv6).await;

        test_tcp_shutdown_should_not_lose_data(IpAddressFamily::Ipv4).await;
        test_tcp_shutdown_should_not_lose_data(IpAddressFamily::Ipv6).await;
        Ok(())
    }
}

fn main() {}

/// Set up a connected pair of sockets
async fn setup<Fut: Future<Output = ()>>(
    family: IpAddressFamily,
    body: impl FnOnce(TcpSocket, TcpSocket) -> Fut,
) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let listener = TcpSocket::new(family);
    listener.bind(bind_address).unwrap();
    let mut accept = listener.listen().unwrap();
    let bound_address = listener.local_address().unwrap();
    let client_socket = TcpSocket::new(family);
    let ((), accepted_socket) = join!(
        async {
            client_socket.connect(bound_address).await.unwrap();
        },
        async { accept.next().await.unwrap() },
    );
    body(accepted_socket, client_socket).await;
}

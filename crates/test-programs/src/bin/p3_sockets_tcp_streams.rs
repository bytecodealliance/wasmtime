use futures::join;
use std::pin::pin;
use std::task::{Context, Poll, Waker};
use test_programs::p3::wasi::sockets::types::{
    IpAddress, IpAddressFamily, IpSocketAddress, TcpSocket,
};
use test_programs::p3::wit_stream;
use test_programs::sockets::supports_ipv6;
use wit_bindgen::StreamResult;

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
        let small_buffer_size = client.get_send_buffer_size().unwrap();

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

/// Model a situation where there's a continuous stream of data coming into the
/// guest from one side and the other side is reading in chunks but also
/// cancelling reads occasionally. Should receive the complete stream of data
/// into the result.
async fn test_tcp_read_cancellation(family: IpAddressFamily) {
    // Send 2M of data in 256-byte chunks.
    const CHUNKS: usize = (2 << 20) / 256;
    let mut data = [0; 256];
    for (i, slot) in data.iter_mut().enumerate() {
        *slot = i as u8;
    }

    setup(family, |server, client| async move {
        // Minimize the local send buffer:
        client.set_send_buffer_size(1024).unwrap();

        let (mut client_tx, client_rx) = wit_stream::new();
        join!(
            async {
                client.send(client_rx).await.unwrap();
            },
            async {
                for _ in 0..CHUNKS {
                    let ret = client_tx.write_all(data.to_vec()).await;
                    assert!(ret.is_empty());
                }
                drop(client_tx);
            },
            async {
                let mut buf = Vec::with_capacity(1024);
                let (mut server_rx, server_fut) = server.receive();
                let mut i = 0_usize;
                let mut consecutive_zero_length_reads = 0;
                loop {
                    assert!(buf.is_empty());
                    let (status, b) = {
                        let mut fut = pin!(server_rx.read(buf));
                        let mut cx = Context::from_waker(Waker::noop());
                        match fut.as_mut().poll(&mut cx) {
                            Poll::Ready(pair) => pair,
                            Poll::Pending => fut.cancel(),
                        }
                    };
                    buf = b;
                    match status {
                        StreamResult::Complete(n) => {
                            assert_eq!(buf.len(), n);
                            for slot in buf.iter_mut() {
                                assert_eq!(*slot, i as u8);
                                i = i.wrapping_add(1);
                            }
                            buf.truncate(0);
                            consecutive_zero_length_reads = 0;
                        }
                        StreamResult::Dropped => break,
                        StreamResult::Cancelled => {
                            assert!(consecutive_zero_length_reads < 10);
                            consecutive_zero_length_reads += 1;
                            server_rx.read(Vec::new()).await;
                        }
                    }
                }
                assert_eq!(i, CHUNKS * 256);
                server_fut.await.unwrap();
            },
        );
    })
    .await;
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_tcp_input_stream_should_be_closed_by_remote_shutdown(IpAddressFamily::Ipv4).await;
        test_tcp_input_stream_should_be_closed_by_local_shutdown(IpAddressFamily::Ipv4).await;
        test_tcp_output_stream_should_be_closed_by_local_shutdown(IpAddressFamily::Ipv4).await;
        test_tcp_shutdown_should_not_lose_data(IpAddressFamily::Ipv4).await;
        test_tcp_read_cancellation(IpAddressFamily::Ipv4).await;

        if supports_ipv6() {
            test_tcp_input_stream_should_be_closed_by_remote_shutdown(IpAddressFamily::Ipv6).await;
            test_tcp_input_stream_should_be_closed_by_local_shutdown(IpAddressFamily::Ipv6).await;
            test_tcp_output_stream_should_be_closed_by_local_shutdown(IpAddressFamily::Ipv6).await;
            test_tcp_shutdown_should_not_lose_data(IpAddressFamily::Ipv6).await;
        }
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
    let listener = TcpSocket::create(family).unwrap();
    listener.bind(bind_address).unwrap();
    let mut accept = listener.listen().unwrap();
    let bound_address = listener.get_local_address().unwrap();
    let client_socket = TcpSocket::create(family).unwrap();
    let ((), accepted_socket) = join!(
        async {
            client_socket.connect(bound_address).await.unwrap();
        },
        async { accept.next().await.unwrap() },
    );
    body(accepted_socket, client_socket).await;
}

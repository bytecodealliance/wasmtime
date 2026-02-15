use futures::join;
use std::pin::pin;
use std::task::{Context, Poll, Waker};
use test_programs::p3::wasi::sockets::types::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, TcpSocket,
};
use test_programs::p3::wit_stream;
use test_programs::sockets::supports_ipv6;
use wit_bindgen::{FutureReader, StreamReader, StreamResult, StreamWriter};

struct Component;

test_programs::p3::export!(Component);

/// Test basic functionality.
async fn test_tcp_ping_pong(family: IpAddressFamily) {
    setup(family, |mut server, mut client| async move {
        {
            let rest = server.send.write_all(b"ping".into()).await;
            assert!(rest.is_empty());
        }
        {
            let (status, buf) = client.receive.read(Vec::with_capacity(4)).await;
            assert_eq!(status, StreamResult::Complete(4));
            assert_eq!(buf, b"ping");
        }
        {
            let rest = client.send.write_all(b"pong".into()).await;
            assert!(rest.is_empty());
        }
        {
            let (status, buf) = server.receive.read(Vec::with_capacity(4)).await;
            assert_eq!(status, StreamResult::Complete(4));
            assert_eq!(buf, b"pong");
        }
    })
    .await;
}

/// The stream and future returned by `receive` should complete/resolve after
/// the connection has been shut down by the remote.
async fn test_tcp_receive_stream_should_be_dropped_by_remote_shutdown(family: IpAddressFamily) {
    setup(family, |server, mut client| async move {
        drop(server);

        // Wait for the shutdown signal to reach the client:
        let (stream_result, data) = client.receive.read(Vec::with_capacity(1)).await;
        assert_eq!(data.len(), 0);
        assert_eq!(stream_result, StreamResult::Dropped);
        assert_eq!(client.receive_result.await, Ok(()));
    })
    .await;
}

/// The future returned by `receive` should resolve once the companion stream
/// has been dropped. Regardless of whether there was still data pending.
async fn test_tcp_receive_future_should_resolve_when_stream_dropped(family: IpAddressFamily) {
    setup(family, |mut server, client| async move {
        {
            let rest = server.send.write_all(b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.".into()).await;
            assert!(rest.is_empty());
        }
        {
            let Connection { mut receive, receive_result, .. } = client;

            // Wait for the data to be ready:
            receive.next().await.unwrap();
            drop(receive);

            // Dropping the stream should've caused the future to resolve even
            // though there was still data pending:
            assert_eq!(receive_result.await, Ok(()));
        }
    }).await;
}

/// The future returned by `send` should resolve after the input stream is dropped.
async fn test_tcp_send_future_should_resolve_when_stream_dropped(family: IpAddressFamily) {
    setup(family, |_server, client| async move {
        let Connection {
            send, send_result, ..
        } = client;
        drop(send);
        assert_eq!(send_result.await, Ok(()));
    })
    .await;
}

/// `send` should drop the input stream when the connection is shut down by the remote.
async fn test_tcp_send_drops_stream_when_remote_shutdown(family: IpAddressFamily) {
    setup(family, |server, mut client| async move {
        drop(server);

        // Give it a few tries for the shutdown signal to reach the client:
        loop {
            let stream_result = client.send.write(b"undeliverable".into()).await.0;
            if stream_result == StreamResult::Dropped {
                break;
            }
        }

        // A remote shutdown is part of normal TCP connection teardown, hence
        // the expected Ok:
        assert_eq!(client.send_result.await, Ok(()));
    })
    .await;
}

/// `receive` may be called successfully at most once.
async fn test_tcp_receive_once(family: IpAddressFamily) {
    setup(family, |mut server, client| async move {
        // Give the client some potential data to _hopefully never_ read.
        {
            let rest = server.send.write_all(b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.".into()).await;
            assert!(rest.is_empty());
        }

        // FYI, the first call to `receive` is part of the `setup` code, so every
        // `receive` in here should fail.
        for _ in 0..3 {
            let (mut reader, future) = client.socket.receive();

            let (stream_result, data) = reader.read(Vec::with_capacity(10)).await;
            assert_eq!(data.len(), 0);
            assert_eq!(stream_result, StreamResult::Dropped);
            assert_eq!(future.await, Err(ErrorCode::InvalidState));
        }
    })
    .await;
}

/// `send` may be called successfully at most once.
async fn test_tcp_send_once(family: IpAddressFamily) {
    setup(family, |_server, client| async move {
        // FYI, the first call to `send` is part of the `setup` code, so every
        // `send` in here should fail.
        for _ in 0..3 {
            let (mut writer, send_rx) = wit_stream::new();
            let future = client.socket.send(send_rx);

            const DATA: &[u8] = b"undeliverable";
            let (stream_result, rest) = writer.write(DATA.into()).await;
            assert_eq!(rest.into_vec(), DATA);
            assert_eq!(stream_result, StreamResult::Dropped);
            assert_eq!(future.await, Err(ErrorCode::InvalidState));
        }
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

    setup(family, |mut server, mut client| async move {
        // Minimize the local send buffer:
        client.socket.set_send_buffer_size(1024).unwrap();

        join!(
            async {
                for _ in 0..CHUNKS {
                    let ret = client.send.write_all(data.to_vec()).await;
                    assert!(ret.is_empty());
                }
                drop(client.send);
            },
            async {
                let mut buf = Vec::with_capacity(1024);
                let mut i = 0_usize;
                let mut consecutive_zero_length_reads = 0;
                loop {
                    assert!(buf.is_empty());
                    let (status, b) = {
                        let mut fut = pin!(server.receive.read(buf));
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
                            server.receive.read(Vec::new()).await;
                        }
                    }
                }
                assert_eq!(i, CHUNKS * 256);
                server.receive_result.await.unwrap();
            },
        );
    })
    .await;
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_tcp_ping_pong(IpAddressFamily::Ipv4).await;
        test_tcp_receive_stream_should_be_dropped_by_remote_shutdown(IpAddressFamily::Ipv4).await;
        test_tcp_receive_future_should_resolve_when_stream_dropped(IpAddressFamily::Ipv4).await;
        test_tcp_send_future_should_resolve_when_stream_dropped(IpAddressFamily::Ipv4).await;
        test_tcp_send_drops_stream_when_remote_shutdown(IpAddressFamily::Ipv4).await;
        test_tcp_receive_once(IpAddressFamily::Ipv4).await;
        test_tcp_send_once(IpAddressFamily::Ipv4).await;
        test_tcp_read_cancellation(IpAddressFamily::Ipv4).await;

        if supports_ipv6() {
            test_tcp_ping_pong(IpAddressFamily::Ipv6).await;
            test_tcp_receive_stream_should_be_dropped_by_remote_shutdown(IpAddressFamily::Ipv6)
                .await;
            test_tcp_receive_future_should_resolve_when_stream_dropped(IpAddressFamily::Ipv6).await;
            test_tcp_send_future_should_resolve_when_stream_dropped(IpAddressFamily::Ipv6).await;
            test_tcp_send_drops_stream_when_remote_shutdown(IpAddressFamily::Ipv6).await;
            test_tcp_receive_once(IpAddressFamily::Ipv6).await;
            test_tcp_send_once(IpAddressFamily::Ipv6).await;
            test_tcp_read_cancellation(IpAddressFamily::Ipv6).await;
        }
        Ok(())
    }
}

fn main() {}

struct Connection {
    socket: TcpSocket,
    receive: StreamReader<u8>,
    receive_result: FutureReader<Result<(), ErrorCode>>,
    send: StreamWriter<u8>,
    send_result: FutureReader<Result<(), ErrorCode>>,
}
impl Connection {
    fn new(socket: TcpSocket) -> Self {
        let (send, send_rx) = wit_stream::new();
        let send_result = socket.send(send_rx);
        let (receive, receive_result) = socket.receive();
        Self {
            socket,
            receive,
            receive_result,
            send,
            send_result,
        }
    }
}

/// Set up a connected pair of sockets
async fn setup<Fut: Future<Output = ()>>(
    family: IpAddressFamily,
    body: impl FnOnce(Connection, Connection) -> Fut,
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

    body(
        Connection::new(accepted_socket),
        Connection::new(client_socket),
    )
    .await;
}

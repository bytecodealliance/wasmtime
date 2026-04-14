use futures::join;
use std::ptr;
use test_programs::async_::{
    BLOCKED, EVENT_NONE, EVENT_STREAM_READ, thread_yield, waitable_join, waitable_set_drop,
    waitable_set_new, waitable_set_poll,
};
use test_programs::p3::wasi::sockets::types::{
    IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress, TcpSocket,
};
use test_programs::p3::wit_stream;
use test_programs::sockets::supports_ipv6;
use wit_bindgen::StreamResult;

#[link(wasm_import_module = "wasi:http/types@0.3.0-rc-2026-03-15")]
unsafe extern "C" {
    #[link_name = "[async-lower][stream-read-0][static]request.new"]
    fn stream_read(_: u32, _: *mut u8, _: usize) -> u32;
}

// Historically, `wasmtime-wasi` had a bug such that polling would starve the
// host executor and prevent e.g. socket readiness from being delivered.  Here
// we verify that such starvation does not happen.
async fn test_tcp_busy_poll(family: IpAddressFamily, address: IpSocketAddress) {
    let listener = TcpSocket::create(family).unwrap();
    listener.bind(address).unwrap();
    listener.set_listen_backlog_size(32).unwrap();
    let mut accept = listener.listen().unwrap();

    let address = listener.get_local_address().unwrap();

    let message = b"Hello, world!";

    for _ in 0..100 {
        let client = TcpSocket::create(family).unwrap();
        client.connect(address).await.unwrap();
        let (mut data_tx, data_rx) = wit_stream::new();
        join!(
            async {
                client.send(data_rx).await.unwrap();
            },
            async {
                let remaining = data_tx.write_all(message.into()).await;
                assert!(remaining.is_empty());
                drop(data_tx);
            }
        );

        let sock = accept.next().await.unwrap();
        let (mut data_rx, fut) = sock.receive();

        unsafe {
            if (stream_read(data_rx.handle(), ptr::null_mut(), 0)) == BLOCKED {
                let set = waitable_set_new();
                waitable_join(data_rx.handle(), set);
                let mut counter = 0;
                loop {
                    if counter > 1_000_000 {
                        panic!("socket still not ready!");
                    }

                    let (mut event, mut waitable, _) = waitable_set_poll(set);
                    if event == EVENT_NONE {
                        // `waitable-set.poll` does not yield, so we
                        // call `thread.yield` to give the executor a
                        // chance to run, then poll one more time.
                        assert!(thread_yield() == 0);
                        (event, waitable, _) = waitable_set_poll(set);
                    }
                    if event == EVENT_STREAM_READ && waitable == data_rx.handle() {
                        waitable_join(data_rx.handle(), 0);
                        waitable_set_drop(set);
                        break;
                    }
                    counter += 1;
                }
            }
        }

        let (result, data) = data_rx.read(Vec::with_capacity(100)).await;
        assert_eq!(result, StreamResult::Complete(message.len()));
        assert_eq!(data, message); // Not guaranteed to work but should work in practice.

        let (result, data) = data_rx.read(Vec::with_capacity(1)).await;
        assert_eq!(result, StreamResult::Dropped);
        assert_eq!(data, []);

        fut.await.unwrap();
    }
}

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_tcp_busy_poll(
            IpAddressFamily::Ipv4,
            IpSocketAddress::Ipv4(Ipv4SocketAddress {
                port: 0,
                address: (127, 0, 0, 1),
            }),
        )
        .await;

        if supports_ipv6() {
            test_tcp_busy_poll(
                IpAddressFamily::Ipv6,
                IpSocketAddress::Ipv6(Ipv6SocketAddress {
                    port: 0,
                    address: (0, 0, 0, 0, 0, 0, 0, 1),
                    flow_info: 0,
                    scope_id: 0,
                }),
            )
            .await;
        }

        Ok(())
    }
}

fn main() {}

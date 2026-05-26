use test_programs::sockets::supports_ipv6;
use test_programs::wasi::clocks::monotonic_clock;
use test_programs::wasi::io::poll;
use test_programs::wasi::sockets::network::{
    IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress, Network,
};
use test_programs::wasi::sockets::tcp::TcpSocket;

// Historically, `wasmtime-wasi` had a bug such that polling with a
// zero-duration (i.e. always-ready) wait would starve the host executor and
// prevent e.g. socket readiness from being delivered.  Here we verify that such
// starvation does not happen.
fn test_tcp_busy_poll(family: IpAddressFamily, address: IpSocketAddress) {
    let zero_wait = monotonic_clock::subscribe_duration(0);

    let net = Network::default();

    let listener = TcpSocket::new(family).unwrap();
    listener.blocking_bind(&net, address).unwrap();
    listener.set_listen_backlog_size(32).unwrap();
    listener.blocking_listen().unwrap();

    let address = listener.local_address().unwrap();

    let message = b"Hello, world!";

    for _ in 0..100 {
        let client = TcpSocket::new(family).unwrap();
        let (_rx, tx) = client.blocking_connect(&net, address).unwrap();
        tx.blocking_write_util(message).unwrap();

        let (_accepted, rx, _tx) = listener.blocking_accept().unwrap();
        let rx_ready = rx.subscribe();
        let mut counter = 0;
        loop {
            if counter > 1_000_000 {
                panic!("socket still not ready!");
            }

            let ready = poll::poll(&[&zero_wait, &rx_ready]);
            if ready.contains(&1) {
                break;
            }
            counter += 1;
        }

        let data = rx.read(message.len().try_into().unwrap()).unwrap();
        assert_eq!(data, message); // Not guaranteed to work but should work in practice.
    }
}

fn main() {
    test_tcp_busy_poll(
        IpAddressFamily::Ipv4,
        IpSocketAddress::Ipv4(Ipv4SocketAddress {
            port: 0,
            address: (127, 0, 0, 1),
        }),
    );

    if supports_ipv6() {
        test_tcp_busy_poll(
            IpAddressFamily::Ipv6,
            IpSocketAddress::Ipv6(Ipv6SocketAddress {
                port: 0,
                address: (0, 0, 0, 0, 0, 0, 0, 1),
                flow_info: 0,
                scope_id: 0,
            }),
        );
    }
}

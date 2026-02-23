use test_programs::wasi::sockets::network::{IpAddress, IpAddressFamily, IpSocketAddress, Network};
use test_programs::wasi::sockets::udp::{OutgoingDatagram, UdpSocket};

/// `outgoing-datagram-stream.send` should trap we attempt to send more
/// datagrams than `check-send` gave permission for.
fn main() {
    let net = Network::default();
    let family = IpAddressFamily::Ipv4;
    let unspecified_port = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let remote = IpSocketAddress::new(IpAddress::new_loopback(family), 4320);

    let client = UdpSocket::new(family).unwrap();
    client.blocking_bind(&net, unspecified_port).unwrap();

    let (_, tx) = client.stream(Some(remote)).unwrap();
    assert_eq!(client.remote_address(), Ok(remote));

    tx.subscribe().block();

    let permits = tx.check_send().unwrap();
    assert!(permits > 0);
    // This should trap according to the `wasi-sockets` spec since we're trying
    // to send more than `check_send` gave permission for:
    tx.send(
        &std::iter::repeat_with(|| OutgoingDatagram {
            data: b"hello".into(),
            remote_address: None,
        })
        .take(usize::try_from(permits + 1).unwrap())
        .collect::<Vec<_>>(),
    )
    .unwrap();
    unreachable!("attempt to send excess datagrams should have trapped");
}

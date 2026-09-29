use test_programs::wasi::clocks::monotonic_clock;
use test_programs::wasi::io::poll;
use test_programs::wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Network,
};
use test_programs::wasi::sockets::udp::{OutgoingDatagram, UdpSocket};

fn test_send_to_closed_receiver(net: &Network, family: IpAddressFamily) {
    let unspecified_port = IpSocketAddress::new(IpAddress::new_loopback(family), 0);

    let sender = UdpSocket::new(family).unwrap();
    sender.blocking_bind(&net, unspecified_port).unwrap();

    let receiver = UdpSocket::new(family).unwrap();
    receiver.blocking_bind(&net, unspecified_port).unwrap();

    let receiver_address = receiver.local_address().unwrap();
    drop(receiver);
    let (rx, tx) = sender.stream(Some(receiver_address)).unwrap();
    tx.blocking_send(&[OutgoingDatagram {
        data: b"hello".to_vec(),
        remote_address: None,
    }])
    .unwrap();

    let deadline = monotonic_clock::now() + 5_000_000_000;
    loop {
        match rx.receive(1) {
            Ok(datagrams) if datagrams.is_empty() => {}
            Ok(_) => panic!("expected error, got datagrams"),
            Err(ErrorCode::ConnectionRefused | ErrorCode::ConnectionReset) => break,
            Err(error) => panic!("unexpected error: {error:?}"),
        }

        let received = rx.subscribe();
        let timeout = monotonic_clock::subscribe_instant(deadline);

        for ready in poll::poll(&[&received, &timeout]) {
            match ready {
                0 => break,
                1 => panic!("receive timed out instead of returning an error"),
                _ => unreachable!(),
            }
        }
    }
}

fn main() {
    let net = Network::default();

    test_send_to_closed_receiver(&net, IpAddressFamily::Ipv4);
    test_send_to_closed_receiver(&net, IpAddressFamily::Ipv6);
}

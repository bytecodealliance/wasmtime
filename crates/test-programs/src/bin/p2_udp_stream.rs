use test_programs::wasi::sockets::network::{IpAddress, IpAddressFamily, IpSocketAddress, Network};
use test_programs::wasi::sockets::udp::{OutgoingDatagram, UdpSocket};

// Check that the implementation supports bulk sends & receives. This isn't
// strictly required by the spec, but it is something wasmtime wants to support
// as long as it doesn't add an unreasonable maintenance burden.
fn test_udp_send_receive_multiple(net: &Network, family: IpAddressFamily) {
    let bind_unspec = IpSocketAddress::new(IpAddress::new_loopback(family), 0);

    let dest = UdpSocket::new(family).unwrap();
    dest.blocking_bind(&net, bind_unspec).unwrap();
    let dest_addr = dest.local_address().unwrap();

    let src = UdpSocket::new(family).unwrap();
    src.blocking_bind(&net, bind_unspec).unwrap();
    let src_addr = src.local_address().unwrap();

    {
        let (_, stream) = src.stream(Some(dest_addr)).unwrap();

        let permit = stream.check_send().unwrap();
        assert!(permit >= 2);

        let sent = stream
            .send(&[
                OutgoingDatagram {
                    data: b"1".into(),
                    remote_address: None,
                },
                OutgoingDatagram {
                    data: b"2".into(),
                    remote_address: None,
                },
            ])
            .unwrap();
        assert_eq!(sent, 2);
    }
    {
        let (stream, _) = dest.stream(Some(src_addr)).unwrap();

        stream.subscribe().block();

        let datagrams = stream.receive(2).unwrap();

        assert_eq!(datagrams.len(), 2);
        assert_eq!(datagrams[0].data, b"1");
        assert_eq!(datagrams[1].data, b"2");
    }
}

// If `stream` is called with `Some(addr)` then the returned stream should only
// return packets from that peer, even if packets from other peers already
// arrived before calling `stream`.
fn test_udp_receive_after_connect(net: &Network, family: IpAddressFamily) {
    let bind_unspec = IpSocketAddress::new(IpAddress::new_loopback(family), 0);

    let dest = UdpSocket::new(family).unwrap();
    dest.blocking_bind(&net, bind_unspec).unwrap();
    let dest_addr = dest.local_address().unwrap();

    let src_a = UdpSocket::new(family).unwrap();
    src_a.blocking_bind(&net, bind_unspec).unwrap();
    let (_, src_stream_a) = src_a.stream(Some(dest_addr)).unwrap();
    let src_a_addr = src_a.local_address().unwrap();

    let src_b = UdpSocket::new(family).unwrap();
    src_b.blocking_bind(&net, bind_unspec).unwrap();
    let (_, src_stream_b) = src_b.stream(Some(dest_addr)).unwrap();
    let src_b_addr = src_b.local_address().unwrap();

    // First enqueue packets from multiple sources:
    src_stream_a
        .blocking_send(&[OutgoingDatagram {
            data: b"A".into(),
            remote_address: None,
        }])
        .unwrap();
    src_stream_b
        .blocking_send(&[OutgoingDatagram {
            data: b"B".into(),
            remote_address: None,
        }])
        .unwrap();
    src_stream_a
        .blocking_send(&[OutgoingDatagram {
            data: b"A".into(),
            remote_address: None,
        }])
        .unwrap();

    // _Then_ connect the socket:
    let (dest_stream, _) = dest.stream(Some(src_b_addr)).unwrap();
    let dgram = dest_stream
        .blocking_receive(1..100)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(dgram.data, b"B");
    assert_eq!(dgram.remote_address, src_b_addr);
    assert_ne!(dgram.remote_address, src_a_addr);
}

fn main() {
    let net = Network::default();

    test_udp_send_receive_multiple(&net, IpAddressFamily::Ipv4);
    test_udp_send_receive_multiple(&net, IpAddressFamily::Ipv6);

    test_udp_receive_after_connect(&net, IpAddressFamily::Ipv4);
    test_udp_receive_after_connect(&net, IpAddressFamily::Ipv6);
}

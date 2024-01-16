use test_programs::wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Network,
};
use test_programs::wasi::sockets::udp::UdpSocket;

fn test_udp_unbound_state_invariants(family: IpAddressFamily) {
    let sock = UdpSocket::new(family).unwrap();

    // Skipping: udp::start_bind
    assert!(matches!(sock.finish_bind(), Err(ErrorCode::NotInProgress)));

    assert!(matches!(sock.stream(None), Err(ErrorCode::InvalidState)));

    assert!(matches!(sock.local_address(), Err(ErrorCode::InvalidState)));
    assert!(matches!(
        sock.remote_address(),
        Err(ErrorCode::InvalidState)
    ));
    assert_eq!(sock.address_family(), family);

    assert!(matches!(sock.unicast_hop_limit(), Ok(_)));
    assert!(matches!(sock.set_unicast_hop_limit(255), Ok(_)));
    assert!(matches!(sock.receive_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_receive_buffer_size(16000), Ok(_)));
    assert!(matches!(sock.send_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_send_buffer_size(16000), Ok(_)));
}

fn test_udp_bound_state_invariants(net: &Network, family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = UdpSocket::new(family).unwrap();
    sock.blocking_bind(net, bind_address).unwrap();

    assert!(matches!(
        sock.start_bind(net, bind_address),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(sock.finish_bind(), Err(ErrorCode::NotInProgress)));
    // Skipping: udp::stream

    assert!(matches!(sock.local_address(), Ok(_)));
    assert!(matches!(
        sock.remote_address(),
        Err(ErrorCode::InvalidState)
    ));
    assert_eq!(sock.address_family(), family);

    assert!(matches!(sock.unicast_hop_limit(), Ok(_)));
    assert!(matches!(sock.set_unicast_hop_limit(255), Ok(_)));
    assert!(matches!(sock.receive_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_receive_buffer_size(16000), Ok(_)));
    assert!(matches!(sock.send_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_send_buffer_size(16000), Ok(_)));
}

fn test_udp_connected_state_invariants(net: &Network, family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let connect_address = IpSocketAddress::new(IpAddress::new_loopback(family), 54321);
    let sock = UdpSocket::new(family).unwrap();
    sock.blocking_bind(net, bind_address).unwrap();
    sock.stream(Some(connect_address)).unwrap();

    assert!(matches!(
        sock.start_bind(net, bind_address),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(sock.finish_bind(), Err(ErrorCode::NotInProgress)));
    // Skipping: udp::stream

    assert!(matches!(sock.local_address(), Ok(_)));
    assert!(matches!(sock.remote_address(), Ok(_)));
    assert_eq!(sock.address_family(), family);

    assert!(matches!(sock.unicast_hop_limit(), Ok(_)));
    assert!(matches!(sock.set_unicast_hop_limit(255), Ok(_)));
    assert!(matches!(sock.receive_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_receive_buffer_size(16000), Ok(_)));
    assert!(matches!(sock.send_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_send_buffer_size(16000), Ok(_)));
}

fn main() {
    let net = Network::default();

    test_udp_unbound_state_invariants(IpAddressFamily::Ipv4);
    test_udp_unbound_state_invariants(IpAddressFamily::Ipv6);

    test_udp_bound_state_invariants(&net, IpAddressFamily::Ipv4);
    test_udp_bound_state_invariants(&net, IpAddressFamily::Ipv6);

    test_udp_connected_state_invariants(&net, IpAddressFamily::Ipv4);
    test_udp_connected_state_invariants(&net, IpAddressFamily::Ipv6);
}

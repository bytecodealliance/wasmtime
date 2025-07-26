use test_programs::p3::wasi::sockets::types::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, UdpSocket,
};

struct Component;

test_programs::p3::export!(Component);

async fn test_udp_unbound_state_invariants(family: IpAddressFamily) {
    let sock = UdpSocket::new(family);

    // Skipping: udp::start_bind

    assert_eq!(
        sock.send(b"test".into(), None).await,
        Err(ErrorCode::InvalidArgument)
    );
    assert_eq!(sock.disconnect(), Err(ErrorCode::InvalidState));
    assert_eq!(sock.local_address(), Err(ErrorCode::InvalidState));
    assert_eq!(sock.remote_address(), Err(ErrorCode::InvalidState));
    assert_eq!(sock.address_family(), family);

    assert!(matches!(sock.unicast_hop_limit(), Ok(_)));
    assert!(matches!(sock.set_unicast_hop_limit(255), Ok(_)));
    assert!(matches!(sock.receive_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_receive_buffer_size(16000), Ok(_)));
    assert!(matches!(sock.send_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_send_buffer_size(16000), Ok(_)));
}

fn test_udp_bound_state_invariants(family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = UdpSocket::new(family);
    sock.bind(bind_address).unwrap();

    assert!(matches!(
        sock.bind(bind_address),
        Err(ErrorCode::InvalidState)
    ));
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

fn test_udp_connected_state_invariants(family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let connect_address = IpSocketAddress::new(IpAddress::new_loopback(family), 54321);
    let sock = UdpSocket::new(family);
    sock.bind(bind_address).unwrap();
    sock.connect(connect_address).unwrap();

    assert!(matches!(
        sock.bind(bind_address),
        Err(ErrorCode::InvalidState)
    ));
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

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_udp_unbound_state_invariants(IpAddressFamily::Ipv4).await;
        test_udp_unbound_state_invariants(IpAddressFamily::Ipv6).await;

        test_udp_bound_state_invariants(IpAddressFamily::Ipv4);
        test_udp_bound_state_invariants(IpAddressFamily::Ipv6);

        test_udp_connected_state_invariants(IpAddressFamily::Ipv4);
        test_udp_connected_state_invariants(IpAddressFamily::Ipv6);
        Ok(())
    }
}

fn main() {}

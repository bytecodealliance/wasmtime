use test_programs::wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Network,
};
use test_programs::wasi::sockets::tcp::{ShutdownType, TcpSocket};

fn test_tcp_unbound_state_invariants(family: IpAddressFamily) {
    let sock = TcpSocket::new(family).unwrap();

    // Skipping: tcp::start_bind
    assert!(matches!(sock.finish_bind(), Err(ErrorCode::NotInProgress)));
    // Skipping: tcp::start_connect
    assert!(matches!(
        sock.finish_connect(),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(
        sock.start_listen(),
        Err(ErrorCode::InvalidState) // Unlike POSIX, trying to listen without an explicit bind should fail in WASI.
    ));
    assert!(matches!(
        sock.finish_listen(),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(sock.accept(), Err(ErrorCode::InvalidState)));
    assert!(matches!(
        sock.shutdown(ShutdownType::Both),
        Err(ErrorCode::InvalidState)
    ));

    assert!(matches!(sock.local_address(), Err(ErrorCode::InvalidState)));
    assert!(matches!(
        sock.remote_address(),
        Err(ErrorCode::InvalidState)
    ));
    assert_eq!(sock.is_listening(), false);
    assert_eq!(sock.address_family(), family);

    assert!(matches!(sock.set_listen_backlog_size(32), Ok(_)));
    assert!(matches!(sock.keep_alive_enabled(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_enabled(false), Ok(_)));
    assert!(matches!(sock.keep_alive_idle_time(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_idle_time(1), Ok(_)));
    assert!(matches!(sock.keep_alive_interval(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_interval(1), Ok(_)));
    assert!(matches!(sock.keep_alive_count(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_count(1), Ok(_)));
    assert!(matches!(sock.hop_limit(), Ok(_)));
    assert!(matches!(sock.set_hop_limit(255), Ok(_)));
    assert!(matches!(sock.receive_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_receive_buffer_size(16000), Ok(_)));
    assert!(matches!(sock.send_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_send_buffer_size(16000), Ok(_)));
}

fn test_tcp_bound_state_invariants(net: &Network, family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = TcpSocket::new(family).unwrap();
    sock.blocking_bind(net, bind_address).unwrap();

    assert!(matches!(
        sock.start_bind(net, bind_address),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(sock.finish_bind(), Err(ErrorCode::NotInProgress)));
    // Skipping: tcp::start_connect
    assert!(matches!(
        sock.finish_connect(),
        Err(ErrorCode::NotInProgress)
    ));
    // Skipping: tcp::start_listen
    assert!(matches!(
        sock.finish_listen(),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(sock.accept(), Err(ErrorCode::InvalidState)));
    assert!(matches!(
        sock.shutdown(ShutdownType::Both),
        Err(ErrorCode::InvalidState)
    ));

    assert!(matches!(sock.local_address(), Ok(_)));
    assert!(matches!(
        sock.remote_address(),
        Err(ErrorCode::InvalidState)
    ));
    assert_eq!(sock.is_listening(), false);
    assert_eq!(sock.address_family(), family);

    assert!(matches!(sock.set_listen_backlog_size(32), Ok(_)));
    assert!(matches!(sock.keep_alive_enabled(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_enabled(false), Ok(_)));
    assert!(matches!(sock.keep_alive_idle_time(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_idle_time(1), Ok(_)));
    assert!(matches!(sock.keep_alive_interval(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_interval(1), Ok(_)));
    assert!(matches!(sock.keep_alive_count(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_count(1), Ok(_)));
    assert!(matches!(sock.hop_limit(), Ok(_)));
    assert!(matches!(sock.set_hop_limit(255), Ok(_)));
    assert!(matches!(sock.receive_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_receive_buffer_size(16000), Ok(_)));
    assert!(matches!(sock.send_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_send_buffer_size(16000), Ok(_)));
}

fn test_tcp_listening_state_invariants(net: &Network, family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = TcpSocket::new(family).unwrap();
    sock.blocking_bind(net, bind_address).unwrap();
    sock.blocking_listen().unwrap();

    assert!(matches!(
        sock.start_bind(net, bind_address),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(sock.finish_bind(), Err(ErrorCode::NotInProgress)));
    assert!(matches!(
        sock.start_connect(net, bind_address), // Actual address shouldn't matter
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(
        sock.finish_connect(),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(sock.start_listen(), Err(ErrorCode::InvalidState)));
    assert!(matches!(
        sock.finish_listen(),
        Err(ErrorCode::NotInProgress)
    ));
    // Skipping: tcp::accept
    assert!(matches!(
        sock.shutdown(ShutdownType::Both),
        Err(ErrorCode::InvalidState)
    ));

    assert!(matches!(sock.local_address(), Ok(_)));
    assert!(matches!(
        sock.remote_address(),
        Err(ErrorCode::InvalidState)
    ));
    assert_eq!(sock.is_listening(), true);
    assert_eq!(sock.address_family(), family);

    assert!(matches!(
        sock.set_listen_backlog_size(32),
        Ok(_) | Err(ErrorCode::NotSupported)
    ));
    assert!(matches!(sock.keep_alive_enabled(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_enabled(false), Ok(_)));
    assert!(matches!(sock.keep_alive_idle_time(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_idle_time(1), Ok(_)));
    assert!(matches!(sock.keep_alive_interval(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_interval(1), Ok(_)));
    assert!(matches!(sock.keep_alive_count(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_count(1), Ok(_)));
    assert!(matches!(sock.hop_limit(), Ok(_)));
    assert!(matches!(sock.set_hop_limit(255), Ok(_)));
    assert!(matches!(sock.receive_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_receive_buffer_size(16000), Ok(_)));
    assert!(matches!(sock.send_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_send_buffer_size(16000), Ok(_)));
}

fn test_tcp_connected_state_invariants(net: &Network, family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock_listener = TcpSocket::new(family).unwrap();
    sock_listener.blocking_bind(net, bind_address).unwrap();
    sock_listener.blocking_listen().unwrap();
    let addr_listener = sock_listener.local_address().unwrap();
    let sock = TcpSocket::new(family).unwrap();
    let (_input, _output) = sock.blocking_connect(net, addr_listener).unwrap();

    assert!(matches!(
        sock.start_bind(net, bind_address),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(sock.finish_bind(), Err(ErrorCode::NotInProgress)));
    assert!(matches!(
        sock.start_connect(net, addr_listener),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(
        sock.finish_connect(),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(sock.start_listen(), Err(ErrorCode::InvalidState)));
    assert!(matches!(
        sock.finish_listen(),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(sock.accept(), Err(ErrorCode::InvalidState)));
    // Skipping: tcp::shutdown

    assert!(matches!(sock.local_address(), Ok(_)));
    assert!(matches!(sock.remote_address(), Ok(_)));
    assert_eq!(sock.is_listening(), false);
    assert_eq!(sock.address_family(), family);

    assert!(matches!(sock.keep_alive_enabled(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_enabled(false), Ok(_)));
    assert!(matches!(sock.keep_alive_idle_time(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_idle_time(1), Ok(_)));
    assert!(matches!(sock.keep_alive_interval(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_interval(1), Ok(_)));
    assert!(matches!(sock.keep_alive_count(), Ok(_)));
    assert!(matches!(sock.set_keep_alive_count(1), Ok(_)));
    assert!(matches!(sock.hop_limit(), Ok(_)));
    assert!(matches!(sock.set_hop_limit(255), Ok(_)));
    assert!(matches!(sock.receive_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_receive_buffer_size(16000), Ok(_)));
    assert!(matches!(sock.send_buffer_size(), Ok(_)));
    assert!(matches!(sock.set_send_buffer_size(16000), Ok(_)));
}

fn main() {
    let net = Network::default();

    test_tcp_unbound_state_invariants(IpAddressFamily::Ipv4);
    test_tcp_unbound_state_invariants(IpAddressFamily::Ipv6);

    test_tcp_bound_state_invariants(&net, IpAddressFamily::Ipv4);
    test_tcp_bound_state_invariants(&net, IpAddressFamily::Ipv6);

    test_tcp_listening_state_invariants(&net, IpAddressFamily::Ipv4);
    test_tcp_listening_state_invariants(&net, IpAddressFamily::Ipv6);

    test_tcp_connected_state_invariants(&net, IpAddressFamily::Ipv4);
    test_tcp_connected_state_invariants(&net, IpAddressFamily::Ipv6);
}

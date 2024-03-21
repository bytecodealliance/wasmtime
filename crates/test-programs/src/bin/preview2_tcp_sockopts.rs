use test_programs::wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Network,
};
use test_programs::wasi::sockets::tcp::TcpSocket;

const SECOND: u64 = 1_000_000_000;

fn test_tcp_sockopt_defaults(family: IpAddressFamily) {
    let sock = TcpSocket::new(family).unwrap();

    assert_eq!(sock.address_family(), family);

    sock.keep_alive_enabled().unwrap(); // Only verify that it has a default value at all, but either value is valid.
    assert!(sock.keep_alive_idle_time().unwrap() > 0);
    assert!(sock.keep_alive_interval().unwrap() > 0);
    assert!(sock.keep_alive_count().unwrap() > 0);
    assert!(sock.hop_limit().unwrap() > 0);
    assert!(sock.receive_buffer_size().unwrap() > 0);
    assert!(sock.send_buffer_size().unwrap() > 0);
}

fn test_tcp_sockopt_input_ranges(family: IpAddressFamily) {
    let sock = TcpSocket::new(family).unwrap();

    assert!(matches!(
        sock.set_listen_backlog_size(0),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(sock.set_listen_backlog_size(1), Ok(_))); // Unsupported sizes should be silently capped.
    assert!(matches!(sock.set_listen_backlog_size(u64::MAX), Ok(_))); // Unsupported sizes should be silently capped.

    assert!(matches!(sock.set_keep_alive_enabled(true), Ok(_)));
    assert!(matches!(sock.set_keep_alive_enabled(false), Ok(_)));

    assert!(matches!(
        sock.set_keep_alive_idle_time(0),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(sock.set_keep_alive_idle_time(1), Ok(_))); // Unsupported sizes should be silently clamped.
    let idle_time = sock.keep_alive_idle_time().unwrap(); // Check that the special 0/reset behavior was not triggered by the previous line.
    assert!(idle_time > 0 && idle_time <= 1 * SECOND);
    assert!(matches!(sock.set_keep_alive_idle_time(u64::MAX), Ok(_))); // Unsupported sizes should be silently clamped.

    assert!(matches!(
        sock.set_keep_alive_interval(0),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(sock.set_keep_alive_interval(1), Ok(_))); // Unsupported sizes should be silently clamped.
    let idle_time = sock.keep_alive_interval().unwrap(); // Check that the special 0/reset behavior was not triggered by the previous line.
    assert!(idle_time > 0 && idle_time <= 1 * SECOND);
    assert!(matches!(sock.set_keep_alive_interval(u64::MAX), Ok(_))); // Unsupported sizes should be silently clamped.

    assert!(matches!(
        sock.set_keep_alive_count(0),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(sock.set_keep_alive_count(1), Ok(_))); // Unsupported sizes should be silently clamped.
    assert!(matches!(sock.set_keep_alive_count(u32::MAX), Ok(_))); // Unsupported sizes should be silently clamped.

    assert!(matches!(
        sock.set_hop_limit(0),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(sock.set_hop_limit(1), Ok(_)));
    assert!(matches!(sock.set_hop_limit(u8::MAX), Ok(_)));

    assert!(matches!(
        sock.set_receive_buffer_size(0),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(sock.set_receive_buffer_size(1), Ok(_))); // Unsupported sizes should be silently capped.
    assert!(matches!(sock.set_receive_buffer_size(u64::MAX), Ok(_))); // Unsupported sizes should be silently capped.
    assert!(matches!(
        sock.set_send_buffer_size(0),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(sock.set_send_buffer_size(1), Ok(_))); // Unsupported sizes should be silently capped.
    assert!(matches!(sock.set_send_buffer_size(u64::MAX), Ok(_))); // Unsupported sizes should be silently capped.
}

fn test_tcp_sockopt_readback(family: IpAddressFamily) {
    let sock = TcpSocket::new(family).unwrap();

    sock.set_keep_alive_enabled(true).unwrap();
    assert_eq!(sock.keep_alive_enabled().unwrap(), true);
    sock.set_keep_alive_enabled(false).unwrap();
    assert_eq!(sock.keep_alive_enabled().unwrap(), false);

    sock.set_keep_alive_idle_time(42 * SECOND).unwrap();
    assert_eq!(sock.keep_alive_idle_time().unwrap(), 42 * SECOND);

    sock.set_keep_alive_interval(42 * SECOND).unwrap();
    assert_eq!(sock.keep_alive_interval().unwrap(), 42 * SECOND);

    sock.set_keep_alive_count(42).unwrap();
    assert_eq!(sock.keep_alive_count().unwrap(), 42);

    sock.set_hop_limit(42).unwrap();
    assert_eq!(sock.hop_limit().unwrap(), 42);

    sock.set_receive_buffer_size(0x10000).unwrap();
    assert_eq!(sock.receive_buffer_size().unwrap(), 0x10000);

    sock.set_send_buffer_size(0x10000).unwrap();
    assert_eq!(sock.send_buffer_size().unwrap(), 0x10000);
}

fn test_tcp_sockopt_inheritance(net: &Network, family: IpAddressFamily) {
    let bind_addr = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let listener = TcpSocket::new(family).unwrap();

    let default_keep_alive = listener.keep_alive_enabled().unwrap();

    // Configure options on listener:
    {
        listener
            .set_keep_alive_enabled(!default_keep_alive)
            .unwrap();
        listener.set_keep_alive_idle_time(42 * SECOND).unwrap();
        listener.set_keep_alive_interval(42 * SECOND).unwrap();
        listener.set_keep_alive_count(42).unwrap();
        listener.set_hop_limit(42).unwrap();
        listener.set_receive_buffer_size(0x10000).unwrap();
        listener.set_send_buffer_size(0x10000).unwrap();
    }

    listener.blocking_bind(&net, bind_addr).unwrap();
    listener.blocking_listen().unwrap();
    let bound_addr = listener.local_address().unwrap();
    let client = TcpSocket::new(family).unwrap();
    client.blocking_connect(&net, bound_addr).unwrap();
    let (accepted_client, _, _) = listener.blocking_accept().unwrap();

    // Verify options on accepted socket:
    {
        assert_eq!(
            accepted_client.keep_alive_enabled().unwrap(),
            !default_keep_alive
        );
        assert_eq!(accepted_client.keep_alive_idle_time().unwrap(), 42 * SECOND);
        assert_eq!(accepted_client.keep_alive_interval().unwrap(), 42 * SECOND);
        assert_eq!(accepted_client.keep_alive_count().unwrap(), 42);
        assert_eq!(accepted_client.hop_limit().unwrap(), 42);
        assert_eq!(accepted_client.receive_buffer_size().unwrap(), 0x10000);
        assert_eq!(accepted_client.send_buffer_size().unwrap(), 0x10000);
    }

    // Update options on listener to something else:
    {
        listener.set_keep_alive_enabled(default_keep_alive).unwrap();
        listener.set_keep_alive_idle_time(43 * SECOND).unwrap();
        listener.set_keep_alive_interval(43 * SECOND).unwrap();
        listener.set_keep_alive_count(43).unwrap();
        listener.set_hop_limit(43).unwrap();
        listener.set_receive_buffer_size(0x20000).unwrap();
        listener.set_send_buffer_size(0x20000).unwrap();
    }

    // Verify that the already accepted socket was not affected:
    {
        assert_eq!(
            accepted_client.keep_alive_enabled().unwrap(),
            !default_keep_alive
        );
        assert_eq!(accepted_client.keep_alive_idle_time().unwrap(), 42 * SECOND);
        assert_eq!(accepted_client.keep_alive_interval().unwrap(), 42 * SECOND);
        assert_eq!(accepted_client.keep_alive_count().unwrap(), 42);
        assert_eq!(accepted_client.hop_limit().unwrap(), 42);
        assert_eq!(accepted_client.receive_buffer_size().unwrap(), 0x10000);
        assert_eq!(accepted_client.send_buffer_size().unwrap(), 0x10000);
    }
}

fn test_tcp_sockopt_after_listen(net: &Network, family: IpAddressFamily) {
    let bind_addr = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let listener = TcpSocket::new(family).unwrap();
    listener.blocking_bind(&net, bind_addr).unwrap();
    listener.blocking_listen().unwrap();
    let bound_addr = listener.local_address().unwrap();

    let default_keep_alive = listener.keep_alive_enabled().unwrap();

    // Update options while the socket is already listening:
    {
        listener
            .set_keep_alive_enabled(!default_keep_alive)
            .unwrap();
        listener.set_keep_alive_idle_time(42 * SECOND).unwrap();
        listener.set_keep_alive_interval(42 * SECOND).unwrap();
        listener.set_keep_alive_count(42).unwrap();
        listener.set_hop_limit(42).unwrap();
        listener.set_receive_buffer_size(0x10000).unwrap();
        listener.set_send_buffer_size(0x10000).unwrap();
    }

    let client = TcpSocket::new(family).unwrap();
    client.blocking_connect(&net, bound_addr).unwrap();
    let (accepted_client, _, _) = listener.blocking_accept().unwrap();

    // Verify options on accepted socket:
    {
        assert_eq!(
            accepted_client.keep_alive_enabled().unwrap(),
            !default_keep_alive
        );
        assert_eq!(accepted_client.keep_alive_idle_time().unwrap(), 42 * SECOND);
        assert_eq!(accepted_client.keep_alive_interval().unwrap(), 42 * SECOND);
        assert_eq!(accepted_client.keep_alive_count().unwrap(), 42);
        assert_eq!(accepted_client.hop_limit().unwrap(), 42);
        assert_eq!(accepted_client.receive_buffer_size().unwrap(), 0x10000);
        assert_eq!(accepted_client.send_buffer_size().unwrap(), 0x10000);
    }
}

fn main() {
    let net = Network::default();

    test_tcp_sockopt_defaults(IpAddressFamily::Ipv4);
    test_tcp_sockopt_defaults(IpAddressFamily::Ipv6);

    test_tcp_sockopt_input_ranges(IpAddressFamily::Ipv4);
    test_tcp_sockopt_input_ranges(IpAddressFamily::Ipv6);

    test_tcp_sockopt_readback(IpAddressFamily::Ipv4);
    test_tcp_sockopt_readback(IpAddressFamily::Ipv6);

    test_tcp_sockopt_inheritance(&net, IpAddressFamily::Ipv4);
    test_tcp_sockopt_inheritance(&net, IpAddressFamily::Ipv6);

    test_tcp_sockopt_after_listen(&net, IpAddressFamily::Ipv4);
    test_tcp_sockopt_after_listen(&net, IpAddressFamily::Ipv6);
}

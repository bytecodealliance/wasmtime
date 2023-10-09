use test_programs::wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Network,
};
use test_programs::wasi::sockets::tcp::TcpSocket;

fn test_tcp_sockopt_defaults(family: IpAddressFamily) {
    let sock = TcpSocket::new(family).unwrap();

    assert_eq!(sock.address_family(), family);

    if family == IpAddressFamily::Ipv6 {
        sock.ipv6_only().unwrap(); // Only verify that it has a default value at all, but either value is valid.
    }

    sock.keep_alive().unwrap(); // Only verify that it has a default value at all, but either value is valid.
    assert_eq!(sock.no_delay().unwrap(), false);
    assert!(sock.unicast_hop_limit().unwrap() > 0);
    assert!(sock.receive_buffer_size().unwrap() > 0);
    assert!(sock.send_buffer_size().unwrap() > 0);
}

fn test_tcp_sockopt_input_ranges(family: IpAddressFamily) {
    let sock = TcpSocket::new(family).unwrap();

    if family == IpAddressFamily::Ipv6 {
        assert!(matches!(sock.set_ipv6_only(true), Ok(_)));
        assert!(matches!(sock.set_ipv6_only(false), Ok(_)));
    }

    assert!(matches!(sock.set_listen_backlog_size(0), Ok(_))); // Unsupported sizes should be silently capped.
    assert!(matches!(sock.set_listen_backlog_size(u64::MAX), Ok(_))); // Unsupported sizes should be silently capped.

    assert!(matches!(sock.set_keep_alive(true), Ok(_)));
    assert!(matches!(sock.set_keep_alive(false), Ok(_)));

    assert!(matches!(sock.set_no_delay(true), Ok(_)));
    assert!(matches!(sock.set_no_delay(false), Ok(_)));

    assert!(matches!(
        sock.set_unicast_hop_limit(0),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(sock.set_unicast_hop_limit(1), Ok(_)));
    assert!(matches!(sock.set_unicast_hop_limit(u8::MAX), Ok(_)));

    assert!(matches!(sock.set_receive_buffer_size(0), Ok(_))); // Unsupported sizes should be silently capped.
    assert!(matches!(sock.set_receive_buffer_size(u64::MAX), Ok(_))); // Unsupported sizes should be silently capped.
    assert!(matches!(sock.set_send_buffer_size(0), Ok(_))); // Unsupported sizes should be silently capped.
    assert!(matches!(sock.set_send_buffer_size(u64::MAX), Ok(_))); // Unsupported sizes should be silently capped.
}

fn test_tcp_sockopt_readback(family: IpAddressFamily) {
    let sock = TcpSocket::new(family).unwrap();

    if family == IpAddressFamily::Ipv6 {
        sock.set_ipv6_only(true).unwrap();
        assert_eq!(sock.ipv6_only().unwrap(), true);
        sock.set_ipv6_only(false).unwrap();
        assert_eq!(sock.ipv6_only().unwrap(), false);
    }

    sock.set_keep_alive(true).unwrap();
    assert_eq!(sock.keep_alive().unwrap(), true);
    sock.set_keep_alive(false).unwrap();
    assert_eq!(sock.keep_alive().unwrap(), false);

    sock.set_no_delay(true).unwrap();
    assert_eq!(sock.no_delay().unwrap(), true);
    sock.set_no_delay(false).unwrap();
    assert_eq!(sock.no_delay().unwrap(), false);

    sock.set_unicast_hop_limit(42).unwrap();
    assert_eq!(sock.unicast_hop_limit().unwrap(), 42);

    sock.set_receive_buffer_size(0x10000).unwrap();
    assert_eq!(sock.receive_buffer_size().unwrap(), 0x10000);

    sock.set_send_buffer_size(0x10000).unwrap();
    assert_eq!(sock.send_buffer_size().unwrap(), 0x10000);
}

fn test_tcp_sockopt_inheritance(net: &Network, family: IpAddressFamily) {
    let bind_addr = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let listener = TcpSocket::new(family).unwrap();

    let default_ipv6_only = listener.ipv6_only().unwrap_or(false);
    let default_keep_alive = listener.keep_alive().unwrap();

    // Configure options on listener:
    {
        if family == IpAddressFamily::Ipv6 {
            listener.set_ipv6_only(!default_ipv6_only).unwrap();
        }

        listener.set_keep_alive(!default_keep_alive).unwrap();
        listener.set_no_delay(true).unwrap();
        listener.set_unicast_hop_limit(42).unwrap();
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
        if family == IpAddressFamily::Ipv6 {
            assert_eq!(accepted_client.ipv6_only().unwrap(), !default_ipv6_only);
        }

        assert_eq!(accepted_client.keep_alive().unwrap(), !default_keep_alive);
        assert_eq!(accepted_client.no_delay().unwrap(), true);
        assert_eq!(accepted_client.unicast_hop_limit().unwrap(), 42);
        assert_eq!(accepted_client.receive_buffer_size().unwrap(), 0x10000);
        assert_eq!(accepted_client.send_buffer_size().unwrap(), 0x10000);
    }

    // Update options on listener to something else:
    {
        listener.set_keep_alive(default_keep_alive).unwrap();
        listener.set_no_delay(false).unwrap();
        listener.set_unicast_hop_limit(43).unwrap();
        listener.set_receive_buffer_size(0x20000).unwrap();
        listener.set_send_buffer_size(0x20000).unwrap();
    }

    // Verify that the already accepted socket was not affected:
    {
        assert_eq!(accepted_client.keep_alive().unwrap(), !default_keep_alive);
        assert_eq!(accepted_client.no_delay().unwrap(), true);
        assert_eq!(accepted_client.unicast_hop_limit().unwrap(), 42);
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

    let default_keep_alive = listener.keep_alive().unwrap();

    // Update options while the socket is already listening:
    {
        listener.set_keep_alive(!default_keep_alive).unwrap();
        listener.set_no_delay(true).unwrap();
        listener.set_unicast_hop_limit(42).unwrap();
        listener.set_receive_buffer_size(0x10000).unwrap();
        listener.set_send_buffer_size(0x10000).unwrap();
    }

    let client = TcpSocket::new(family).unwrap();
    client.blocking_connect(&net, bound_addr).unwrap();
    let (accepted_client, _, _) = listener.blocking_accept().unwrap();

    // Verify options on accepted socket:
    {
        assert_eq!(accepted_client.keep_alive().unwrap(), !default_keep_alive);
        assert_eq!(accepted_client.no_delay().unwrap(), true);
        assert_eq!(accepted_client.unicast_hop_limit().unwrap(), 42);
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

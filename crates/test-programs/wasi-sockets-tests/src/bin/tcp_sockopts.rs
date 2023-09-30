use wasi::sockets::network::{ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress};
use wasi::sockets::tcp;
use wasi_sockets_tests::*;

fn test_tcp_sockopt_defaults(family: IpAddressFamily) {

    let sock = TcpSocketResource::new(family).unwrap();

    assert_eq!(tcp::address_family(sock.handle), family);

    if family == IpAddressFamily::Ipv6 {
        tcp::ipv6_only(sock.handle).unwrap(); // Only verify that it has a default value at all, but either value is valid.
    }

    tcp::keep_alive(sock.handle).unwrap(); // Only verify that it has a default value at all, but either value is valid.
    assert_eq!(tcp::no_delay(sock.handle).unwrap(), false);
    assert!(tcp::unicast_hop_limit(sock.handle).unwrap() > 0);
    assert!(tcp::receive_buffer_size(sock.handle).unwrap() > 0);
    assert!(tcp::send_buffer_size(sock.handle).unwrap() > 0);
}

fn test_tcp_sockopt_input_ranges(family: IpAddressFamily) {

    let sock = TcpSocketResource::new(family).unwrap();

    if family == IpAddressFamily::Ipv6 {
        assert!(matches!(tcp::set_ipv6_only(sock.handle, true), Ok(_)));
        assert!(matches!(tcp::set_ipv6_only(sock.handle, false), Ok(_) | Err(ErrorCode::NotSupported)));
    }

    // FIXME: #7034
    // assert!(matches!(tcp::set_listen_backlog_size(sock.handle, 0), Ok(_))); // Unsupported sizes should be silently capped.
    // assert!(matches!(tcp::set_listen_backlog_size(sock.handle, u64::MAX), Ok(_))); // Unsupported sizes should be silently capped.

    assert!(matches!(tcp::set_keep_alive(sock.handle, true), Ok(_)));
    assert!(matches!(tcp::set_keep_alive(sock.handle, false), Ok(_)));

    assert!(matches!(tcp::set_no_delay(sock.handle, true), Ok(_)));
    assert!(matches!(tcp::set_no_delay(sock.handle, false), Ok(_)));

    assert!(matches!(tcp::set_unicast_hop_limit(sock.handle, 0), Err(ErrorCode::InvalidArgument)));
    assert!(matches!(tcp::set_unicast_hop_limit(sock.handle, 1), Ok(_)));
    assert!(matches!(tcp::set_unicast_hop_limit(sock.handle, u8::MAX), Ok(_)));

    assert!(matches!(tcp::set_receive_buffer_size(sock.handle, 0), Ok(_))); // Unsupported sizes should be silently capped.
    assert!(matches!(tcp::set_receive_buffer_size(sock.handle, u64::MAX), Ok(_))); // Unsupported sizes should be silently capped.
    assert!(matches!(tcp::set_send_buffer_size(sock.handle, 0), Ok(_))); // Unsupported sizes should be silently capped.
    assert!(matches!(tcp::set_send_buffer_size(sock.handle, u64::MAX), Ok(_))); // Unsupported sizes should be silently capped.
}

fn test_tcp_sockopt_readback(family: IpAddressFamily) {

    let sock = TcpSocketResource::new(family).unwrap();

    if family == IpAddressFamily::Ipv6 {
        tcp::set_ipv6_only(sock.handle, true).unwrap();
        assert_eq!(tcp::ipv6_only(sock.handle).unwrap(), true);

        if let Ok(_) = tcp::set_ipv6_only(sock.handle, false) {
            assert_eq!(tcp::ipv6_only(sock.handle).unwrap(), false);
        }
    }

    tcp::set_keep_alive(sock.handle, true).unwrap();
    assert_eq!(tcp::keep_alive(sock.handle).unwrap(), true);
    tcp::set_keep_alive(sock.handle, false).unwrap();
    assert_eq!(tcp::keep_alive(sock.handle).unwrap(), false);

    tcp::set_no_delay(sock.handle, true).unwrap();
    assert_eq!(tcp::no_delay(sock.handle).unwrap(), true);
    tcp::set_no_delay(sock.handle, false).unwrap();
    assert_eq!(tcp::no_delay(sock.handle).unwrap(), false);

    tcp::set_unicast_hop_limit(sock.handle, 42).unwrap();
    assert_eq!(tcp::unicast_hop_limit(sock.handle).unwrap(), 42);

    tcp::set_receive_buffer_size(sock.handle, 0x10000).unwrap();
    assert_eq!(tcp::receive_buffer_size(sock.handle).unwrap(), 0x10000);

    tcp::set_send_buffer_size(sock.handle, 0x10000).unwrap();
    assert_eq!(tcp::send_buffer_size(sock.handle).unwrap(), 0x10000);
}

fn test_tcp_sockopt_inheritance(net: &NetworkResource, family: IpAddressFamily) {
    
    let bind_addr = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let listener = TcpSocketResource::new(family).unwrap();

    let default_keep_alive = tcp::keep_alive(listener.handle).unwrap();

    // Configure options on listener:
    {
        tcp::set_keep_alive(listener.handle, !default_keep_alive).unwrap();
        tcp::set_no_delay(listener.handle, true).unwrap();
        tcp::set_unicast_hop_limit(listener.handle, 42).unwrap();
        tcp::set_receive_buffer_size(listener.handle, 0x10000).unwrap();
        tcp::set_send_buffer_size(listener.handle, 0x10000).unwrap();
    }


    listener.bind(&net, bind_addr).unwrap();
    listener.listen().unwrap();
    let bound_addr = tcp::local_address(listener.handle).unwrap();
    let client = TcpSocketResource::new(family).unwrap();
    client.connect(&net, bound_addr).unwrap();
    let (accepted_client, _, _) = listener.accept().unwrap();

    // Verify options on accepted socket:
    {
        assert_eq!(tcp::keep_alive(accepted_client.handle).unwrap(), !default_keep_alive);
        assert_eq!(tcp::no_delay(accepted_client.handle).unwrap(), true);
        assert_eq!(tcp::unicast_hop_limit(accepted_client.handle).unwrap(), 42);
        assert_eq!(tcp::receive_buffer_size(accepted_client.handle).unwrap(), 0x10000);
        assert_eq!(tcp::send_buffer_size(accepted_client.handle).unwrap(), 0x10000);
    }

    // Update options on listener to something else:
    {
        tcp::set_keep_alive(listener.handle, default_keep_alive).unwrap();
        tcp::set_no_delay(listener.handle, false).unwrap();
        tcp::set_unicast_hop_limit(listener.handle, 43).unwrap();
        tcp::set_receive_buffer_size(listener.handle, 0x20000).unwrap();
        tcp::set_send_buffer_size(listener.handle, 0x20000).unwrap();
    }

    // Verify that the already accepted socket was not affected:
    {
        assert_eq!(tcp::keep_alive(accepted_client.handle).unwrap(), !default_keep_alive);
        assert_eq!(tcp::no_delay(accepted_client.handle).unwrap(), true);
        assert_eq!(tcp::unicast_hop_limit(accepted_client.handle).unwrap(), 42);
        assert_eq!(tcp::receive_buffer_size(accepted_client.handle).unwrap(), 0x10000);
        assert_eq!(tcp::send_buffer_size(accepted_client.handle).unwrap(), 0x10000);
    }
}


fn main() {
    let net = NetworkResource::default();

    test_tcp_sockopt_defaults(IpAddressFamily::Ipv4);
    test_tcp_sockopt_defaults(IpAddressFamily::Ipv6);

    test_tcp_sockopt_input_ranges(IpAddressFamily::Ipv4);
    test_tcp_sockopt_input_ranges(IpAddressFamily::Ipv6);

    test_tcp_sockopt_readback(IpAddressFamily::Ipv4);
    test_tcp_sockopt_readback(IpAddressFamily::Ipv6);

    test_tcp_sockopt_inheritance(&net, IpAddressFamily::Ipv4);
    test_tcp_sockopt_inheritance(&net, IpAddressFamily::Ipv6);
}

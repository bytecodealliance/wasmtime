use wasi::sockets::network::{ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress};
use wasi::sockets::tcp;
use wasi_sockets_tests::*;

fn test_tcp_unbound_state_invariants(family: IpAddressFamily) {
    let sock = TcpSocketResource::new(family).unwrap();

    // Skipping: tcp::start_bind
    assert!(matches!(
        tcp::finish_bind(sock.handle),
        Err(ErrorCode::NotInProgress)
    ));
    // Skipping: tcp::start_connect
    assert!(matches!(
        tcp::finish_connect(sock.handle),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(
        tcp::start_listen(sock.handle),
        Err(ErrorCode::InvalidState) // Unlike POSIX, trying to listen without an explicit bind should fail in WASI.
    ));
    assert!(matches!(
        tcp::finish_listen(sock.handle),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(
        tcp::accept(sock.handle),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(
        tcp::shutdown(sock.handle, tcp::ShutdownType::Both),
        Err(ErrorCode::InvalidState)
    ));

    assert!(matches!(
        tcp::local_address(sock.handle),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(
        tcp::remote_address(sock.handle),
        Err(ErrorCode::InvalidState)
    ));
    assert_eq!(tcp::address_family(sock.handle), family);

    if family == IpAddressFamily::Ipv6 {
        assert!(matches!(tcp::ipv6_only(sock.handle), Ok(_)));

        // Even on platforms that don't support dualstack sockets,
        // setting ipv6_only to true (disabling dualstack mode) should work.
        assert!(matches!(tcp::set_ipv6_only(sock.handle, true), Ok(_)));
    } else {
        assert!(matches!(
            tcp::ipv6_only(sock.handle),
            Err(ErrorCode::NotSupported)
        ));
        assert!(matches!(
            tcp::set_ipv6_only(sock.handle, true),
            Err(ErrorCode::NotSupported)
        ));
    }

    // assert!(matches!(tcp::set_listen_backlog_size(sock.handle, 32), Ok(_))); // FIXME
    assert!(matches!(tcp::keep_alive(sock.handle), Ok(_)));
    assert!(matches!(tcp::set_keep_alive(sock.handle, false), Ok(_)));
    assert!(matches!(tcp::no_delay(sock.handle), Ok(_)));
    assert!(matches!(tcp::set_no_delay(sock.handle, false), Ok(_)));
    assert!(matches!(tcp::unicast_hop_limit(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::set_unicast_hop_limit(sock.handle, 255),
        Ok(_)
    ));
    assert!(matches!(tcp::receive_buffer_size(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::set_receive_buffer_size(sock.handle, 16000),
        Ok(_)
    ));
    assert!(matches!(tcp::send_buffer_size(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::set_send_buffer_size(sock.handle, 16000),
        Ok(_)
    ));
}

fn test_tcp_bound_state_invariants(net: &NetworkResource, family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = TcpSocketResource::new(family).unwrap();
    sock.bind(net, bind_address).unwrap();

    assert!(matches!(
        tcp::start_bind(sock.handle, net.handle, bind_address),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(
        tcp::finish_bind(sock.handle),
        Err(ErrorCode::NotInProgress)
    ));
    // Skipping: tcp::start_connect
    assert!(matches!(
        tcp::finish_connect(sock.handle),
        Err(ErrorCode::NotInProgress)
    ));
    // Skipping: tcp::start_listen
    assert!(matches!(
        tcp::finish_listen(sock.handle),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(
        tcp::accept(sock.handle),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(
        tcp::shutdown(sock.handle, tcp::ShutdownType::Both),
        Err(ErrorCode::InvalidState)
    ));

    assert!(matches!(tcp::local_address(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::remote_address(sock.handle),
        Err(ErrorCode::InvalidState)
    ));
    assert_eq!(tcp::address_family(sock.handle), family);

    if family == IpAddressFamily::Ipv6 {
        assert!(matches!(tcp::ipv6_only(sock.handle), Ok(_)));
        assert!(matches!(
            tcp::set_ipv6_only(sock.handle, true),
            Err(ErrorCode::InvalidState)
        ));
    } else {
        assert!(matches!(
            tcp::ipv6_only(sock.handle),
            Err(ErrorCode::NotSupported)
        ));
        assert!(matches!(
            tcp::set_ipv6_only(sock.handle, true),
            Err(ErrorCode::NotSupported)
        ));
    }

    // assert!(matches!(tcp::set_listen_backlog_size(sock.handle, 32), Err(ErrorCode::AlreadyBound))); // FIXME
    assert!(matches!(tcp::keep_alive(sock.handle), Ok(_)));
    assert!(matches!(tcp::set_keep_alive(sock.handle, false), Ok(_)));
    assert!(matches!(tcp::no_delay(sock.handle), Ok(_)));
    assert!(matches!(tcp::set_no_delay(sock.handle, false), Ok(_)));
    assert!(matches!(tcp::unicast_hop_limit(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::set_unicast_hop_limit(sock.handle, 255),
        Ok(_)
    ));
    assert!(matches!(tcp::receive_buffer_size(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::set_receive_buffer_size(sock.handle, 16000),
        Ok(_)
    ));
    assert!(matches!(tcp::send_buffer_size(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::set_send_buffer_size(sock.handle, 16000),
        Ok(_)
    ));
}

fn test_tcp_listening_state_invariants(net: &NetworkResource, family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = TcpSocketResource::new(family).unwrap();
    sock.bind(net, bind_address).unwrap();
    sock.listen().unwrap();

    assert!(matches!(
        tcp::start_bind(sock.handle, net.handle, bind_address),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(
        tcp::finish_bind(sock.handle),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(
        tcp::start_connect(sock.handle, net.handle, bind_address), // Actual address shouldn't matter
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(
        tcp::finish_connect(sock.handle),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(
        tcp::start_listen(sock.handle),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(
        tcp::finish_listen(sock.handle),
        Err(ErrorCode::NotInProgress)
    ));
    // Skipping: tcp::accept
    assert!(matches!(
        tcp::shutdown(sock.handle, tcp::ShutdownType::Both),
        Err(ErrorCode::InvalidState)
    ));

    assert!(matches!(tcp::local_address(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::remote_address(sock.handle),
        Err(ErrorCode::InvalidState)
    ));
    assert_eq!(tcp::address_family(sock.handle), family);

    if family == IpAddressFamily::Ipv6 {
        assert!(matches!(tcp::ipv6_only(sock.handle), Ok(_)));
        assert!(matches!(
            tcp::set_ipv6_only(sock.handle, true),
            Err(ErrorCode::InvalidState)
        ));
    } else {
        assert!(matches!(
            tcp::ipv6_only(sock.handle),
            Err(ErrorCode::NotSupported)
        ));
        assert!(matches!(
            tcp::set_ipv6_only(sock.handle, true),
            Err(ErrorCode::NotSupported)
        ));
    }

    // assert!(matches!(tcp::set_listen_backlog_size(sock.handle, 32), Err(ErrorCode::AlreadyBound))); // FIXME
    assert!(matches!(tcp::keep_alive(sock.handle), Ok(_)));
    assert!(matches!(tcp::set_keep_alive(sock.handle, false), Ok(_)));
    assert!(matches!(tcp::no_delay(sock.handle), Ok(_)));
    assert!(matches!(tcp::set_no_delay(sock.handle, false), Ok(_)));
    assert!(matches!(tcp::unicast_hop_limit(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::set_unicast_hop_limit(sock.handle, 255),
        Ok(_)
    ));
    assert!(matches!(tcp::receive_buffer_size(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::set_receive_buffer_size(sock.handle, 16000),
        Ok(_)
    ));
    assert!(matches!(tcp::send_buffer_size(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::set_send_buffer_size(sock.handle, 16000),
        Ok(_)
    ));
}

fn test_tcp_connected_state_invariants(net: &NetworkResource, family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock_listener = TcpSocketResource::new(family).unwrap();
    sock_listener.bind(net, bind_address).unwrap();
    sock_listener.listen().unwrap();
    let addr_listener = tcp::local_address(sock_listener.handle).unwrap();
    let sock = TcpSocketResource::new(family).unwrap();
    let (_input, _output) = sock.connect(net, addr_listener).unwrap();

    assert!(matches!(
        tcp::start_bind(sock.handle, net.handle, bind_address),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(
        tcp::finish_bind(sock.handle),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(
        tcp::start_connect(sock.handle, net.handle, addr_listener),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(
        tcp::finish_connect(sock.handle),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(
        tcp::start_listen(sock.handle),
        Err(ErrorCode::InvalidState)
    ));
    assert!(matches!(
        tcp::finish_listen(sock.handle),
        Err(ErrorCode::NotInProgress)
    ));
    assert!(matches!(
        tcp::accept(sock.handle),
        Err(ErrorCode::InvalidState)
    ));
    // Skipping: tcp::shutdown

    assert!(matches!(tcp::local_address(sock.handle), Ok(_)));
    assert!(matches!(tcp::remote_address(sock.handle), Ok(_)));
    assert_eq!(tcp::address_family(sock.handle), family);

    if family == IpAddressFamily::Ipv6 {
        assert!(matches!(tcp::ipv6_only(sock.handle), Ok(_)));
        assert!(matches!(
            tcp::set_ipv6_only(sock.handle, true),
            Err(ErrorCode::InvalidState)
        ));
    } else {
        assert!(matches!(
            tcp::ipv6_only(sock.handle),
            Err(ErrorCode::NotSupported)
        ));
        assert!(matches!(
            tcp::set_ipv6_only(sock.handle, true),
            Err(ErrorCode::NotSupported)
        ));
    }

    assert!(matches!(tcp::keep_alive(sock.handle), Ok(_)));
    assert!(matches!(tcp::set_keep_alive(sock.handle, false), Ok(_)));
    assert!(matches!(tcp::no_delay(sock.handle), Ok(_)));
    assert!(matches!(tcp::set_no_delay(sock.handle, false), Ok(_)));
    assert!(matches!(tcp::unicast_hop_limit(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::set_unicast_hop_limit(sock.handle, 255),
        Ok(_)
    ));
    assert!(matches!(tcp::receive_buffer_size(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::set_receive_buffer_size(sock.handle, 16000),
        Ok(_)
    ));
    assert!(matches!(tcp::send_buffer_size(sock.handle), Ok(_)));
    assert!(matches!(
        tcp::set_send_buffer_size(sock.handle, 16000),
        Ok(_)
    ));
}

fn main() {
    let net = NetworkResource::default();

    test_tcp_unbound_state_invariants(IpAddressFamily::Ipv4);
    test_tcp_unbound_state_invariants(IpAddressFamily::Ipv6);

    test_tcp_bound_state_invariants(&net, IpAddressFamily::Ipv4);
    test_tcp_bound_state_invariants(&net, IpAddressFamily::Ipv6);

    test_tcp_listening_state_invariants(&net, IpAddressFamily::Ipv4);
    test_tcp_listening_state_invariants(&net, IpAddressFamily::Ipv6);

    test_tcp_connected_state_invariants(&net, IpAddressFamily::Ipv4);
    test_tcp_connected_state_invariants(&net, IpAddressFamily::Ipv6);
}

use test_programs::sockets::{attempt_random_port, supports_ipv6};
use test_programs::wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Network,
};
use test_programs::wasi::sockets::tcp::TcpSocket;

/// Bind a socket and let the system determine a port.
fn test_tcp_bind_ephemeral_port(net: &Network, ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock = TcpSocket::new(ip.family()).unwrap();
    sock.blocking_bind(net, bind_addr).unwrap();

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(bind_addr.ip(), bound_addr.ip());
    assert_ne!(bind_addr.port(), bound_addr.port());
}

/// Bind a socket on a specified port.
fn test_tcp_bind_specific_port(net: &Network, ip: IpAddress) {
    let sock = TcpSocket::new(ip.family()).unwrap();

    let bind_addr =
        attempt_random_port(ip, |bind_addr| sock.blocking_bind(net, bind_addr)).unwrap();

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(bind_addr.ip(), bound_addr.ip());
    assert_eq!(bind_addr.port(), bound_addr.port());
}

/// Two sockets may not be actively bound to the same address at the same time.
fn test_tcp_bind_addrinuse(net: &Network, ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock1 = TcpSocket::new(ip.family()).unwrap();
    sock1.blocking_bind(net, bind_addr).unwrap();
    sock1.blocking_listen().unwrap();

    let bound_addr = sock1.local_address().unwrap();

    let sock2 = TcpSocket::new(ip.family()).unwrap();
    assert_eq!(
        sock2.blocking_bind(net, bound_addr),
        Err(ErrorCode::AddressInUse)
    );
}

// The WASI runtime should set SO_REUSEADDR for us
fn test_tcp_bind_reuseaddr(net: &Network, ip: IpAddress) {
    let client = TcpSocket::new(ip.family()).unwrap();

    let bind_addr = {
        let listener1 = TcpSocket::new(ip.family()).unwrap();

        let bind_addr =
            attempt_random_port(ip, |bind_addr| listener1.blocking_bind(net, bind_addr)).unwrap();

        listener1.blocking_listen().unwrap();

        let connect_addr =
            IpSocketAddress::new(IpAddress::new_loopback(ip.family()), bind_addr.port());
        client.blocking_connect(net, connect_addr).unwrap();

        let (accepted_connection, accepted_input, accepted_output) =
            listener1.blocking_accept().unwrap();
        accepted_output.blocking_write_zeroes_and_flush(10).unwrap();
        drop(accepted_input);
        drop(accepted_output);
        drop(accepted_connection);
        drop(listener1);

        bind_addr
    };

    {
        let listener2 = TcpSocket::new(ip.family()).unwrap();

        // If SO_REUSEADDR was configured correctly, the following lines shouldn't be
        // affected by the TIME_WAIT state of the just closed `listener1` socket:
        listener2.blocking_bind(net, bind_addr).unwrap();
        listener2.blocking_listen().unwrap();
    }

    drop(client);
}

// Try binding to an address that is not configured on the system.
fn test_tcp_bind_addrnotavail(net: &Network, ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock = TcpSocket::new(ip.family()).unwrap();

    assert_eq!(
        sock.blocking_bind(net, bind_addr),
        Err(ErrorCode::AddressNotBindable)
    );
}

/// Bind should validate the address family.
fn test_tcp_bind_wrong_family(net: &Network, family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IpAddress::IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IpAddress::IPV4_LOOPBACK,
    };

    let sock = TcpSocket::new(family).unwrap();
    let result = sock.blocking_bind(net, IpSocketAddress::new(wrong_ip, 0));

    assert!(matches!(result, Err(ErrorCode::InvalidArgument)));
}

/// Bind only works on unicast addresses.
fn test_tcp_bind_non_unicast(net: &Network) {
    let ipv4_broadcast = IpSocketAddress::new(IpAddress::IPV4_BROADCAST, 0);
    let ipv4_multicast = IpSocketAddress::new(IpAddress::Ipv4((224, 254, 0, 0)), 0);
    let ipv6_multicast = IpSocketAddress::new(IpAddress::Ipv6((0xff00, 0, 0, 0, 0, 0, 0, 0)), 0);

    let sock_v4 = TcpSocket::new(IpAddressFamily::Ipv4).unwrap();
    let sock_v6 = TcpSocket::new(IpAddressFamily::Ipv6).unwrap();

    assert!(matches!(
        sock_v4.blocking_bind(net, ipv4_broadcast),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(
        sock_v4.blocking_bind(net, ipv4_multicast),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(
        sock_v6.blocking_bind(net, ipv6_multicast),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn test_tcp_bind_dual_stack(net: &Network) {
    let sock = TcpSocket::new(IpAddressFamily::Ipv6).unwrap();
    let addr = IpSocketAddress::new(IpAddress::IPV4_MAPPED_LOOPBACK, 0);

    // Binding an IPv4-mapped-IPv6 address on a ipv6-only socket should fail:
    assert!(matches!(
        sock.blocking_bind(net, addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn main() {
    const RESERVED_IPV4_ADDRESS: IpAddress = IpAddress::Ipv4((192, 0, 2, 0)); // Reserved for documentation and examples.
    const RESERVED_IPV6_ADDRESS: IpAddress = IpAddress::Ipv6((0x2001, 0x0db8, 0, 0, 0, 0, 0, 0)); // Reserved for documentation and examples.

    let net = Network::default();

    test_tcp_bind_ephemeral_port(&net, IpAddress::IPV4_LOOPBACK);
    test_tcp_bind_ephemeral_port(&net, IpAddress::IPV4_UNSPECIFIED);
    test_tcp_bind_specific_port(&net, IpAddress::IPV4_LOOPBACK);
    test_tcp_bind_specific_port(&net, IpAddress::IPV4_UNSPECIFIED);
    test_tcp_bind_reuseaddr(&net, IpAddress::IPV4_LOOPBACK);
    test_tcp_bind_addrinuse(&net, IpAddress::IPV4_LOOPBACK);
    test_tcp_bind_addrinuse(&net, IpAddress::IPV4_UNSPECIFIED);
    test_tcp_bind_addrnotavail(&net, RESERVED_IPV4_ADDRESS);
    test_tcp_bind_wrong_family(&net, IpAddressFamily::Ipv4);

    if supports_ipv6() {
        test_tcp_bind_ephemeral_port(&net, IpAddress::IPV6_LOOPBACK);
        test_tcp_bind_ephemeral_port(&net, IpAddress::IPV6_UNSPECIFIED);
        test_tcp_bind_specific_port(&net, IpAddress::IPV6_LOOPBACK);
        test_tcp_bind_specific_port(&net, IpAddress::IPV6_UNSPECIFIED);
        test_tcp_bind_reuseaddr(&net, IpAddress::IPV6_LOOPBACK);
        test_tcp_bind_addrinuse(&net, IpAddress::IPV6_LOOPBACK);
        test_tcp_bind_addrinuse(&net, IpAddress::IPV6_UNSPECIFIED);
        test_tcp_bind_addrnotavail(&net, RESERVED_IPV6_ADDRESS);
        test_tcp_bind_wrong_family(&net, IpAddressFamily::Ipv6);
        test_tcp_bind_non_unicast(&net);
        test_tcp_bind_dual_stack(&net);
    }
}

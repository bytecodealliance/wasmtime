use test_programs::wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Network,
};
use test_programs::wasi::sockets::tcp::TcpSocket;

const SOME_PORT: u16 = 47; // If the tests pass, this will never actually be connected to.

/// `0.0.0.0` / `::` is not a valid remote address in WASI.
fn test_tcp_connect_unspec(net: &Network, family: IpAddressFamily) {
    let addr = IpSocketAddress::new(IpAddress::new_unspecified(family), SOME_PORT);
    let sock = TcpSocket::new(family).unwrap();

    assert!(matches!(
        sock.blocking_connect(net, addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// 0 is not a valid remote port.
fn test_tcp_connect_port_0(net: &Network, family: IpAddressFamily) {
    let addr = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = TcpSocket::new(family).unwrap();

    assert!(matches!(
        sock.blocking_connect(net, addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// Connect should validate the address family.
fn test_tcp_connect_wrong_family(net: &Network, family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IpAddress::IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IpAddress::IPV4_LOOPBACK,
    };
    let remote_addr = IpSocketAddress::new(wrong_ip, SOME_PORT);

    let sock = TcpSocket::new(family).unwrap();

    assert!(matches!(
        sock.blocking_connect(net, remote_addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// Can only connect to unicast addresses.
fn test_tcp_connect_non_unicast(net: &Network) {
    let ipv4_broadcast = IpSocketAddress::new(IpAddress::IPV4_BROADCAST, SOME_PORT);
    let ipv4_multicast = IpSocketAddress::new(IpAddress::Ipv4((224, 254, 0, 0)), SOME_PORT);
    let ipv6_multicast =
        IpSocketAddress::new(IpAddress::Ipv6((0xff00, 0, 0, 0, 0, 0, 0, 0)), SOME_PORT);

    let sock_v4 = TcpSocket::new(IpAddressFamily::Ipv4).unwrap();
    let sock_v6 = TcpSocket::new(IpAddressFamily::Ipv6).unwrap();

    assert!(matches!(
        sock_v4.blocking_connect(net, ipv4_broadcast),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(
        sock_v4.blocking_connect(net, ipv4_multicast),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(
        sock_v6.blocking_connect(net, ipv6_multicast),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn test_tcp_connect_dual_stack(net: &Network) {
    // Set-up:
    let v4_listener = TcpSocket::new(IpAddressFamily::Ipv4).unwrap();
    v4_listener
        .blocking_bind(&net, IpSocketAddress::new(IpAddress::IPV4_LOOPBACK, 0))
        .unwrap();
    v4_listener.blocking_listen().unwrap();

    let v4_listener_addr = v4_listener.local_address().unwrap();
    let v6_listener_addr =
        IpSocketAddress::new(IpAddress::IPV4_MAPPED_LOOPBACK, v4_listener_addr.port());

    let v6_client = TcpSocket::new(IpAddressFamily::Ipv6).unwrap();

    // Tests:

    // Connecting to an IPv4 address on an IPv6 socket should fail:
    assert!(matches!(
        v6_client.blocking_connect(net, v4_listener_addr),
        Err(ErrorCode::InvalidArgument)
    ));
    // Connecting to an IPv4-mapped-IPv6 address on an IPv6 socket should fail:
    assert!(matches!(
        v6_client.blocking_connect(net, v6_listener_addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// Client sockets can be explicitly bound.
fn test_tcp_connect_explicit_bind(net: &Network, family: IpAddressFamily) {
    let ip = IpAddress::new_loopback(family);

    let listener = {
        let bind_address = IpSocketAddress::new(ip, 0);
        let listener = TcpSocket::new(family).unwrap();
        listener.blocking_bind(&net, bind_address).unwrap();
        listener.blocking_listen().unwrap();
        listener
    };

    let listener_address = listener.local_address().unwrap();
    let client = TcpSocket::new(family).unwrap();

    // Manually bind the client:
    client
        .blocking_bind(net, IpSocketAddress::new(ip, 0))
        .unwrap();

    // Connect should work:
    client.blocking_connect(net, listener_address).unwrap();
}

fn main() {
    let net = Network::default();

    test_tcp_connect_unspec(&net, IpAddressFamily::Ipv4);
    test_tcp_connect_unspec(&net, IpAddressFamily::Ipv6);

    test_tcp_connect_port_0(&net, IpAddressFamily::Ipv4);
    test_tcp_connect_port_0(&net, IpAddressFamily::Ipv6);

    test_tcp_connect_wrong_family(&net, IpAddressFamily::Ipv4);
    test_tcp_connect_wrong_family(&net, IpAddressFamily::Ipv6);

    test_tcp_connect_non_unicast(&net);

    test_tcp_connect_dual_stack(&net);

    test_tcp_connect_explicit_bind(&net, IpAddressFamily::Ipv4);
    test_tcp_connect_explicit_bind(&net, IpAddressFamily::Ipv6);
}

use wasi::sockets::network::{ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress};
use wasi::sockets::tcp;
use wasi_sockets_tests::*;

const SOME_PORT: u16 = 47; // If the tests pass, this will never actually be connected to.

/// `0.0.0.0` / `::` is not a valid remote address in WASI.
fn test_tcp_connect_unspec(net: &NetworkResource, family: IpAddressFamily) {
    let addr = IpSocketAddress::new(IpAddress::new_unspecified(family), SOME_PORT);
    let sock = TcpSocketResource::new(family).unwrap();

    assert!(matches!(
        sock.connect(net, addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// 0 is not a valid remote port.
fn test_tcp_connect_port_0(net: &NetworkResource, family: IpAddressFamily) {
    let addr = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = TcpSocketResource::new(family).unwrap();

    assert!(matches!(
        sock.connect(net, addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// Bind should validate the address family.
fn test_tcp_connect_wrong_family(net: &NetworkResource, family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IpAddress::IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IpAddress::IPV4_LOOPBACK,
    };
    let remote_addr = IpSocketAddress::new(wrong_ip, SOME_PORT);

    let sock = TcpSocketResource::new(family).unwrap();

    assert!(matches!(
        sock.connect(net, remote_addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// Can only connect to unicast addresses.
fn test_tcp_connect_non_unicast(net: &NetworkResource) {
    let ipv4_broadcast = IpSocketAddress::new(IpAddress::IPV4_BROADCAST, SOME_PORT);
    let ipv4_multicast = IpSocketAddress::new(IpAddress::Ipv4((224, 254, 0, 0)), SOME_PORT);
    let ipv6_multicast =
        IpSocketAddress::new(IpAddress::Ipv6((0xff00, 0, 0, 0, 0, 0, 0, 0)), SOME_PORT);

    let sock_v4 = TcpSocketResource::new(IpAddressFamily::Ipv4).unwrap();
    let sock_v6 = TcpSocketResource::new(IpAddressFamily::Ipv6).unwrap();

    assert!(matches!(
        sock_v4.connect(net, ipv4_broadcast),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(
        sock_v4.connect(net, ipv4_multicast),
        Err(ErrorCode::InvalidArgument)
    ));
    assert!(matches!(
        sock_v6.connect(net, ipv6_multicast),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn test_tcp_connect_dual_stack(net: &NetworkResource) {
    // Set-up:
    let v4_listener = TcpSocketResource::new(IpAddressFamily::Ipv4).unwrap();
    v4_listener
        .bind(&net, IpSocketAddress::new(IpAddress::IPV4_LOOPBACK, 0))
        .unwrap();
    v4_listener.listen().unwrap();

    let v4_listener_addr = tcp::local_address(v4_listener.handle).unwrap();
    let v6_listener_addr =
        IpSocketAddress::new(IpAddress::IPV4_MAPPED_LOOPBACK, v4_listener_addr.port());

    let v6_client = TcpSocketResource::new(IpAddressFamily::Ipv6).unwrap();

    // Tests:

    // Even on platforms that don't support dualstack sockets,
    // setting ipv6_only to true (disabling dualstack mode) should work.
    tcp::set_ipv6_only(v6_client.handle, true).unwrap();

    // Connecting to an IPv4-mapped-IPv6 address on an ipv6-only socket should fail:
    assert!(matches!(
        v6_client.connect(net, v6_listener_addr),
        Err(ErrorCode::InvalidArgument)
    ));

    tcp::set_ipv6_only(v6_client.handle, false).unwrap();

    v6_client.connect(net, v6_listener_addr).unwrap();

    let connected_addr = tcp::local_address(v6_client.handle).unwrap();

    assert_eq!(connected_addr.family(), IpAddressFamily::Ipv6);
}

fn main() {
    let net = NetworkResource::default();

    test_tcp_connect_unspec(&net, IpAddressFamily::Ipv4);
    test_tcp_connect_unspec(&net, IpAddressFamily::Ipv6);

    test_tcp_connect_port_0(&net, IpAddressFamily::Ipv4);
    test_tcp_connect_port_0(&net, IpAddressFamily::Ipv6);

    test_tcp_connect_wrong_family(&net, IpAddressFamily::Ipv4);
    test_tcp_connect_wrong_family(&net, IpAddressFamily::Ipv6);

    test_tcp_connect_non_unicast(&net);

    test_tcp_connect_dual_stack(&net);
}

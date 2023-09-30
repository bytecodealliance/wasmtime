use wasi::sockets::network::{ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress};
use wasi::sockets::tcp;
use wasi_sockets_tests::*;

/// Bind a socket and let the system determine a port.
fn test_tcp_bind_ephemeral_port(net: &NetworkResource, ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock = TcpSocketResource::new(ip.family()).unwrap();
    sock.bind(net, bind_addr).unwrap();

    let bound_addr = tcp::local_address(sock.handle).unwrap();

    assert_eq!(bind_addr.ip(), bound_addr.ip());
    assert_ne!(bind_addr.port(), bound_addr.port());
}

/// Bind a socket on a specified port.
fn test_tcp_bind_specific_port(net: &NetworkResource, ip: IpAddress) {
    const PORT: u16 = 54321;

    let bind_addr = IpSocketAddress::new(ip, PORT);

    let sock = TcpSocketResource::new(ip.family()).unwrap();
    sock.bind(net, bind_addr).unwrap();

    let bound_addr = tcp::local_address(sock.handle).unwrap();

    assert_eq!(bind_addr.ip(), bound_addr.ip());
    assert_eq!(bind_addr.port(), bound_addr.port());
}

/// Two sockets may not be actively bound to the same address at the same time.
fn test_tcp_bind_addrinuse(net: &NetworkResource, ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock1 = TcpSocketResource::new(ip.family()).unwrap();
    sock1.bind(net, bind_addr).unwrap();
    sock1.listen().unwrap();

    let bound_addr = tcp::local_address(sock1.handle).unwrap();

    let sock2 = TcpSocketResource::new(ip.family()).unwrap();
    assert_eq!(sock2.bind(net, bound_addr), Err(ErrorCode::AddressInUse));
}

// Try binding to an address that is not configured on the system.
fn test_tcp_bind_addrnotavail(net: &NetworkResource, ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock = TcpSocketResource::new(ip.family()).unwrap();

    assert_eq!(
        sock.bind(net, bind_addr),
        Err(ErrorCode::AddressNotBindable)
    );
}

/// Bind should validate the address family.
fn test_tcp_bind_wrong_family(net: &NetworkResource, family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IpAddress::IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IpAddress::IPV4_LOOPBACK,
    };

    let sock = TcpSocketResource::new(family).unwrap();
    let result = sock.bind(net, IpSocketAddress::new(wrong_ip, 0));

    assert!(matches!(result, Err(ErrorCode::InvalidArgument)));
}

/// Bind only works on unicast addresses.
fn test_tcp_bind_non_unicast(net: &NetworkResource) {

    let ipv4_broadcast = IpSocketAddress::new(IpAddress::IPV4_BROADCAST, 0);
    let ipv4_multicast = IpSocketAddress::new(IpAddress::Ipv4((224, 254, 0, 0)), 0);
    let ipv6_multicast = IpSocketAddress::new(IpAddress::Ipv6((0xff00, 0, 0, 0, 0, 0, 0, 0)), 0);

    let sock_v4 = TcpSocketResource::new(IpAddressFamily::Ipv4).unwrap();
    let sock_v6 = TcpSocketResource::new(IpAddressFamily::Ipv6).unwrap();

    assert!(matches!(sock_v4.bind(net, ipv4_broadcast), Err(ErrorCode::InvalidArgument)));
    assert!(matches!(sock_v4.bind(net, ipv4_multicast), Err(ErrorCode::InvalidArgument)));
    assert!(matches!(sock_v6.bind(net, ipv6_multicast), Err(ErrorCode::InvalidArgument)));
}

fn test_tcp_bind_dual_stack(net: &NetworkResource) {
    let sock = TcpSocketResource::new(IpAddressFamily::Ipv6).unwrap();
    let addr = IpSocketAddress::new(IpAddress::IPV4_MAPPED_LOOPBACK, 0);

    // Even on platforms that don't support dualstack sockets,
    // setting ipv6_only to true (disabling dualstack mode) should work.
    tcp::set_ipv6_only(sock.handle, true).unwrap();

    // Binding an IPv4-mapped-IPv6 address on a ipv6-only socket should fail:
    assert!(matches!(
        sock.bind(net, addr),
        Err(ErrorCode::InvalidArgument)
    ));

    tcp::set_ipv6_only(sock.handle, false).unwrap();

    sock.bind(net, addr).unwrap();

    let bound_addr = tcp::local_address(sock.handle).unwrap();

    assert_eq!(bound_addr.family(), IpAddressFamily::Ipv6);
}

fn main() {
    const RESERVED_IPV4_ADDRESS: IpAddress = IpAddress::Ipv4((192, 0, 2, 0)); // Reserved for documentation and examples.
    const RESERVED_IPV6_ADDRESS: IpAddress = IpAddress::Ipv6((0x2001, 0x0db8, 0, 0, 0, 0, 0, 0)); // Reserved for documentation and examples.

    let net = NetworkResource::default();

    test_tcp_bind_ephemeral_port(&net, IpAddress::IPV4_LOOPBACK);
    test_tcp_bind_ephemeral_port(&net, IpAddress::IPV6_LOOPBACK);
    test_tcp_bind_ephemeral_port(&net, IpAddress::IPV4_UNSPECIFIED);
    test_tcp_bind_ephemeral_port(&net, IpAddress::IPV6_UNSPECIFIED);

    test_tcp_bind_specific_port(&net, IpAddress::IPV4_LOOPBACK);
    test_tcp_bind_specific_port(&net, IpAddress::IPV6_LOOPBACK);
    test_tcp_bind_specific_port(&net, IpAddress::IPV4_UNSPECIFIED);
    test_tcp_bind_specific_port(&net, IpAddress::IPV6_UNSPECIFIED);

    test_tcp_bind_addrinuse(&net, IpAddress::IPV4_LOOPBACK);
    test_tcp_bind_addrinuse(&net, IpAddress::IPV6_LOOPBACK);
    test_tcp_bind_addrinuse(&net, IpAddress::IPV4_UNSPECIFIED);
    test_tcp_bind_addrinuse(&net, IpAddress::IPV6_UNSPECIFIED);

    test_tcp_bind_addrnotavail(&net, RESERVED_IPV4_ADDRESS);
    test_tcp_bind_addrnotavail(&net, RESERVED_IPV6_ADDRESS);

    test_tcp_bind_wrong_family(&net, IpAddressFamily::Ipv4);
    test_tcp_bind_wrong_family(&net, IpAddressFamily::Ipv6);

    test_tcp_bind_non_unicast(&net);
    
    test_tcp_bind_dual_stack(&net);
}

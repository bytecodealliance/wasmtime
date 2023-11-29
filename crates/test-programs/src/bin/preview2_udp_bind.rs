use test_programs::wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Network,
};
use test_programs::wasi::sockets::udp::UdpSocket;

/// Bind a socket and let the system determine a port.
fn test_udp_bind_ephemeral_port(net: &Network, ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock = UdpSocket::new(ip.family()).unwrap();
    sock.blocking_bind(net, bind_addr).unwrap();

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(bind_addr.ip(), bound_addr.ip());
    assert_ne!(bind_addr.port(), bound_addr.port());
}

/// Bind a socket on a specified port.
fn test_udp_bind_specific_port(net: &Network, ip: IpAddress) {
    const PORT: u16 = 54321;

    let bind_addr = IpSocketAddress::new(ip, PORT);

    let sock = UdpSocket::new(ip.family()).unwrap();
    match sock.blocking_bind(net, bind_addr) {
        Ok(()) => {}

        // Concurrent invocations of this test can yield `AddressInUse` and that
        // same error can show up on Windows as `AccessDenied`.
        Err(ErrorCode::AddressInUse | ErrorCode::AccessDenied) => return,
        r => r.unwrap(),
    }

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(bind_addr.ip(), bound_addr.ip());
    assert_eq!(bind_addr.port(), bound_addr.port());
}

/// Two sockets may not be actively bound to the same address at the same time.
fn test_udp_bind_addrinuse(net: &Network, ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock1 = UdpSocket::new(ip.family()).unwrap();
    sock1.blocking_bind(net, bind_addr).unwrap();

    let bound_addr = sock1.local_address().unwrap();

    let sock2 = UdpSocket::new(ip.family()).unwrap();
    assert!(matches!(
        sock2.blocking_bind(net, bound_addr),
        Err(ErrorCode::AddressInUse)
    ));
}

// Try binding to an address that is not configured on the system.
fn test_udp_bind_addrnotavail(net: &Network, ip: IpAddress) {
    let bind_addr = IpSocketAddress::new(ip, 0);

    let sock = UdpSocket::new(ip.family()).unwrap();

    assert!(matches!(
        sock.blocking_bind(net, bind_addr),
        Err(ErrorCode::AddressNotBindable)
    ));
}

/// Bind should validate the address family.
fn test_udp_bind_wrong_family(net: &Network, family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IpAddress::IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IpAddress::IPV4_LOOPBACK,
    };

    let sock = UdpSocket::new(family).unwrap();
    let result = sock.blocking_bind(net, IpSocketAddress::new(wrong_ip, 0));

    assert!(matches!(result, Err(ErrorCode::InvalidArgument)));
}

fn test_udp_bind_dual_stack(net: &Network) {
    let sock = UdpSocket::new(IpAddressFamily::Ipv6).unwrap();
    let addr = IpSocketAddress::new(IpAddress::IPV4_MAPPED_LOOPBACK, 0);

    // Even on platforms that don't support dualstack sockets,
    // setting ipv6_only to true (disabling dualstack mode) should work.
    sock.set_ipv6_only(true).unwrap();

    // Binding an IPv4-mapped-IPv6 address on a ipv6-only socket should fail:
    assert!(matches!(
        sock.blocking_bind(net, addr),
        Err(ErrorCode::InvalidArgument)
    ));

    sock.set_ipv6_only(false).unwrap();

    sock.blocking_bind(net, addr).unwrap();

    let bound_addr = sock.local_address().unwrap();

    assert_eq!(bound_addr.family(), IpAddressFamily::Ipv6);
}

fn main() {
    const RESERVED_IPV4_ADDRESS: IpAddress = IpAddress::Ipv4((192, 0, 2, 0)); // Reserved for documentation and examples.
    const RESERVED_IPV6_ADDRESS: IpAddress = IpAddress::Ipv6((0x2001, 0x0db8, 0, 0, 0, 0, 0, 0)); // Reserved for documentation and examples.

    let net = Network::default();

    test_udp_bind_ephemeral_port(&net, IpAddress::IPV4_LOOPBACK);
    test_udp_bind_ephemeral_port(&net, IpAddress::IPV6_LOOPBACK);
    test_udp_bind_ephemeral_port(&net, IpAddress::IPV4_UNSPECIFIED);
    test_udp_bind_ephemeral_port(&net, IpAddress::IPV6_UNSPECIFIED);

    test_udp_bind_specific_port(&net, IpAddress::IPV4_LOOPBACK);
    test_udp_bind_specific_port(&net, IpAddress::IPV6_LOOPBACK);
    test_udp_bind_specific_port(&net, IpAddress::IPV4_UNSPECIFIED);
    test_udp_bind_specific_port(&net, IpAddress::IPV6_UNSPECIFIED);

    test_udp_bind_addrinuse(&net, IpAddress::IPV4_LOOPBACK);
    test_udp_bind_addrinuse(&net, IpAddress::IPV6_LOOPBACK);
    test_udp_bind_addrinuse(&net, IpAddress::IPV4_UNSPECIFIED);
    test_udp_bind_addrinuse(&net, IpAddress::IPV6_UNSPECIFIED);

    test_udp_bind_addrnotavail(&net, RESERVED_IPV4_ADDRESS);
    test_udp_bind_addrnotavail(&net, RESERVED_IPV6_ADDRESS);

    test_udp_bind_wrong_family(&net, IpAddressFamily::Ipv4);
    test_udp_bind_wrong_family(&net, IpAddressFamily::Ipv6);

    test_udp_bind_dual_stack(&net);
}

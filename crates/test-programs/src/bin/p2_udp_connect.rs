use test_programs::wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Network,
};
use test_programs::wasi::sockets::udp::UdpSocket;

const SOME_PORT: u16 = 47; // If the tests pass, this will never actually be connected to.

fn test_udp_connect_disconnect_reconnect(net: &Network, family: IpAddressFamily) {
    let unspecified_addr = IpSocketAddress::new(IpAddress::new_unspecified(family), 0);
    let remote1 = IpSocketAddress::new(IpAddress::new_loopback(family), 4321);
    let remote2 = IpSocketAddress::new(IpAddress::new_loopback(family), 4320);

    let client = UdpSocket::new(family).unwrap();
    client.blocking_bind(&net, unspecified_addr).unwrap();

    _ = client.stream(None).unwrap();
    assert_eq!(client.remote_address(), Err(ErrorCode::InvalidState));

    _ = client.stream(None).unwrap();
    assert_eq!(client.remote_address(), Err(ErrorCode::InvalidState));

    _ = client.stream(Some(remote1)).unwrap();
    assert_eq!(client.remote_address(), Ok(remote1));

    _ = client.stream(Some(remote1)).unwrap();
    assert_eq!(client.remote_address(), Ok(remote1));

    _ = client.stream(Some(remote2)).unwrap();
    assert_eq!(client.remote_address(), Ok(remote2));

    _ = client.stream(None).unwrap();
    assert_eq!(client.remote_address(), Err(ErrorCode::InvalidState));

    _ = client.stream(Some(remote1)).unwrap();
    assert_eq!(client.remote_address(), Ok(remote1));
}

/// `0.0.0.0` / `::` is not a valid remote address in WASI.
fn test_udp_connect_unspec(net: &Network, family: IpAddressFamily) {
    let addr = IpSocketAddress::new(IpAddress::new_unspecified(family), SOME_PORT);
    let sock = UdpSocket::new(family).unwrap();
    sock.blocking_bind_unspecified(&net).unwrap();

    assert!(matches!(
        sock.stream(Some(addr)),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// 0 is not a valid remote port.
fn test_udp_connect_port_0(net: &Network, family: IpAddressFamily) {
    let addr = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = UdpSocket::new(family).unwrap();
    sock.blocking_bind_unspecified(&net).unwrap();

    assert!(matches!(
        sock.stream(Some(addr)),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// Connect should validate the address family.
fn test_udp_connect_wrong_family(net: &Network, family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IpAddress::IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IpAddress::IPV4_LOOPBACK,
    };
    let remote_addr = IpSocketAddress::new(wrong_ip, SOME_PORT);

    let sock = UdpSocket::new(family).unwrap();
    sock.blocking_bind_unspecified(&net).unwrap();

    assert!(matches!(
        sock.stream(Some(remote_addr)),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn test_udp_connect_dual_stack(net: &Network) {
    // Set-up:
    let v4_server = UdpSocket::new(IpAddressFamily::Ipv4).unwrap();
    v4_server
        .blocking_bind(&net, IpSocketAddress::new(IpAddress::IPV4_LOOPBACK, 0))
        .unwrap();

    let v4_server_addr = v4_server.local_address().unwrap();
    let v6_server_addr =
        IpSocketAddress::new(IpAddress::IPV4_MAPPED_LOOPBACK, v4_server_addr.port());

    // Tests:
    let v6_client = UdpSocket::new(IpAddressFamily::Ipv6).unwrap();

    v6_client.blocking_bind_unspecified(&net).unwrap();

    // Connecting to an IPv4 address on an IPv6 socket should fail:
    assert!(matches!(
        v6_client.stream(Some(v4_server_addr)),
        Err(ErrorCode::InvalidArgument)
    ));

    // Connecting to an IPv4-mapped-IPv6 address on an IPv6 socket should fail:
    assert!(matches!(
        v6_client.stream(Some(v6_server_addr)),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn main() {
    let net = Network::default();

    test_udp_connect_disconnect_reconnect(&net, IpAddressFamily::Ipv4);
    test_udp_connect_disconnect_reconnect(&net, IpAddressFamily::Ipv6);

    test_udp_connect_unspec(&net, IpAddressFamily::Ipv4);
    test_udp_connect_unspec(&net, IpAddressFamily::Ipv6);

    test_udp_connect_port_0(&net, IpAddressFamily::Ipv4);
    test_udp_connect_port_0(&net, IpAddressFamily::Ipv6);

    test_udp_connect_wrong_family(&net, IpAddressFamily::Ipv4);
    test_udp_connect_wrong_family(&net, IpAddressFamily::Ipv6);

    test_udp_connect_dual_stack(&net);
}

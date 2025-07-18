use test_programs::p3::wasi::sockets::types::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, UdpSocket,
};

struct Component;

test_programs::p3::export!(Component);

const SOME_PORT: u16 = 47; // If the tests pass, this will never actually be connected to.

fn test_udp_connect_disconnect_reconnect(family: IpAddressFamily) {
    let unspecified_addr = IpSocketAddress::new(IpAddress::new_unspecified(family), 0);
    let remote1 = IpSocketAddress::new(IpAddress::new_loopback(family), 4321);
    let remote2 = IpSocketAddress::new(IpAddress::new_loopback(family), 4320);

    let client = UdpSocket::new(family);
    client.bind(unspecified_addr).unwrap();

    assert_eq!(client.disconnect(), Err(ErrorCode::InvalidState));
    assert_eq!(client.remote_address(), Err(ErrorCode::InvalidState));

    assert_eq!(client.disconnect(), Err(ErrorCode::InvalidState));
    assert_eq!(client.remote_address(), Err(ErrorCode::InvalidState));

    _ = client.connect(remote1).unwrap();
    assert_eq!(client.remote_address(), Ok(remote1));

    _ = client.connect(remote1).unwrap();
    assert_eq!(client.remote_address(), Ok(remote1));

    _ = client.connect(remote2).unwrap();
    assert_eq!(client.remote_address(), Ok(remote2));

    _ = client.disconnect().unwrap();
    assert_eq!(client.remote_address(), Err(ErrorCode::InvalidState));

    _ = client.connect(remote1).unwrap();
    assert_eq!(client.remote_address(), Ok(remote1));
}

/// `0.0.0.0` / `::` is not a valid remote address in WASI.
fn test_udp_connect_unspec(family: IpAddressFamily) {
    let ip = IpAddress::new_unspecified(family);
    let addr = IpSocketAddress::new(ip, SOME_PORT);
    let sock = UdpSocket::new(family);
    sock.bind_unspecified().unwrap();

    assert!(matches!(
        sock.connect(addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// 0 is not a valid remote port.
fn test_udp_connect_port_0(family: IpAddressFamily) {
    let addr = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = UdpSocket::new(family);
    sock.bind_unspecified().unwrap();

    assert!(matches!(
        sock.connect(addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// Connect should validate the address family.
fn test_udp_connect_wrong_family(family: IpAddressFamily) {
    let wrong_ip = match family {
        IpAddressFamily::Ipv4 => IpAddress::IPV6_LOOPBACK,
        IpAddressFamily::Ipv6 => IpAddress::IPV4_LOOPBACK,
    };
    let remote_addr = IpSocketAddress::new(wrong_ip, SOME_PORT);

    let sock = UdpSocket::new(family);
    sock.bind_unspecified().unwrap();

    assert!(matches!(
        sock.connect(remote_addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

fn test_udp_connect_dual_stack() {
    // Set-up:
    let v4_server = UdpSocket::new(IpAddressFamily::Ipv4);
    v4_server
        .bind(IpSocketAddress::new(IpAddress::IPV4_LOOPBACK, 0))
        .unwrap();

    let v4_server_addr = v4_server.local_address().unwrap();
    let v6_server_addr =
        IpSocketAddress::new(IpAddress::IPV4_MAPPED_LOOPBACK, v4_server_addr.port());

    // Tests:
    let v6_client = UdpSocket::new(IpAddressFamily::Ipv6);

    v6_client.bind_unspecified().unwrap();

    // Connecting to an IPv4 address on an IPv6 socket should fail:
    assert!(matches!(
        v6_client.connect(v4_server_addr),
        Err(ErrorCode::InvalidArgument)
    ));

    // Connecting to an IPv4-mapped-IPv6 address on an IPv6 socket should fail:
    assert!(matches!(
        v6_client.connect(v6_server_addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_udp_connect_disconnect_reconnect(IpAddressFamily::Ipv4);
        test_udp_connect_disconnect_reconnect(IpAddressFamily::Ipv6);

        test_udp_connect_unspec(IpAddressFamily::Ipv4);
        test_udp_connect_unspec(IpAddressFamily::Ipv6);

        test_udp_connect_port_0(IpAddressFamily::Ipv4);
        test_udp_connect_port_0(IpAddressFamily::Ipv6);

        test_udp_connect_wrong_family(IpAddressFamily::Ipv4);
        test_udp_connect_wrong_family(IpAddressFamily::Ipv6);

        test_udp_connect_dual_stack();
        Ok(())
    }
}

fn main() {}

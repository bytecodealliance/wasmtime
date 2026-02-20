use test_programs::{
    p3::wasi::sockets::types::{
        ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Ipv4Address, Ipv6Address, UdpSocket,
    },
    sockets::supports_ipv6,
};

struct Component;

test_programs::p3::export!(Component);

// If the tests work as expected, these will never actually be connected to:
const SOME_PORT: u16 = 47;
const SOME_PUBLIC_IPV4: Ipv4Address = (123, 234, 12, 34);
const SOME_PUBLIC_IPV6: Ipv6Address = (123, 234, 0, 0, 0, 0, 0, 34);

fn test_udp_connect_disconnect_reconnect(family: IpAddressFamily) {
    let remote1 = IpSocketAddress::new(IpAddress::new_loopback(family), 4321);
    let remote2 = IpSocketAddress::new(IpAddress::new_loopback(family), 4320);

    let client = UdpSocket::create(family).unwrap();

    assert_eq!(client.disconnect(), Err(ErrorCode::InvalidState));
    assert_eq!(client.get_remote_address(), Err(ErrorCode::InvalidState));

    assert_eq!(client.disconnect(), Err(ErrorCode::InvalidState));
    assert_eq!(client.get_remote_address(), Err(ErrorCode::InvalidState));

    _ = client.connect(remote1).unwrap();
    assert_eq!(client.get_remote_address(), Ok(remote1));

    _ = client.connect(remote1).unwrap();
    assert_eq!(client.get_remote_address(), Ok(remote1));

    _ = client.connect(remote2).unwrap();
    assert_eq!(client.get_remote_address(), Ok(remote2));

    _ = client.disconnect().unwrap();
    assert_eq!(client.get_remote_address(), Err(ErrorCode::InvalidState));

    _ = client.connect(remote1).unwrap();
    assert_eq!(client.get_remote_address(), Ok(remote1));
}

/// `0.0.0.0` / `::` is not a valid remote address in WASI.
fn test_udp_connect_unspec(family: IpAddressFamily) {
    let ip = IpAddress::new_unspecified(family);
    let addr = IpSocketAddress::new(ip, SOME_PORT);
    let sock = UdpSocket::create(family).unwrap();

    assert!(matches!(
        sock.connect(addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// If not explicitly bound, connecting a UDP socket should update the local
/// address to reflect the best network path.
fn test_udp_connect_local_address_change(family: IpAddressFamily) {
    fn connect(sock: &UdpSocket, ip: IpAddress, port: u16) -> IpSocketAddress {
        let remote = IpSocketAddress::new(ip, port);
        sock.connect(remote).unwrap();
        let local = sock.get_local_address().unwrap();
        println!("connect({remote:?}) changed local address to: {local:?}",);
        local
    }

    if !has_public_interface(family) {
        println!("No public interface detected, skipping test");
        return;
    }

    let loopback_ip = IpAddress::new_loopback(family);
    let public_ip = some_public_ip(family);

    let client = UdpSocket::create(family).unwrap();

    let loopback_if1 = connect(&client, loopback_ip, 4321);
    let loopback_if2 = connect(&client, loopback_ip, 4322);
    let public_if = connect(&client, public_ip, 4323);

    // Note: these assertions are based on observed behavior on Linux, MacOS and
    // Windows, but there is nothing in their official documentation to
    // corroborate this.
    assert_eq!(loopback_if1, loopback_if2);
    assert_ne!(loopback_if1, public_if);
}

/// 0 is not a valid remote port.
fn test_udp_connect_port_0(family: IpAddressFamily) {
    let addr = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = UdpSocket::create(family).unwrap();

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

    let sock = UdpSocket::create(family).unwrap();

    assert!(matches!(
        sock.connect(remote_addr),
        Err(ErrorCode::InvalidArgument)
    ));
}

/// Connect should perform implicit bind.
fn test_udp_connect_without_bind(family: IpAddressFamily) {
    let remote_addr = IpSocketAddress::new(IpAddress::new_loopback(family), SOME_PORT);

    let sock = UdpSocket::create(family).unwrap();

    assert!(matches!(sock.get_local_address(), Err(_)));
    assert!(matches!(sock.connect(remote_addr), Ok(_)));
    assert!(matches!(sock.get_local_address(), Ok(_)));
}

/// Connect should work in combination with an explicit bind.
fn test_udp_connect_with_bind(family: IpAddressFamily) {
    let remote_addr = IpSocketAddress::new(IpAddress::new_loopback(family), SOME_PORT);

    let sock = UdpSocket::create(family).unwrap();

    sock.bind_unspecified().unwrap();

    assert!(matches!(sock.get_local_address(), Ok(_)));
    assert!(matches!(sock.connect(remote_addr), Ok(_)));
    assert!(matches!(sock.get_local_address(), Ok(_)));
}

fn test_udp_connect_dual_stack() {
    // Set-up:
    let v4_server = UdpSocket::create(IpAddressFamily::Ipv4).unwrap();
    v4_server
        .bind(IpSocketAddress::new(IpAddress::IPV4_LOOPBACK, 0))
        .unwrap();

    let v4_server_addr = v4_server.get_local_address().unwrap();
    let v6_server_addr =
        IpSocketAddress::new(IpAddress::IPV4_MAPPED_LOOPBACK, v4_server_addr.port());

    // Tests:
    let v6_client = UdpSocket::create(IpAddressFamily::Ipv6).unwrap();

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

/// A UDP socket should be immediately writable
async fn test_udp_connect_and_send(family: IpAddressFamily) {
    let unspecified_port = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let remote = IpSocketAddress::new(IpAddress::new_loopback(family), 4320);

    let client = UdpSocket::create(family).unwrap();
    client.bind(unspecified_port).unwrap();

    client.connect(remote).unwrap();
    assert_eq!(client.get_remote_address(), Ok(remote));

    assert_eq!(client.send(b"hello".into(), None).await, Ok(()));
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let supports_ipv6 = supports_ipv6();

        test_udp_connect_disconnect_reconnect(IpAddressFamily::Ipv4);
        test_udp_connect_unspec(IpAddressFamily::Ipv4);
        test_udp_connect_local_address_change(IpAddressFamily::Ipv4);
        test_udp_connect_port_0(IpAddressFamily::Ipv4);
        test_udp_connect_wrong_family(IpAddressFamily::Ipv4);
        test_udp_connect_without_bind(IpAddressFamily::Ipv4);
        test_udp_connect_with_bind(IpAddressFamily::Ipv4);
        test_udp_connect_and_send(IpAddressFamily::Ipv4).await;

        if supports_ipv6 {
            test_udp_connect_disconnect_reconnect(IpAddressFamily::Ipv6);
            test_udp_connect_unspec(IpAddressFamily::Ipv6);
            test_udp_connect_local_address_change(IpAddressFamily::Ipv6);
            test_udp_connect_port_0(IpAddressFamily::Ipv6);
            test_udp_connect_wrong_family(IpAddressFamily::Ipv6);
            test_udp_connect_without_bind(IpAddressFamily::Ipv6);
            test_udp_connect_with_bind(IpAddressFamily::Ipv6);
            test_udp_connect_and_send(IpAddressFamily::Ipv6).await;
            test_udp_connect_dual_stack();
        }

        Ok(())
    }
}

fn some_public_ip(family: IpAddressFamily) -> IpAddress {
    match family {
        IpAddressFamily::Ipv4 => IpAddress::Ipv4(SOME_PUBLIC_IPV4),
        IpAddressFamily::Ipv6 => IpAddress::Ipv6(SOME_PUBLIC_IPV6),
    }
}

fn has_public_interface(family: IpAddressFamily) -> bool {
    let sock = UdpSocket::create(family).unwrap();
    sock.connect(IpSocketAddress::new(some_public_ip(family), SOME_PORT))
        .is_ok()
}

fn main() {}
